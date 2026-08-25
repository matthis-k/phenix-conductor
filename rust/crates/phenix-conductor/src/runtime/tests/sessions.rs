    #[test]
    fn session_lineage_is_distinct_from_execution_parentage() {
        let mut runtime = ConductorRuntime::new();
        let root = runtime.create_session(None, None, fixed("a")).unwrap();
        let fork = runtime.fork_session(&root.id, None).unwrap();
        let execution = runtime.submit(&fork.id, "work").unwrap();
        assert_eq!(fork.parent_session, Some(root.id));
        assert_eq!(execution.parent_execution, None);
    }

    #[test]
    fn sessions_bind_to_runtime_workspace_and_forks_inherit_it() {
        let mut runtime = ConductorRuntime::new();
        let workspace = WorkspaceId::parse("workspace:/repo").unwrap();
        runtime.bind_workspace(workspace.clone()).unwrap();
        let root = runtime.create_session(None, None, fixed("a")).unwrap();
        let fork = runtime.fork_session(&root.id, None).unwrap();

        assert_eq!(root.workspace_id, workspace);
        assert_eq!(fork.workspace_id, root.workspace_id);
        assert!(matches!(
            runtime.bind_workspace(WorkspaceId::parse("workspace:/other").unwrap()),
            Err(ConductorError::WorkspaceMismatch { .. })
        ));
    }

    #[test]
    fn closed_session_is_durable_terminal_but_can_be_forked() {
        let mut runtime = ConductorRuntime::new();
        let session = runtime.create_session(None, None, fixed("a")).unwrap();
        let closed = runtime.close_session(&session.id).unwrap();
        assert_eq!(closed.state, SessionState::Closed);
        assert_eq!(runtime.close_session(&session.id).unwrap(), closed);
        assert!(matches!(
            runtime.submit(&session.id, "more"),
            Err(ConductorError::ClosedSession(id)) if id == session.id
        ));
        let fork = runtime
            .fork_session(&session.id, Some("continuation".to_owned()))
            .unwrap();
        assert_eq!(fork.parent_session, Some(session.id));
        assert_eq!(fork.state, SessionState::Active);
    }
