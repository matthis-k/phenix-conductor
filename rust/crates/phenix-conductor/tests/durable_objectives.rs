use phenix_conductor::{
    ConductorError, ConductorRuntime, DomainEvent, JournalEntry, JournalError, ObjectiveError,
    SqliteStore,
};
use phenix_core::{
    BackendId, ExecutionId, ExecutionTarget, InferenceOptions, ModelId, ModelTarget,
    ObjectiveCriterion, ObjectiveCriterionEvidence, ObjectiveCriterionId, ObjectiveId,
    ObjectiveOrigin, ObjectiveRecord, ObjectiveState, ObjectiveTransition,
    ObjectiveTransitionCause, ProviderId,
};
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

fn criterion(id: &str, required: bool) -> ObjectiveCriterion {
    ObjectiveCriterion {
        id: ObjectiveCriterionId::parse(id).unwrap(),
        description: format!("criterion {id}"),
        required,
    }
}

fn temporary_database() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "phenix-durable-objectives-{}-{nonce}.db",
        std::process::id()
    ))
}

#[test]
fn root_submission_creates_and_replays_primary_objective() {
    let mut runtime = ConductorRuntime::new();
    let session = runtime.create_session(None, None, target()).unwrap();
    let execution = runtime
        .submit(&session.id, "Make durable objectives canonical")
        .unwrap();

    let assignment = runtime
        .execution_objectives(&execution.id)
        .unwrap()
        .expect("new execution has a primary objective");
    let objective = runtime.objective(&assignment.primary).unwrap();
    assert_eq!(objective.origin, ObjectiveOrigin::Root);
    assert_eq!(objective.state, ObjectiveState::Active);
    assert_eq!(objective.statement, "Make durable objectives canonical");

    let restored = ConductorRuntime::restore(runtime.journal().clone()).unwrap();
    assert_eq!(
        restored.execution_objectives(&execution.id).unwrap(),
        Some(assignment)
    );
}

#[test]
fn later_user_intent_supersedes_root_without_rewriting_history() {
    let mut runtime = ConductorRuntime::new();
    let first = runtime
        .create_root_objective_from_user_intent("Original user intent", Vec::new(), None)
        .unwrap();
    let successor = runtime
        .create_root_objective_from_user_intent(
            "Revised user intent",
            Vec::new(),
            Some(first.id.clone()),
        )
        .unwrap();

    assert_eq!(successor.origin, ObjectiveOrigin::Root);
    assert_eq!(successor.supersedes, Some(first.id.clone()));
    assert_eq!(
        runtime.objective(&first.id).unwrap().state,
        ObjectiveState::Superseded
    );
    assert_eq!(
        runtime.objective(&first.id).unwrap().statement,
        "Original user intent"
    );

    let restored = ConductorRuntime::restore(runtime.journal().clone()).unwrap();
    assert_eq!(
        restored.objective(&first.id).unwrap().state,
        ObjectiveState::Superseded
    );
    assert_eq!(restored.objective(&successor.id).unwrap(), successor);
}

#[test]
fn replay_rejects_supersession_that_is_not_root_to_root() {
    let mut runtime = ConductorRuntime::new();
    let root = runtime
        .create_root_objective_from_user_intent("Root intent", Vec::new(), None)
        .unwrap();
    let derived = runtime
        .create_derived_objective(&root.id, "Derived work", Vec::new())
        .unwrap();
    let base = runtime.journal().clone();

    let mut root_over_derived = base.clone();
    root_over_derived.entries.push(JournalEntry {
        sequence: root_over_derived.entries.len() as u64 + 1,
        event: DomainEvent::ObjectiveCreated {
            objective: ObjectiveRecord {
                id: ObjectiveId::parse("objective-3").unwrap(),
                workspace: derived.workspace.clone(),
                origin: ObjectiveOrigin::Root,
                statement: "Tampered root".to_owned(),
                criteria: Vec::new(),
                state: ObjectiveState::Active,
                supersedes: Some(derived.id.clone()),
            },
        },
    });
    assert!(matches!(
        root_over_derived.validate_structure(),
        Err(JournalError::InvalidEvent(message))
            if message.contains("cannot supersede derived objective")
    ));

    let mut derived_over_root = base;
    derived_over_root.entries.push(JournalEntry {
        sequence: derived_over_root.entries.len() as u64 + 1,
        event: DomainEvent::ObjectiveCreated {
            objective: ObjectiveRecord {
                id: ObjectiveId::parse("objective-3").unwrap(),
                workspace: derived.workspace.clone(),
                origin: ObjectiveOrigin::Derived {
                    parent: root.id.clone(),
                },
                statement: "Tampered derived".to_owned(),
                criteria: Vec::new(),
                state: ObjectiveState::Draft,
                supersedes: Some(root.id),
            },
        },
    });
    assert!(matches!(
        derived_over_root.validate_structure(),
        Err(JournalError::InvalidEvent(message))
            if message.contains("cannot supersede another objective")
    ));
}

#[test]
fn enacted_objective_meaning_freezes_and_completion_requires_evidence() {
    let mut runtime = ConductorRuntime::new();
    let root = runtime
        .create_root_objective_from_user_intent("Ship objective semantics", Vec::new(), None)
        .unwrap();
    let derived = runtime
        .create_derived_objective(
            &root.id,
            "Persist objective history",
            vec![criterion("persisted", true), criterion("documented", false)],
        )
        .unwrap();
    let revised = runtime
        .revise_objective_draft(
            &derived.id,
            "Persist and replay objective history",
            vec![criterion("persisted", true), criterion("documented", false)],
        )
        .unwrap();
    assert_eq!(revised.state, ObjectiveState::Draft);

    runtime
        .activate_objective(&derived.id, ObjectiveTransitionCause::UserIntent)
        .unwrap();
    assert!(matches!(
        runtime.revise_objective_draft(&derived.id, "rewrite", Vec::new()),
        Err(ConductorError::Objective(ObjectiveError::EnactedObjectiveIsImmutable(id)))
            if id == derived.id
    ));
    assert!(matches!(
        runtime.complete_objective(&derived.id, ObjectiveTransitionCause::UserIntent),
        Err(ConductorError::Objective(
            ObjectiveError::MissingRequiredEvidence { .. }
        ))
    ));

    runtime
        .record_objective_evidence(
            &derived.id,
            ObjectiveCriterionEvidence {
                criterion_id: ObjectiveCriterionId::parse("persisted").unwrap(),
                evidence_ref: "execution:1".to_owned(),
            },
        )
        .unwrap();
    let completed = runtime
        .complete_objective(
            &derived.id,
            ObjectiveTransitionCause::EvidenceAssessment {
                evidence_ref: "execution:1".to_owned(),
            },
        )
        .unwrap();
    assert_eq!(completed.state, ObjectiveState::Completed);
}

#[test]
fn transition_causes_require_durable_provenance_at_runtime_and_replay() {
    let mut runtime = ConductorRuntime::new();
    let root = runtime
        .create_root_objective_from_user_intent("Root intent", Vec::new(), None)
        .unwrap();
    let derived = runtime
        .create_derived_objective(&root.id, "Derived work", Vec::new())
        .unwrap();
    let missing = ExecutionId::parse("execution-missing").unwrap();

    assert!(matches!(
        runtime.activate_objective(
            &derived.id,
            ObjectiveTransitionCause::AgentAction {
                execution_id: missing.clone(),
            },
        ),
        Err(ConductorError::Objective(ObjectiveError::UnknownExecution(id))) if id == missing
    ));

    let mut tampered = runtime.journal().clone();
    tampered.entries.push(JournalEntry {
        sequence: tampered.entries.len() as u64 + 1,
        event: DomainEvent::ObjectiveStateChanged {
            transition: ObjectiveTransition {
                objective_id: derived.id,
                from: ObjectiveState::Draft,
                to: ObjectiveState::Active,
                cause: ObjectiveTransitionCause::ExecutionOutcome {
                    execution_id: missing,
                },
            },
        },
    });
    assert!(matches!(
        tampered.validate_structure(),
        Err(JournalError::InvalidEvent(message))
            if message.contains("transition references unknown execution")
    ));
}

#[test]
fn sqlite_roundtrip_preserves_objective_facts() {
    let mut runtime = ConductorRuntime::new();
    let session = runtime.create_session(None, None, target()).unwrap();
    let execution = runtime
        .submit(&session.id, "Persist this objective")
        .unwrap();
    let assignment = runtime
        .execution_objectives(&execution.id)
        .unwrap()
        .unwrap();

    let path = temporary_database();
    let store = SqliteStore::new(&path);
    store.save(runtime.journal()).unwrap();
    let loaded = store.load().unwrap();
    let restored = ConductorRuntime::restore(loaded).unwrap();
    assert_eq!(
        restored.execution_objectives(&execution.id).unwrap(),
        Some(assignment)
    );
    fs::remove_file(path).unwrap();
}
