use crate::{session_tree_service, SessionTreeCommand};
use phenix_core::{
    session_service, Authority, Kernel, KernelConfig, PhenixValue, Project, ResolvedHarness,
    ResolvedHarnessActivation, SessionCommand, SessionResponse,
};
use phenix_plugin_sessions::{session_component_manifest, session_factory, session_manifest};

#[test]
fn flat_sessions_remain_available_when_session_tree_is_omitted() {
    let sessions = session_manifest();
    let authority = sessions.maximum_authority.clone();
    let session_plugin = sessions.id.clone();
    let resolved = ResolvedHarness::resolve(
        [sessions.clone()],
        [session_component_manifest()],
        [],
        &authority,
    )
    .unwrap();
    let mut kernel = Kernel::new(KernelConfig::new([sessions]).unwrap());
    kernel.activate_resolved_harness(&resolved).unwrap();
    kernel
        .register_embedded_factory(session_plugin, session_factory)
        .unwrap();
    kernel.activate_all().unwrap();

    let command = SessionCommand::Create {
        id: "standalone".into(),
    };
    let output = kernel
        .invoke(
            &session_service(),
            &serde_json::to_vec(&PhenixValue::from(&command)).unwrap(),
            &authority,
            None,
        )
        .unwrap();
    let output: PhenixValue = serde_json::from_slice(&output).unwrap();
    assert_eq!(
        SessionResponse::try_from(Project(&output)).unwrap(),
        SessionResponse::Created {
            session: phenix_core::SessionRecord {
                id: "standalone".into(),
            },
        }
    );

    let error = kernel
        .invoke(
            &session_tree_service(),
            &serde_json::to_vec(&SessionTreeCommand::Children {
                parent_session_id: None,
            })
            .unwrap(),
            &Authority::default(),
            None,
        )
        .unwrap_err();
    assert!(error.to_string().contains("no eligible provider"));
}
