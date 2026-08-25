use crate::{ConductorError, ConductorRuntime, DecisionError, ResolvedExactReference, SqliteStore};
use phenix_core::{
    BackendId, ContextInjectionLifetime, ContextInjectionRequester, ContextResourceKind,
    DecisionApplicability, DecisionCreator, DecisionDraftInput, DecisionHistoryQuery,
    DecisionHistoryScope, DecisionRecord, DecisionRelation, ExactReference, ExecutionTarget,
    InferenceOptions, ModelId, ModelTarget, ProviderId,
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

fn database(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "phenix-decisions-{label}-{}-{nonce}.db",
        std::process::id()
    ))
}

fn input(question: &str, objective: phenix_core::ObjectiveId) -> DecisionDraftInput {
    DecisionDraftInput {
        question: question.to_owned(),
        chosen_option: "use durable typed state".to_owned(),
        rationale: "durable state preserves exact history".to_owned(),
        alternatives: vec!["keep only transient prose".to_owned()],
        alternatives_not_considered_reason: None,
        evidence: vec![ExactReference::Objective(objective.clone())],
        creator: DecisionCreator::User,
        objectives: BTreeSet::from([objective]),
        dependencies: BTreeSet::new(),
        relation: None,
    }
}

#[test]
fn recorded_decision_rejects_mutation_and_revert_preserves_history() {
    let mut runtime = ConductorRuntime::new();
    let session = runtime.create_session(None, None, target()).unwrap();
    let execution = runtime.submit(&session.id, "record decision").unwrap();
    let objective = runtime
        .execution_objectives(&execution.id)
        .unwrap()
        .unwrap()
        .primary;

    let first = runtime
        .create_decision_draft(input("Which durable representation?", objective.clone()))
        .unwrap();
    let first = runtime.record_decision(&first.id).unwrap();
    assert!(matches!(
        runtime.revise_decision_draft(
            &first.id,
            first.revision,
            input("mutate recorded decision", objective.clone()),
        ),
        Err(ConductorError::Decision(
            DecisionError::RecordedDecisionIsImmutable(_)
        ))
    ));

    let mut revert_input = input("Should the first decision be reverted?", objective);
    revert_input.relation = Some(DecisionRelation::Reverts {
        decision_id: first.id.clone(),
    });
    let revert = runtime.create_decision_draft(revert_input).unwrap();
    let revert = runtime.record_decision(&revert.id).unwrap();

    assert_eq!(runtime.decision(&first.id).unwrap(), first);
    assert_eq!(
        revert.relation,
        Some(DecisionRelation::Reverts {
            decision_id: first.id
        })
    );
}

#[test]
fn sqlite_roundtrip_preserves_evidence_dependencies_and_applicability() {
    let mut runtime = ConductorRuntime::new();
    let session = runtime.create_session(None, None, target()).unwrap();
    let execution = runtime.submit(&session.id, "persist decisions").unwrap();
    let objective = runtime
        .execution_objectives(&execution.id)
        .unwrap()
        .unwrap()
        .primary;

    let first = runtime
        .create_decision_draft(input("First durable decision", objective.clone()))
        .unwrap();
    let first = runtime.record_decision(&first.id).unwrap();

    let mut second_input = input("Second durable decision", objective);
    second_input.dependencies.insert(first.id.clone());
    second_input
        .evidence
        .push(ExactReference::Decision(first.id.clone()));
    let second = runtime.create_decision_draft(second_input).unwrap();
    runtime.record_decision(&second.id).unwrap();
    let second = runtime
        .assess_decision_applicability(&second.id, DecisionApplicability::Questionable)
        .unwrap();

    let path = database("roundtrip");
    let store = SqliteStore::new(&path);
    store.save(runtime.journal()).unwrap();
    let restored = ConductorRuntime::restore(store.load().unwrap()).unwrap();
    assert_eq!(restored.decision(&first.id).unwrap(), first);
    assert_eq!(restored.decision(&second.id).unwrap(), second);
    assert!(second.dependencies.contains(&first.id));
    assert!(second
        .evidence
        .contains(&ExactReference::Decision(first.id.clone())));
    fs::remove_file(path).unwrap();
}

#[test]
fn default_history_scope_is_objective_lineage_and_workspace_scope_is_explicit() {
    let mut runtime = ConductorRuntime::new();
    let first_session = runtime.create_session(None, None, target()).unwrap();
    let first_execution = runtime.submit(&first_session.id, "first lineage").unwrap();
    let first_objective = runtime
        .execution_objectives(&first_execution.id)
        .unwrap()
        .unwrap()
        .primary;
    let first = runtime
        .create_decision_draft(input("Durable choice in first lineage", first_objective))
        .unwrap();
    let first = runtime.record_decision(&first.id).unwrap();

    let second_session = runtime.create_session(None, None, target()).unwrap();
    let second_execution = runtime
        .submit(&second_session.id, "second lineage")
        .unwrap();
    let second_objective = runtime
        .execution_objectives(&second_execution.id)
        .unwrap()
        .unwrap()
        .primary;
    let second = runtime
        .create_decision_draft(input("Durable choice in second lineage", second_objective))
        .unwrap();
    let second = runtime.record_decision(&second.id).unwrap();

    let path = database("search");
    let store = SqliteStore::new(&path);
    store.save(runtime.journal()).unwrap();

    let default_query = runtime
        .decision_history_query_for_execution(&first_execution.id, "durable", 20)
        .unwrap();
    assert!(matches!(
        default_query.scope,
        DecisionHistoryScope::ObjectiveLineage(_)
    ));
    let lineage = store.search_decision_history(&default_query).unwrap();
    assert_eq!(lineage.len(), 1);
    assert_eq!(lineage[0].decision_id, first.id);

    let workspace = store
        .search_decision_history(&DecisionHistoryQuery {
            text: "durable".to_owned(),
            scope: DecisionHistoryScope::Workspace,
            limit: 20,
        })
        .unwrap();
    assert_eq!(workspace.len(), 2);
    assert!(workspace.iter().any(|item| item.decision_id == first.id));
    assert!(workspace.iter().any(|item| item.decision_id == second.id));

    let before = store.load().unwrap();
    store.rebuild_decision_search_index().unwrap();
    assert_eq!(store.load().unwrap(), before);
    let rebuilt = store.search_decision_history(&default_query).unwrap();
    assert_eq!(rebuilt, lineage);
    fs::remove_file(path).unwrap();
}

#[test]
fn decision_exact_resolution_and_context_loading_share_canonical_paths() {
    let mut runtime = ConductorRuntime::new();
    let session = runtime.create_session(None, None, target()).unwrap();
    let execution = runtime
        .submit(&session.id, "load decision context")
        .unwrap();
    let objective = runtime
        .execution_objectives(&execution.id)
        .unwrap()
        .unwrap()
        .primary;
    let decision = runtime
        .create_decision_draft(input("Durable context decision", objective))
        .unwrap();
    let decision = runtime.record_decision(&decision.id).unwrap();

    let reference = ExactReference::Decision(decision.id.clone());
    assert!(matches!(
        runtime.resolve_exact_reference(&reference).unwrap(),
        ResolvedExactReference::Decision(ref resolved) if resolved == &decision
    ));

    let descriptor = runtime
        .context_descriptors_for_execution(&execution.id)
        .unwrap()
        .into_iter()
        .find(|descriptor| descriptor.kind == ContextResourceKind::Decision)
        .expect("recorded lineage decision must be discoverable");
    let (resource, injection) = runtime
        .load_context_for_execution(
            &execution.id,
            &descriptor.id,
            &descriptor.revision,
            ContextInjectionRequester::Agent,
            ContextInjectionLifetime::Execution,
            "inspect durable decision",
        )
        .unwrap();
    assert_eq!(resource.source_ref, reference);
    assert_eq!(
        serde_json::from_str::<DecisionRecord>(resource.content.as_deref().unwrap()).unwrap(),
        decision
    );
    assert_eq!(injection.source_ref, reference);

    let path = database("context");
    let store = SqliteStore::new(&path);
    store.save(runtime.journal()).unwrap();
    let restored = ConductorRuntime::restore(store.load().unwrap()).unwrap();
    assert_eq!(restored.decision(&decision.id).unwrap(), decision);
    fs::remove_file(path).unwrap();
}

#[test]
fn recording_requires_stable_decision_references() {
    let mut runtime = ConductorRuntime::new();
    let session = runtime.create_session(None, None, target()).unwrap();
    let execution = runtime
        .submit(&session.id, "record exact decision references")
        .unwrap();
    let objective = runtime
        .execution_objectives(&execution.id)
        .unwrap()
        .unwrap()
        .primary;

    let prerequisite = runtime
        .create_decision_draft(input("Mutable prerequisite", objective.clone()))
        .unwrap();
    let mut dependent_input = input("Stable dependent", objective);
    dependent_input.dependencies.insert(prerequisite.id.clone());
    dependent_input.relation = Some(DecisionRelation::Supersedes {
        decision_id: prerequisite.id.clone(),
    });
    dependent_input
        .evidence
        .push(ExactReference::Decision(prerequisite.id.clone()));
    let dependent = runtime.create_decision_draft(dependent_input).unwrap();

    assert!(matches!(
        runtime.record_decision(&dependent.id),
        Err(ConductorError::Decision(
            DecisionError::DecisionReferenceNotRecorded(id)
        )) if id == prerequisite.id
    ));

    runtime.record_decision(&prerequisite.id).unwrap();
    let recorded = runtime.record_decision(&dependent.id).unwrap();
    assert_eq!(recorded.state, phenix_core::DecisionState::Recorded);
}

#[test]
fn decision_records_why_no_alternatives_were_considered() {
    let mut runtime = ConductorRuntime::new();
    let session = runtime.create_session(None, None, target()).unwrap();
    let execution = runtime
        .submit(&session.id, "record no-alternative provenance")
        .unwrap();
    let objective = runtime
        .execution_objectives(&execution.id)
        .unwrap()
        .unwrap()
        .primary;

    let mut input = input("Was another option viable?", objective);
    input.alternatives.clear();
    assert!(matches!(
        runtime.create_decision_draft(input.clone()),
        Err(ConductorError::Decision(DecisionError::InvalidText(
            "why no alternatives were considered"
        )))
    ));

    input.alternatives_not_considered_reason =
        Some("The prerequisite invariant permits only one semantically valid option.".to_owned());
    let decision = runtime.create_decision_draft(input).unwrap();
    let decision = runtime.record_decision(&decision.id).unwrap();
    assert!(decision.alternatives.is_empty());
    assert!(decision
        .alternatives_not_considered_reason
        .as_deref()
        .is_some_and(|reason| reason.contains("only one")));

    let path = database("no-alternatives");
    let store = SqliteStore::new(&path);
    store.save(runtime.journal()).unwrap();
    let restored = ConductorRuntime::restore(store.load().unwrap()).unwrap();
    assert_eq!(restored.decision(&decision.id).unwrap(), decision);
    fs::remove_file(path).unwrap();
}
