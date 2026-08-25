use phenix_conductor::{
    CompiledConfiguration, ConductorRuntime, ContextRegistry, ResolvedExactReference, SkillRegistry,
};
use phenix_core::{
    BackendId, ContextInjectionLifetime, ContextInjectionRequester, ContextResourceId,
    ExactReference, ExecutionTarget, FilesystemAuthority, InferenceOptions, ModelId, ModelTarget,
    ProviderId, RepositoryAuthority,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("phenix-context-catalog-injection-{nonce}"))
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

fn assert_public_exact_reference_type(_: &ResolvedExactReference) {}

#[test]
fn execution_loads_exact_project_context_and_records_the_injection() {
    let root = fixture_root();
    fs::create_dir_all(root.join(".git")).unwrap();
    write(root.join("CONTRIBUTING.md"), "exact project context");

    let mut runtime = ConductorRuntime::new();
    runtime
        .reload_configuration(configuration_for(&root))
        .unwrap();
    let session = runtime.create_session(None, None, fixed_target()).unwrap();
    let execution = runtime.submit(&session.id, "load project context").unwrap();

    let id = ContextResourceId::parse("project-document:CONTRIBUTING.md").unwrap();
    let descriptor = runtime
        .context_descriptors_for_execution(&execution.id)
        .unwrap()
        .into_iter()
        .find(|descriptor| descriptor.id == id)
        .unwrap();

    let (resource, injection) = runtime
        .load_context_for_execution(
            &execution.id,
            &id,
            &descriptor.revision,
            ContextInjectionRequester::Agent,
            ContextInjectionLifetime::SingleRequest,
            "agent requested exact project context",
        )
        .unwrap();

    assert_eq!(resource.descriptor, descriptor);
    assert_eq!(resource.content.as_deref(), Some("exact project context"));
    assert_eq!(injection.execution_id, execution.id);
    assert_eq!(injection.source_ref, ExactReference::Context(id));
    assert_eq!(injection.source_revision, resource.descriptor.revision);
    assert_eq!(injection.content_identity, resource.content_identity);
    assert_eq!(injection.requested_by, ContextInjectionRequester::Agent);
    assert_eq!(injection.lifetime, ContextInjectionLifetime::SingleRequest);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn path_context_load_preserves_execution_authority() {
    let root = fixture_root();
    fs::create_dir_all(root.join(".git")).unwrap();
    write(root.join("CONTRIBUTING.md"), "read-scoped project context");

    let mut runtime = ConductorRuntime::new();
    runtime
        .reload_configuration(configuration_for(&root))
        .unwrap();
    let session = runtime.create_session(None, None, fixed_target()).unwrap();
    let execution = runtime
        .submit(&session.id, "load context without authority escalation")
        .unwrap();
    let authority_before = runtime.execution_authority(&execution.id).unwrap();
    assert_eq!(authority_before.filesystem, FilesystemAuthority::ReadOnly);
    assert_eq!(authority_before.repository, RepositoryAuthority::Read);

    let id = ContextResourceId::parse("project-document:CONTRIBUTING.md").unwrap();
    let descriptor = runtime
        .context_descriptors_for_execution(&execution.id)
        .unwrap()
        .into_iter()
        .find(|descriptor| descriptor.id == id)
        .unwrap();

    runtime
        .load_context_for_execution(
            &execution.id,
            &id,
            &descriptor.revision,
            ContextInjectionRequester::Agent,
            ContextInjectionLifetime::SingleRequest,
            "load configured path context under existing read authority",
        )
        .unwrap();

    assert_eq!(
        runtime.execution_authority(&execution.id).unwrap(),
        authority_before,
        "loading configured context must not expand filesystem, repository, or callable authority"
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn objective_lifetime_uses_the_execution_primary_objective() {
    let root = fixture_root();
    fs::create_dir_all(root.join(".git")).unwrap();
    write(root.join("CONTRIBUTING.md"), "objective-scoped context");

    let mut runtime = ConductorRuntime::new();
    runtime
        .reload_configuration(configuration_for(&root))
        .unwrap();
    let session = runtime.create_session(None, None, fixed_target()).unwrap();
    let execution = runtime
        .submit(&session.id, "load objective-scoped context")
        .unwrap();
    let objective_assignment = runtime
        .execution_objectives(&execution.id)
        .unwrap()
        .expect("root execution must have a primary objective");

    let id = ContextResourceId::parse("project-document:CONTRIBUTING.md").unwrap();
    let descriptor = runtime
        .context_descriptors_for_execution(&execution.id)
        .unwrap()
        .into_iter()
        .find(|descriptor| descriptor.id == id)
        .unwrap();

    let (resource, injection) = runtime
        .load_context_for_execution(
            &execution.id,
            &id,
            &descriptor.revision,
            ContextInjectionRequester::Agent,
            ContextInjectionLifetime::Objective,
            "agent requested objective-lifetime context",
        )
        .unwrap();

    assert_eq!(resource.descriptor, descriptor);
    assert_eq!(injection.execution_id, execution.id);
    assert_eq!(injection.lifetime, ContextInjectionLifetime::Objective);
    assert_eq!(
        runtime.execution_objectives(&execution.id).unwrap(),
        Some(objective_assignment)
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn stale_revision_never_substitutes_current_content() {
    let root = fixture_root();
    fs::create_dir_all(root.join(".git")).unwrap();
    write(root.join("CONTRIBUTING.md"), "first project context");

    let mut runtime = ConductorRuntime::new();
    runtime
        .reload_configuration(configuration_for(&root))
        .unwrap();
    let first_session = runtime.create_session(None, None, fixed_target()).unwrap();
    let first_execution = runtime
        .submit(&first_session.id, "observe first context revision")
        .unwrap();

    let id = ContextResourceId::parse("project-document:CONTRIBUTING.md").unwrap();
    let first_descriptor = runtime
        .context_descriptors_for_execution(&first_execution.id)
        .unwrap()
        .into_iter()
        .find(|descriptor| descriptor.id == id)
        .unwrap();

    write(root.join("CONTRIBUTING.md"), "second project context");
    runtime
        .reload_configuration(configuration_for(&root))
        .unwrap();
    let second_session = runtime.create_session(None, None, fixed_target()).unwrap();
    let second_execution = runtime
        .submit(&second_session.id, "observe second context revision")
        .unwrap();
    let second_descriptor = runtime
        .context_descriptors_for_execution(&second_execution.id)
        .unwrap()
        .into_iter()
        .find(|descriptor| descriptor.id == id)
        .unwrap();

    assert_ne!(first_descriptor.revision, second_descriptor.revision);
    assert!(runtime
        .load_context_for_execution(
            &second_execution.id,
            &id,
            &first_descriptor.revision,
            ContextInjectionRequester::Agent,
            ContextInjectionLifetime::SingleRequest,
            "agent requested a stale descriptor revision",
        )
        .is_err());

    let (resource, _) = runtime
        .load_context_for_execution(
            &second_execution.id,
            &id,
            &second_descriptor.revision,
            ContextInjectionRequester::Agent,
            ContextInjectionLifetime::SingleRequest,
            "agent requested the current exact descriptor revision",
        )
        .unwrap();
    assert_eq!(resource.content.as_deref(), Some("second project context"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn executions_load_context_from_their_pinned_configuration_revision() {
    let root = fixture_root();
    fs::create_dir_all(root.join(".git")).unwrap();
    write(root.join("CONTRIBUTING.md"), "first pinned context");

    let mut runtime = ConductorRuntime::new();
    runtime
        .reload_configuration(configuration_for(&root))
        .unwrap();
    let first_session = runtime.create_session(None, None, fixed_target()).unwrap();
    let first_execution = runtime
        .submit(&first_session.id, "load first pinned context")
        .unwrap();

    let id = ContextResourceId::parse("project-document:CONTRIBUTING.md").unwrap();
    let first_descriptor = runtime
        .context_descriptors_for_execution(&first_execution.id)
        .unwrap()
        .into_iter()
        .find(|descriptor| descriptor.id == id)
        .unwrap();

    write(root.join("CONTRIBUTING.md"), "second pinned context");
    runtime
        .reload_configuration(configuration_for(&root))
        .unwrap();
    let second_session = runtime.create_session(None, None, fixed_target()).unwrap();
    let second_execution = runtime
        .submit(&second_session.id, "load second pinned context")
        .unwrap();
    let second_descriptor = runtime
        .context_descriptors_for_execution(&second_execution.id)
        .unwrap()
        .into_iter()
        .find(|descriptor| descriptor.id == id)
        .unwrap();

    assert_ne!(first_descriptor.revision, second_descriptor.revision);

    let (first_resource, _) = runtime
        .load_context_for_execution(
            &first_execution.id,
            &id,
            &first_descriptor.revision,
            ContextInjectionRequester::Agent,
            ContextInjectionLifetime::SingleRequest,
            "load the first execution's pinned context",
        )
        .unwrap();
    let (second_resource, _) = runtime
        .load_context_for_execution(
            &second_execution.id,
            &id,
            &second_descriptor.revision,
            ContextInjectionRequester::Agent,
            ContextInjectionLifetime::SingleRequest,
            "load the second execution's pinned context",
        )
        .unwrap();

    assert_eq!(
        first_resource.content.as_deref(),
        Some("first pinned context")
    );
    assert_eq!(
        second_resource.content.as_deref(),
        Some("second pinned context")
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn exact_durable_references_resolve_without_aliases() {
    let mut runtime = ConductorRuntime::new();
    let session = runtime.create_session(None, None, fixed_target()).unwrap();
    let execution = runtime
        .submit(&session.id, "resolve exact references")
        .unwrap();
    let assignment = runtime
        .execution_objectives(&execution.id)
        .unwrap()
        .expect("root execution must have a primary objective");

    let objective = runtime
        .resolve_exact_reference(&ExactReference::Objective(assignment.primary.clone()))
        .unwrap();
    assert_public_exact_reference_type(&objective);
    assert_eq!(
        objective
            .objective()
            .expect("expected objective reference")
            .id,
        assignment.primary
    );

    let resolved_execution = runtime
        .resolve_exact_reference(&ExactReference::Execution(execution.id.clone()))
        .unwrap();
    assert_eq!(
        resolved_execution
            .execution()
            .expect("expected execution reference"),
        &execution
    );

    let event = runtime
        .resolve_exact_reference(&ExactReference::Event(1))
        .unwrap();
    assert_eq!(event.event().expect("expected event reference").sequence, 1);
}
