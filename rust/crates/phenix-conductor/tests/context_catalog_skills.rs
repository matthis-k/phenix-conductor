use phenix_conductor::SkillRegistry;
use phenix_core::{ContextResourceId, ContextResourceKind, ContextTier};
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
    let first_descriptor = first.current_descriptor(&id).unwrap();
    assert_eq!(first_descriptor.kind, ContextResourceKind::Skill);
    let first_revision = first
        .resolve_revision(&id, &first_descriptor.revision)
        .unwrap();
    assert_eq!(first_revision.tier, ContextTier::DiscoverableContent);
    assert!(first_revision
        .content
        .as_deref()
        .unwrap()
        .contains("first revision"));

    write(&path, &skill_source("second revision"));
    let second = SkillRegistry::discover(&root)
        .unwrap()
        .skill_context_catalog()
        .unwrap();
    let second_descriptor = second.current_descriptor(&id).unwrap();

    assert_eq!(first_descriptor.id, second_descriptor.id);
    assert_ne!(first_descriptor.revision, second_descriptor.revision);
    assert!(second
        .resolve_revision(&id, &second_descriptor.revision)
        .unwrap()
        .content
        .as_deref()
        .unwrap()
        .contains("second revision"));

    fs::remove_dir_all(root).unwrap();
}
