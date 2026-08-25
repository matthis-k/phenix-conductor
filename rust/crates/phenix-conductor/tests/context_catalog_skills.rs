use phenix_conductor::{CompiledConfiguration, ConductorRuntime, SkillRegistry};
use phenix_core::{
    BackendId, ContextInjectionLifetime, ContextInjectionRequester, ContextResourceId,
    ContextResourceKind, ContextTier, ExecutionTarget, InferenceOptions, ModelId, ModelTarget,
    ProviderId,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("phenix-context-catalog-skills-{nonce}"))
}

fn write(path: impl AsRef<Path>, content: &str) {
    let path = path.as_ref();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn skill_source(instructions: &str) -> String {
    format!("---\nname: review\ndescription: Review changes\n---\n{instructions}\n")
}

fn manual_skill_source(instructions: &str) -> String {
    format!(
        "---\nname: review\ndescription: Review changes\ndisable-model-invocation: true\n---\n{instructions}\n"
    )
}

fn fixed_target() -> ExecutionTarget {
    ExecutionTarget::Fixed(ModelTarget {
        backend: BackendId::parse("mock").unwrap(),
        provider: ProviderId::parse("mock").unwrap(),
        model: ModelId::parse("mock").unwrap(),
        inference: InferenceOptions::default(),
    })
}

#[test]
fn skills_are_exact_revisioned_context_catalog_resources() {
    let root = fixture_root();
    fs::create_dir_all(root.join(".git")).unwrap();
    let path = root.join(".phenix/skills/review/SKILL.md");
    write(&path, &skill_source("first revision"));

    let first = SkillRegistry::discover(&root)
        .unwrap()
        .skill_context_catalog()
        .unwrap();
    let id = ContextResourceId::parse("skill:review").unwrap();
    let first_resource = first.current_revision(&id).unwrap();
    let first_descriptor = &first_resource.descriptor;
    assert_eq!(first_descriptor.kind, ContextResourceKind::Skill);
    assert_eq!(first_resource.tier, ContextTier::DiscoverableContent);
    assert!(first_resource
        .content
        .as_deref()
        .unwrap()
        .contains("first revision"));

    write(&path, &skill_source("second revision"));
    let second = SkillRegistry::discover(&root)
        .unwrap()
        .skill_context_catalog()
        .unwrap();
    let second_resource = second.current_revision(&id).unwrap();
    let second_descriptor = &second_resource.descriptor;

    assert_eq!(first_descriptor.id, second_descriptor.id);
    assert_ne!(first_descriptor.revision, second_descriptor.revision);
    assert!(second_resource
        .content
        .as_deref()
        .unwrap()
        .contains("second revision"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn manual_only_skill_requires_user_requester() {
    let root = fixture_root();
    fs::create_dir_all(root.join(".git")).unwrap();
    write(
        root.join(".phenix/skills/review/SKILL.md"),
        &manual_skill_source("manual-only review instructions"),
    );

    let skills = SkillRegistry::discover(&root).unwrap();
    let catalog = skills.skill_context_catalog().unwrap();
    let id = ContextResourceId::parse("skill:review").unwrap();
    let revision = catalog
        .current_revision(&id)
        .unwrap()
        .descriptor
        .revision
        .clone();

    let mut configuration = CompiledConfiguration::default();
    configuration.install_skill_registry(skills);

    let mut runtime = ConductorRuntime::new();
    runtime.reload_configuration(configuration).unwrap();
    let session = runtime.create_session(None, None, fixed_target()).unwrap();
    let execution = runtime
        .submit(&session.id, "inspect review guidance")
        .unwrap();

    for requester in [
        ContextInjectionRequester::Agent,
        ContextInjectionRequester::Orchestration,
        ContextInjectionRequester::ContextPolicy,
        ContextInjectionRequester::Hook,
        ContextInjectionRequester::Frontend,
    ] {
        assert!(runtime
            .load_context_for_execution(
                &execution.id,
                &id,
                &revision,
                requester,
                ContextInjectionLifetime::SingleRequest,
                "non-user requested manual-only skill",
            )
            .is_err());
    }

    let (resource, injection) = runtime
        .load_context_for_execution(
            &execution.id,
            &id,
            &revision,
            ContextInjectionRequester::User,
            ContextInjectionLifetime::SingleRequest,
            "user explicitly requested manual-only skill",
        )
        .unwrap();
    assert_eq!(resource.descriptor.id, id);
    assert_eq!(injection.requested_by, ContextInjectionRequester::User);

    fs::remove_dir_all(root).unwrap();
}
