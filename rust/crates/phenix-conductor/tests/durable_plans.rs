use phenix_conductor::{
    ConductorError, ConductorRuntime, DomainEvent, JournalEntry, JournalError, PlanError,
    SqliteStore,
};
use phenix_core::{
    BackendId, ExecutionId, ExecutionTarget, InferenceOptions, ModelId, ModelTarget, PlanId,
    PlanRecord, PlanState, PlanStep, PlanStepId, PlanStepRevisability, PlanStepState,
    PlanTransitionCause, ProviderId,
};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn target() -> ExecutionTarget {
    ExecutionTarget::Fixed(ModelTarget {
        backend: BackendId::parse("backend").unwrap(),
        provider: ProviderId::parse("provider").unwrap(),
        model: ModelId::parse("model").unwrap(),
        inference: InferenceOptions::default(),
    })
}

fn step(id: &str, depends_on: &[&str]) -> PlanStep {
    PlanStep {
        id: PlanStepId::parse(id).unwrap(),
        description: format!("do {id}"),
        state: PlanStepState::Proposed,
        revisability: PlanStepRevisability::Revisable,
        depends_on: depends_on
            .iter()
            .map(|id| PlanStepId::parse(*id).unwrap())
            .collect(),
        objective_refs: BTreeSet::new(),
    }
}

fn temporary_database() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "phenix-durable-plans-{}-{nonce}.db",
        std::process::id()
    ))
}

#[test]
fn draft_updates_are_optimistic_and_first_assignment_freezes_revision() {
    let mut runtime = ConductorRuntime::new();
    let session = runtime.create_session(None, None, target()).unwrap();
    let first_execution = runtime
        .submit(&session.id, "Implement plan semantics")
        .unwrap();
    let plan = runtime
        .create_plan(
            BTreeSet::new(),
            vec![step("inspect", &[]), step("implement", &["inspect"])],
        )
        .unwrap();
    let revised = runtime
        .revise_plan_draft(
            &plan.id,
            1,
            BTreeSet::new(),
            vec![step("inspect", &[]), step("implement", &["inspect"])],
        )
        .unwrap();
    assert_eq!(revised.revision, 2);
    assert!(matches!(
        runtime.revise_plan_draft(&plan.id, 1, BTreeSet::new(), vec![step("inspect", &[])],),
        Err(ConductorError::Plan(
            PlanError::DraftRevisionConflict { .. }
        ))
    ));

    runtime
        .assign_execution_to_plan_step(
            &first_execution.id,
            &plan.id,
            &PlanStepId::parse("inspect").unwrap(),
        )
        .unwrap();
    let enacted = runtime.plan(&plan.id).unwrap();
    assert_eq!(enacted.state, PlanState::Active);
    assert_eq!(enacted.revision, 2);
    assert_eq!(enacted.steps[0].state, PlanStepState::Active);
    assert_eq!(enacted.steps[1].state, PlanStepState::Committed);
    assert!(matches!(
        runtime.revise_plan_draft(
            &plan.id,
            2,
            BTreeSet::new(),
            vec![step("other", &[])],
        ),
        Err(ConductorError::Plan(PlanError::EnactedPlanIsImmutable(id))) if id == plan.id
    ));
}

#[test]
fn dependencies_gate_step_enactment_and_successors_preserve_history() {
    let mut runtime = ConductorRuntime::new();
    let session = runtime.create_session(None, None, target()).unwrap();
    let first = runtime.submit(&session.id, "First execution").unwrap();
    let second = runtime.submit(&session.id, "Second execution").unwrap();
    let plan = runtime
        .create_plan(
            BTreeSet::new(),
            vec![step("first", &[]), step("second", &["first"])],
        )
        .unwrap();

    assert!(matches!(
        runtime.assign_execution_to_plan_step(
            &second.id,
            &plan.id,
            &PlanStepId::parse("second").unwrap(),
        ),
        Err(ConductorError::Plan(
            PlanError::IncompleteDependencies { .. }
        ))
    ));
    runtime
        .assign_execution_to_plan_step(&first.id, &plan.id, &PlanStepId::parse("first").unwrap())
        .unwrap();
    runtime
        .transition_plan_step(
            &plan.id,
            &PlanStepId::parse("first").unwrap(),
            PlanStepState::Completed,
            PlanTransitionCause::ExecutionOutcome {
                execution_id: first.id.clone(),
            },
        )
        .unwrap();
    runtime
        .assign_execution_to_plan_step(&second.id, &plan.id, &PlanStepId::parse("second").unwrap())
        .unwrap();
    runtime
        .transition_plan(
            &plan.id,
            PlanState::Invalidated,
            PlanTransitionCause::EvidenceAssessment {
                evidence_ref: "file-observation:1".to_owned(),
            },
        )
        .unwrap();
    let successor = runtime
        .create_successor_plan(
            &plan.id,
            BTreeSet::new(),
            vec![step("replacement", &[])],
            PlanTransitionCause::UserAction,
        )
        .unwrap();
    assert_eq!(successor.supersedes, Some(plan.id.clone()));
    assert_eq!(runtime.plan(&plan.id).unwrap().state, PlanState::Superseded);
    assert_eq!(
        runtime.plan(&plan.id).unwrap().steps[0].description,
        "do first"
    );
}

#[test]
fn invalid_successor_cause_is_rejected_without_leaving_partial_history() {
    let mut runtime = ConductorRuntime::new();
    let session = runtime.create_session(None, None, target()).unwrap();
    let execution = runtime.submit(&session.id, "Enact plan").unwrap();
    let plan = runtime
        .create_plan(BTreeSet::new(), vec![step("work", &[])])
        .unwrap();
    runtime
        .assign_execution_to_plan_step(&execution.id, &plan.id, &PlanStepId::parse("work").unwrap())
        .unwrap();

    let before = runtime.plans().unwrap();
    assert!(matches!(
        runtime.create_successor_plan(
            &plan.id,
            BTreeSet::new(),
            vec![step("replacement", &[])],
            PlanTransitionCause::Policy {
                description: " ".to_owned(),
            },
        ),
        Err(ConductorError::Plan(PlanError::InvalidCause))
    ));
    assert_eq!(runtime.plans().unwrap(), before);
    assert_eq!(runtime.plan(&plan.id).unwrap().state, PlanState::Active);
}

#[test]
fn live_transitions_reject_missing_successors_unknown_causes_and_terminal_step_edits() {
    let mut runtime = ConductorRuntime::new();
    let session = runtime.create_session(None, None, target()).unwrap();
    let execution = runtime.submit(&session.id, "Enact plan").unwrap();
    let plan = runtime
        .create_plan(BTreeSet::new(), vec![step("work", &[])])
        .unwrap();
    runtime
        .assign_execution_to_plan_step(&execution.id, &plan.id, &PlanStepId::parse("work").unwrap())
        .unwrap();

    assert!(matches!(
        runtime.transition_plan(&plan.id, PlanState::Superseded, PlanTransitionCause::UserAction),
        Err(ConductorError::Plan(PlanError::InvalidSuccessor(id))) if id == plan.id
    ));
    let unknown = ExecutionId::parse("execution-missing").unwrap();
    assert!(matches!(
        runtime.transition_plan(
            &plan.id,
            PlanState::Failed,
            PlanTransitionCause::AgentAction {
                execution_id: unknown.clone(),
            },
        ),
        Err(ConductorError::Plan(PlanError::UnknownExecution(id))) if id == unknown
    ));

    runtime
        .transition_plan(
            &plan.id,
            PlanState::Failed,
            PlanTransitionCause::ExecutionOutcome {
                execution_id: execution.id.clone(),
            },
        )
        .unwrap();
    assert!(matches!(
        runtime.transition_plan_step(
            &plan.id,
            &PlanStepId::parse("work").unwrap(),
            PlanStepState::Completed,
            PlanTransitionCause::ExecutionOutcome {
                execution_id: execution.id,
            },
        ),
        Err(ConductorError::Plan(
            PlanError::InvalidStepTransition { .. }
        ))
    ));
}

#[test]
fn replay_rejects_successor_of_completed_plan() {
    let mut runtime = ConductorRuntime::new();
    let session = runtime.create_session(None, None, target()).unwrap();
    let execution = runtime.submit(&session.id, "Complete plan").unwrap();
    let plan = runtime
        .create_plan(BTreeSet::new(), vec![step("done", &[])])
        .unwrap();
    runtime
        .assign_execution_to_plan_step(&execution.id, &plan.id, &PlanStepId::parse("done").unwrap())
        .unwrap();
    runtime
        .transition_plan_step(
            &plan.id,
            &PlanStepId::parse("done").unwrap(),
            PlanStepState::Completed,
            PlanTransitionCause::ExecutionOutcome {
                execution_id: execution.id,
            },
        )
        .unwrap();
    runtime
        .transition_plan(
            &plan.id,
            PlanState::Completed,
            PlanTransitionCause::UserAction,
        )
        .unwrap();

    let mut journal = runtime.journal().clone();
    journal.entries.push(JournalEntry {
        sequence: journal.entries.len() as u64 + 1,
        event: DomainEvent::PlanCreated {
            plan: PlanRecord {
                id: PlanId::parse("plan-2").unwrap(),
                workspace: plan.workspace,
                state: PlanState::Draft,
                revision: 1,
                objective_refs: BTreeSet::new(),
                supersedes: Some(plan.id),
                steps: vec![step("replacement", &[])],
            },
        },
    });
    assert!(matches!(
        journal.validate_structure(),
        Err(JournalError::InvalidEvent(message)) if message.contains("invalid predecessor")
    ));
}

#[test]
fn sqlite_roundtrip_preserves_plan_history() {
    let mut runtime = ConductorRuntime::new();
    let session = runtime.create_session(None, None, target()).unwrap();
    let execution = runtime.submit(&session.id, "Persist plan history").unwrap();
    let plan = runtime
        .create_plan(BTreeSet::new(), vec![step("persist", &[])])
        .unwrap();
    let assignment = runtime
        .assign_execution_to_plan_step(
            &execution.id,
            &plan.id,
            &PlanStepId::parse("persist").unwrap(),
        )
        .unwrap();

    let path = temporary_database();
    let store = SqliteStore::new(&path);
    store.save(runtime.journal()).unwrap();
    let restored = ConductorRuntime::restore(store.load().unwrap()).unwrap();
    assert_eq!(
        restored.execution_plan(&execution.id).unwrap(),
        Some(assignment)
    );
    assert_eq!(restored.plan(&plan.id).unwrap().state, PlanState::Active);
    fs::remove_file(path).unwrap();
}

#[test]
fn dependency_cycles_are_rejected_before_persistence() {
    let mut runtime = ConductorRuntime::new();
    assert!(matches!(
        runtime.create_plan(BTreeSet::new(), vec![step("a", &["b"]), step("b", &["a"])],),
        Err(ConductorError::Plan(PlanError::DependencyCycle))
    ));
}
