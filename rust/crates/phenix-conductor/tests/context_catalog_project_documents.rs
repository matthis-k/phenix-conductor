use phenix_conductor::{CompiledConfiguration, ConductorRuntime, ContextRegistry, SkillRegistry};
use phenix_core::{
    BackendId, ContextResourceId, ContextResourceKind, ContextScope, ContextTier, ExecutionTarget,
    InferenceOptions, ModelId, ModelTarget, ObjectiveState, ObjectiveTransitionCause, PlanStep,
    PlanStepId, PlanStepRevisability, PlanStepState, PlanTransitionCause, ProviderId,
};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("phenix-context-catalog-project-docs-{nonce}"))
}

fn write(path: impl AsRef<Path>, content: &str) {
    let path = path.as_ref();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn fixed_target() -> ExecutionTarget {
    ExecutionTarget::Fixed(ModelTarget {
        backend: BackendId::parse("mock").unwrap(),
        provider: ProviderId::parse("mock").unwrap(),
        model: ModelId::parse("mock").unwrap(),
        inference: InferenceOptions::default(),
    })
}

fn configuration_for(root: &Path) -> CompiledConfiguration {
    let mut configuration = CompiledConfiguration::default();
    configuration.install_context_registry(ContextRegistry::discover(root).unwrap());
    configuration.install_skill_registry(SkillRegistry::discover(root).unwrap());
    configuration
}

#[test]
fn project_documents_are_discoverable_but_not_mandatory_prompt_content() {
    let root = fixture_root();
    fs::create_dir_all(root.join(".git")).unwrap();
    write(root.join("AGENTS.md"), "mandatory agent rules");
    write(
        root.join("CONTRIBUTING.md"),
        "discoverable contribution instructions",
    );
    write(
        root.join("DEVELOPMENT.md"),
        "discoverable development instructions",
    );

    let context = ContextRegistry::discover(&root).unwrap();
    let prompt = context
        .compose_prompt(&SkillRegistry::default(), "implement the requested change")
        .unwrap();

    assert!(prompt.contains("mandatory agent rules"));
    assert!(!prompt.contains("discoverable contribution instructions"));
    assert!(!prompt.contains("discoverable development instructions"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn project_documents_are_public_exact_catalog_resources() {
    let root = fixture_root();
    fs::create_dir_all(root.join(".git")).unwrap();
    write(
        root.join("CONTRIBUTING.md"),
        "discoverable contribution instructions",
    );
    write(
        root.join("DEVELOPMENT.md"),
        "discoverable development instructions",
    );

    let context = ContextRegistry::discover(&root).unwrap();
    let catalog = context.project_context_catalog().unwrap();
    let descriptors = catalog.descriptors().cloned().collect::<Vec<_>>();

    assert_eq!(descriptors.len(), 2);
    for path in ["CONTRIBUTING.md", "DEVELOPMENT.md"] {
        let id = ContextResourceId::parse(format!("project-document:{path}")).unwrap();
        let descriptor = catalog.current_descriptor(&id).unwrap();
        assert_eq!(descriptor.kind, ContextResourceKind::ProjectDocument);
        assert_eq!(
            descriptor.scope,
            ContextScope::Path {
                path: PathBuf::from(path)
            }
        );
        let revision = catalog.resolve_revision(&id, &descriptor.revision).unwrap();
        assert_eq!(revision.tier, ContextTier::DiscoverableContent);
        assert_eq!(revision.descriptor, *descriptor);
        assert!(revision.content.is_some());
    }

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn execution_context_catalog_stays_pinned_to_its_configuration_revision() {
    let root = fixture_root();
    fs::create_dir_all(root.join(".git")).unwrap();
    write(root.join("CONTRIBUTING.md"), "revision one");

    let mut runtime = ConductorRuntime::new();
    let first_revision = runtime
        .reload_configuration(configuration_for(&root))
        .unwrap();
    let session = runtime.create_session(None, None, fixed_target()).unwrap();
    let first_execution = runtime.submit(&session.id, "first").unwrap();

    write(root.join("CONTRIBUTING.md"), "revision two");
    let second_revision = runtime
        .reload_configuration(configuration_for(&root))
        .unwrap();
    runtime
        .rebase_session(&session.id, &second_revision)
        .unwrap();
    let second_execution = runtime.submit(&session.id, "second").unwrap();

    assert_eq!(
        runtime
            .execution_config_revision(&first_execution.id)
            .unwrap(),
        first_revision
    );
    assert_eq!(
        runtime
            .execution_config_revision(&second_execution.id)
            .unwrap(),
        second_revision
    );

    let id = ContextResourceId::parse("project-document:CONTRIBUTING.md").unwrap();
    let first = runtime
        .context_descriptors_for_execution(&first_execution.id)
        .unwrap()
        .into_iter()
        .find(|descriptor| descriptor.id == id)
        .unwrap();
    let second = runtime
        .context_descriptors_for_execution(&second_execution.id)
        .unwrap()
        .into_iter()
        .find(|descriptor| descriptor.id == id)
        .unwrap();

    assert_ne!(first.revision, second.revision);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn execution_context_catalog_combines_project_documents_and_skills() {
    let root = fixture_root();
    fs::create_dir_all(root.join(".git")).unwrap();
    write(root.join("CONTRIBUTING.md"), "project resource");
    write(
        root.join(".phenix/skills/review/SKILL.md"),
        "---\nname: review\ndescription: Review changes\n---\nskill resource\n",
    );

    let mut runtime = ConductorRuntime::new();
    runtime
        .reload_configuration(configuration_for(&root))
        .unwrap();
    let session = runtime.create_session(None, None, fixed_target()).unwrap();
    let execution = runtime.submit(&session.id, "inspect catalog").unwrap();

    let descriptors = runtime
        .context_descriptors_for_execution(&execution.id)
        .unwrap();
    let ids = descriptors
        .into_iter()
        .map(|descriptor| descriptor.id)
        .collect::<Vec<_>>();

    assert!(ids.contains(&ContextResourceId::parse("project-document:CONTRIBUTING.md").unwrap()));
    assert!(ids.contains(&ContextResourceId::parse("skill:review").unwrap()));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn execution_context_catalog_exposes_primary_objective_and_assigned_plan() {
    let mut runtime = ConductorRuntime::new();
    let session = runtime.create_session(None, None, fixed_target()).unwrap();
    let execution = runtime.submit(&session.id, "ship exact context").unwrap();
    let assignment = runtime
        .execution_objectives(&execution.id)
        .unwrap()
        .expect("execution has a primary objective");

    let step = PlanStep {
        id: PlanStepId::parse("catalog").unwrap(),
        description: "integrate durable plan context".to_owned(),
        state: PlanStepState::Proposed,
        revisability: PlanStepRevisability::Revisable,
        depends_on: BTreeSet::new(),
        objective_refs: BTreeSet::from([assignment.primary.clone()]),
    };
    let plan = runtime
        .create_plan(BTreeSet::from([assignment.primary.clone()]), vec![step])
        .unwrap();
    runtime
        .assign_execution_to_plan_step(
            &execution.id,
            &plan.id,
            &PlanStepId::parse("catalog").unwrap(),
        )
        .unwrap();

    let descriptors = runtime
        .context_descriptors_for_execution(&execution.id)
        .unwrap();
    let objective_id =
        ContextResourceId::parse(format!("objective:{}", assignment.primary)).unwrap();
    let plan_id = ContextResourceId::parse(format!("plan:{}", plan.id)).unwrap();

    assert!(descriptors.iter().any(|descriptor| {
        descriptor.id == objective_id && descriptor.kind == ContextResourceKind::Objective
    }));
    assert!(descriptors.iter().any(|descriptor| {
        descriptor.id == plan_id && descriptor.kind == ContextResourceKind::Plan
    }));
}

#[test]
fn durable_objective_and_plan_changes_update_exact_context_revisions() {
    let mut runtime = ConductorRuntime::new();
    let session = runtime.create_session(None, None, fixed_target()).unwrap();
    let execution = runtime
        .submit(&session.id, "track durable revisions")
        .unwrap();
    let assignment = runtime
        .execution_objectives(&execution.id)
        .unwrap()
        .expect("execution has a primary objective");
    let step_id = PlanStepId::parse("catalog").unwrap();
    let plan = runtime
        .create_plan(
            BTreeSet::from([assignment.primary.clone()]),
            vec![PlanStep {
                id: step_id.clone(),
                description: "track plan revision".to_owned(),
                state: PlanStepState::Proposed,
                revisability: PlanStepRevisability::Revisable,
                depends_on: BTreeSet::new(),
                objective_refs: BTreeSet::from([assignment.primary.clone()]),
            }],
        )
        .unwrap();
    runtime
        .assign_execution_to_plan_step(&execution.id, &plan.id, &step_id)
        .unwrap();

    let objective_id =
        ContextResourceId::parse(format!("objective:{}", assignment.primary)).unwrap();
    let plan_id = ContextResourceId::parse(format!("plan:{}", plan.id)).unwrap();
    let before = runtime
        .context_descriptors_for_execution(&execution.id)
        .unwrap();
    let before_objective = before
        .iter()
        .find(|descriptor| descriptor.id == objective_id)
        .unwrap()
        .revision
        .clone();
    let before_plan = before
        .iter()
        .find(|descriptor| descriptor.id == plan_id)
        .unwrap()
        .revision
        .clone();

    runtime
        .transition_objective(
            &assignment.primary,
            ObjectiveState::Failed,
            ObjectiveTransitionCause::UserIntent,
        )
        .unwrap();
    runtime
        .transition_plan_step(
            &plan.id,
            &step_id,
            PlanStepState::Failed,
            PlanTransitionCause::UserAction,
        )
        .unwrap();

    let after = runtime
        .context_descriptors_for_execution(&execution.id)
        .unwrap();
    let after_objective = after
        .iter()
        .find(|descriptor| descriptor.id == objective_id)
        .unwrap()
        .revision
        .clone();
    let after_plan = after
        .iter()
        .find(|descriptor| descriptor.id == plan_id)
        .unwrap()
        .revision
        .clone();

    assert!(before_objective.as_str().starts_with("sha256:"));
    assert!(before_plan.as_str().starts_with("sha256:"));
    assert_ne!(before_objective, after_objective);
    assert_ne!(before_plan, after_plan);
}
