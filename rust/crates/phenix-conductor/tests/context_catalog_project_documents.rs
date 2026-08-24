use phenix_conductor::{ContextRegistry, SkillRegistry};
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
