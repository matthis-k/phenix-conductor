use crate::{
    session_tree_component_manifest, session_tree_factory, session_tree_manifest,
    session_tree_service, SessionTreeCommand, SessionTreeResponse,
};
use phenix_core::{
    Authority, Kernel, KernelConfig, LocalPersistence, ResolvedHarness, ResolvedHarnessActivation,
    SessionCommand,
};
use phenix_plugin_sessions::{session_component_manifest, session_factory, session_manifest};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

fn authority() -> Authority {
    Authority::new(
        session_manifest()
            .maximum_authority
            .capabilities()
            .cloned()
            .chain(
                session_tree_manifest()
                    .maximum_authority
                    .capabilities()
                    .cloned(),
            ),
    )
}

fn temp_db() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "phenix-session-tree-atomicity-{}-{nonce}.sqlite",
        std::process::id()
    ))
}

fn kernel_with(path: &PathBuf) -> Kernel {
    let sessions = session_manifest();
    let tree = session_tree_manifest();
    let session_plugin = sessions.id.clone();
    let tree_plugin = tree.id.clone();
    let resolved = ResolvedHarness::resolve(
        [sessions.clone(), tree.clone()],
        [
            session_component_manifest(),
            session_tree_component_manifest(),
        ],
        [],
        &authority(),
    )
    .unwrap();
    let mut kernel = Kernel::with_persistence(
        KernelConfig::new([sessions, tree]).unwrap(),
        LocalPersistence::open(path).unwrap(),
    );
    kernel.activate_resolved_harness(&resolved).unwrap();
    kernel
        .register_embedded_factory(session_plugin, session_factory)
        .unwrap();
    kernel
        .register_embedded_factory(tree_plugin, session_tree_factory)
        .unwrap();
    kernel.activate_all().unwrap();
    kernel
}

fn invoke_session(kernel: &mut Kernel, command: SessionCommand) {
    kernel
        .invoke(
            &phenix_core::session_service(),
            &serde_json::to_vec(&command).unwrap(),
            &authority(),
            None,
        )
        .unwrap();
}

fn invoke_tree(
    kernel: &mut Kernel,
    command: SessionTreeCommand,
) -> Result<SessionTreeResponse, String> {
    let output = kernel
        .invoke(
            &session_tree_service(),
            &serde_json::to_vec(&command).unwrap(),
            &authority(),
            None,
        )
        .map_err(|error| error.to_string())?;
    serde_json::from_slice(&output).map_err(|error| error.to_string())
}

fn children(kernel: &mut Kernel, parent: &str) -> Vec<String> {
    match invoke_tree(
        kernel,
        SessionTreeCommand::Children {
            parent_session_id: Some(parent.into()),
        },
    )
    .unwrap()
    {
        SessionTreeResponse::Children { session_ids } => session_ids,
        other => panic!("unexpected session-tree response: {other:?}"),
    }
}

fn parent(kernel: &mut Kernel, session: &str) -> Option<String> {
    match invoke_tree(
        kernel,
        SessionTreeCommand::Parent {
            session_id: session.into(),
        },
    )
    .unwrap()
    {
        SessionTreeResponse::Parent { parent_session_id } => parent_session_id,
        other => panic!("unexpected session-tree response: {other:?}"),
    }
}

#[test]
fn rejected_reparent_does_not_partially_mutate_lineage_indexes() {
    let path = temp_db();
    let mut kernel = kernel_with(&path);
    for id in ["root-a", "root-b", "child"] {
        invoke_session(&mut kernel, SessionCommand::Create { id: id.into() });
    }

    invoke_tree(
        &mut kernel,
        SessionTreeCommand::Link {
            session_id: "child".into(),
            parent_session_id: Some("root-a".into()),
        },
    )
    .unwrap();

    let error = invoke_tree(
        &mut kernel,
        SessionTreeCommand::Link {
            session_id: "child".into(),
            parent_session_id: Some("root-b".into()),
        },
    )
    .unwrap_err();
    assert!(error.contains("session lineage already exists"));

    assert_eq!(children(&mut kernel, "root-a"), vec!["child"]);
    assert!(children(&mut kernel, "root-b").is_empty());
    assert_eq!(parent(&mut kernel, "child"), Some("root-a".into()));

    drop(kernel);
    let mut restored = kernel_with(&path);
    assert_eq!(children(&mut restored, "root-a"), vec!["child"]);
    assert!(children(&mut restored, "root-b").is_empty());
    let _ = fs::remove_file(path);
}

#[test]
fn rejected_cycle_does_not_partially_mutate_lineage_indexes() {
    let path = temp_db();
    let mut kernel = kernel_with(&path);
    for id in ["root", "child"] {
        invoke_session(&mut kernel, SessionCommand::Create { id: id.into() });
    }

    invoke_tree(
        &mut kernel,
        SessionTreeCommand::Link {
            session_id: "child".into(),
            parent_session_id: Some("root".into()),
        },
    )
    .unwrap();

    let error = invoke_tree(
        &mut kernel,
        SessionTreeCommand::Link {
            session_id: "root".into(),
            parent_session_id: Some("child".into()),
        },
    )
    .unwrap_err();
    assert!(error.contains("session lineage would contain a cycle"));

    assert_eq!(parent(&mut kernel, "child"), Some("root".into()));
    assert_eq!(parent(&mut kernel, "root"), None);
    assert_eq!(children(&mut kernel, "root"), vec!["child"]);
    assert!(children(&mut kernel, "child").is_empty());

    drop(kernel);
    let mut restored = kernel_with(&path);
    assert_eq!(parent(&mut restored, "child"), Some("root".into()));
    assert_eq!(parent(&mut restored, "root"), None);
    assert_eq!(children(&mut restored, "root"), vec!["child"]);
    assert!(children(&mut restored, "child").is_empty());
    let _ = fs::remove_file(path);
}

#[test]
fn combined_child_session_and_lineage_operation_commits_as_one_semantic_operation() {
    let path = temp_db();
    let mut kernel = kernel_with(&path);
    invoke_session(&mut kernel, SessionCommand::Create { id: "root".into() });

    let response = invoke_tree(
        &mut kernel,
        SessionTreeCommand::CreateChild {
            session_id: "child".into(),
            parent_session_id: "root".into(),
        },
    )
    .unwrap();
    assert!(matches!(
        response,
        SessionTreeResponse::ChildCreated { session, lineage }
            if session.id == "child"
                && lineage.session_id == "child"
                && lineage.parent_session_id.as_deref() == Some("root")
    ));
    assert_eq!(parent(&mut kernel, "child"), Some("root".into()));
    assert_eq!(children(&mut kernel, "root"), vec!["child"]);

    drop(kernel);
    let mut restored = kernel_with(&path);
    let child = restored
        .invoke(
            &phenix_core::session_service(),
            &serde_json::to_vec(&SessionCommand::Get { id: "child".into() }).unwrap(),
            &authority(),
            None,
        )
        .unwrap();
    let child: phenix_core::SessionResponse = serde_json::from_slice(&child).unwrap();
    assert!(matches!(
        child,
        phenix_core::SessionResponse::Session { session: Some(session) } if session.id == "child"
    ));
    assert_eq!(parent(&mut restored, "child"), Some("root".into()));
    assert_eq!(children(&mut restored, "root"), vec!["child"]);
    let _ = fs::remove_file(path);
}
