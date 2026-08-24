use phenix_conductor::{CompiledConfiguration, ConductorRuntime, ContextRegistry, SkillRegistry};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("phenix-context-catalog-config-identity-{nonce}"))
}

fn write(path: impl AsRef<Path>, content: &str) {
    let path = path.as_ref();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn configuration(root: &Path) -> CompiledConfiguration {
    let mut configuration = CompiledConfiguration::default();
    configuration.install_context_registry(ContextRegistry::discover(root).unwrap());
    configuration
}

fn configuration_with_skills(root: &Path) -> CompiledConfiguration {
    let mut configuration = configuration(root);
    configuration.install_skill_registry(SkillRegistry::discover(root).unwrap());
    configuration
}

fn skill_source(instructions: &str) -> String {
    format!("---\nname: review\ndescription: Review changes\n---\n{instructions}\n")
}

#[test]
fn discoverable_project_document_bytes_do_not_change_configuration_identity() {
    let root = fixture_root();
    fs::create_dir_all(root.join(".git")).unwrap();
    write(root.join("AGENTS.md"), "mandatory agent rules");
    write(
        root.join("CONTRIBUTING.md"),
        "first contribution instructions",
    );
    write(
        root.join("DEVELOPMENT.md"),
        "first development instructions",
    );

    let first = configuration(&root);
    let mut runtime = ConductorRuntime::new();
    let revision = runtime.reload_configuration(first).unwrap();
    let journal = runtime.journal().clone();

    write(
        root.join("CONTRIBUTING.md"),
        "second contribution instructions",
    );
    write(
        root.join("DEVELOPMENT.md"),
        "second development instructions",
    );
    let second = configuration(&root);

    let mut restored = ConductorRuntime::restore(journal).unwrap();
    restored
        .bind_configuration_revision(&revision, second)
        .expect(
            "discoverable project-document bytes must not change immutable configuration identity",
        );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn discoverable_skill_bytes_do_not_change_configuration_identity() {
    let root = fixture_root();
    fs::create_dir_all(root.join(".git")).unwrap();
    write(root.join("AGENTS.md"), "mandatory agent rules");
    let skill_path = root.join(".phenix/skills/review/SKILL.md");
    write(&skill_path, &skill_source("first skill instructions"));

    let first = configuration_with_skills(&root);
    let mut runtime = ConductorRuntime::new();
    let revision = runtime.reload_configuration(first).unwrap();
    let journal = runtime.journal().clone();

    write(&skill_path, &skill_source("second skill instructions"));
    let second = configuration_with_skills(&root);

    let mut restored = ConductorRuntime::restore(journal).unwrap();
    restored
        .bind_configuration_revision(&revision, second)
        .expect("discoverable skill bytes must not change immutable configuration identity");

    fs::remove_dir_all(root).unwrap();
}
