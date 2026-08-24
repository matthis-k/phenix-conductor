use phenix_conductor::{CompiledConfiguration, ConductorRuntime, ContextRegistry, SkillRegistry};
use phenix_core::{
    BackendId, ContextInjectionLifetime, ContextInjectionRequester, ContextResourceId,
    ExactReference, ExecutionTarget, InferenceOptions, ModelId, ModelTarget, ProviderId,
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
