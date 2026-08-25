    #[test]
    fn workspace_phase_checkpoints_only_the_first_writer_after_a_read_boundary() {
        let mut phase = WorkspacePhase::default();

        assert!(!phase.enter(WorkspaceLeaseMode::Read));
        assert!(phase.enter(WorkspaceLeaseMode::Write));
        assert!(!phase.enter(WorkspaceLeaseMode::Write));
        assert!(!phase.enter(WorkspaceLeaseMode::Read));
        assert!(phase.enter(WorkspaceLeaseMode::Write));
    }

    #[test]
    fn explicit_checkpoint_request_persists_twice_within_one_write_phase() {
        let workspace = temporary_database().with_extension("workspace");
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::write(workspace.join("source.txt"), "one").unwrap();
        let database = temporary_database();
        let store = SqliteStore::new(&database);
        let mut runtime = ConductorRuntime::new();
        let mut authority = ExecutionAuthority::read_only();
        authority.filesystem = FilesystemAuthority::Write;
        runtime
            .register_agent(AgentDefinition::new(
                descriptor("agent.writer", CallableKind::Agent),
                authority,
            ))
            .unwrap();
        let workspace_id = WorkspaceId::parse("workspace:checkpoint").unwrap();
        runtime.bind_workspace(workspace_id.clone()).unwrap();
        let session = runtime
            .create_session(None, None, ExecutionTarget::Fixed(model_target()))
            .unwrap();
        let execution = runtime.submit(&session.id, "write").unwrap();
        let mut server = ConductorServer::new(runtime);
        server.store = Some(store.clone());
        server
            .install_workspace_consistency(WorkspaceDescriptor {
                id: workspace_id,
                root: workspace.clone(),
                scratch_paths: BTreeSet::new(),
            })
            .unwrap();

        assert!(server.capture_workspace_checkpoint(&execution.id).is_err());
        server
            .lock_runtime()
            .unwrap()
            .set_state(&execution.id, ExecutionState::Running)
            .unwrap();
        let request = server
            .lock_runtime()
            .unwrap()
            .workspace_lease_request(&execution.id)
            .unwrap();
        let _lease = server.workspace_leases.acquire(request).unwrap();
        server.capture_workspace_checkpoint(&execution.id).unwrap();
        std::fs::write(workspace.join("source.txt"), "two").unwrap();
        server.capture_workspace_checkpoint(&execution.id).unwrap();
        server.persist().unwrap();

        let journal = store.load().unwrap();
        assert_eq!(
            journal
                .entries
                .iter()
                .filter(|entry| matches!(
                    entry.event,
                    DomainEvent::WorkspaceCheckpointCaptured { .. }
                ))
                .count(),
            2
        );
        std::fs::remove_file(database).unwrap();
        std::fs::remove_dir_all(workspace).unwrap();
    }

    #[test]
    fn writer_semantic_checkpoint_requires_and_uses_the_live_lease() {
        let workspace = temporary_database().with_extension("semantic-checkpoint-workspace");
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::write(workspace.join("source.txt"), "one").unwrap();
        let workspace_id = WorkspaceId::parse("workspace:semantic-checkpoint").unwrap();
        let mut runtime = ConductorRuntime::new();
        let mut authority = ExecutionAuthority::read_only();
        authority.filesystem = FilesystemAuthority::Write;
        runtime
            .register_agent(AgentDefinition::new(
                descriptor("agent.writer", CallableKind::Agent),
                authority,
            ))
            .unwrap();
        runtime.bind_workspace(workspace_id.clone()).unwrap();
        let session = runtime
            .create_session(None, None, ExecutionTarget::Fixed(model_target()))
            .unwrap();
        let execution = runtime.submit(&session.id, "write").unwrap();
        runtime
            .set_state(&execution.id, ExecutionState::Running)
            .unwrap();
        let mut server = ConductorServer::new(runtime);
        server
            .install_workspace_consistency(WorkspaceDescriptor {
                id: workspace_id.clone(),
                root: workspace.clone(),
                scratch_paths: BTreeSet::new(),
            })
            .unwrap();
        let checkpoint = CallableId::parse(semantic_tools::WORKSPACE_CHECKPOINT_ID).unwrap();
        let mut host = SharedRuntimeHost {
            runtime: server.runtime.clone(),
            execution_id: execution.id.clone(),
            allowed_tools: BTreeSet::from([checkpoint.clone()]),
            workspace_id: workspace_id.clone(),
            workspace_leases: server.workspace_leases.clone(),
            workspace_consistency: server.workspace_consistency.clone(),
            store: None,
            persist_lock: server.persist_lock.clone(),
        };
        let invocation = || ToolInvocation {
            callable: checkpoint.clone(),
            arguments_json: "{}".to_owned(),
        };

        assert!(!host.invoke_tool(invocation()).unwrap().success);
        let request = server
            .lock_runtime()
            .unwrap()
            .workspace_lease_request(&execution.id)
            .unwrap();
        let _lease = server.workspace_leases.acquire(request).unwrap();
        assert!(host.invoke_tool(invocation()).unwrap().success);
        assert!(server
            .lock_runtime()
            .unwrap()
            .journal()
            .entries
            .iter()
            .any(|entry| matches!(
                entry.event,
                DomainEvent::WorkspaceCheckpointCaptured { ref execution_id, .. }
                    if execution_id == &execution.id
            )));
        std::fs::remove_dir_all(workspace).unwrap();
    }

    #[test]
    fn read_only_sessions_share_workspace_and_execute_concurrently() {
        let gate = ConcurrentGate {
            state: Arc::new((Mutex::new(0), Condvar::new())),
        };
        let mut server = ConductorServer::new(ConductorRuntime::new());
        server
            .register_backend(
                BackendId::parse("fixture").unwrap(),
                Box::new(ConcurrentBackend { gate }),
            )
            .unwrap();
        let target = serde_json::to_string(&ExecutionTarget::Fixed(model_target())).unwrap();
        let input = format!(
            "{{\"id\":1,\"command\":{{\"type\":\"create_session\",\"parent_session\":null,\"name\":\"a\",\"target\":{target}}}}}\n\\
             {{\"id\":2,\"command\":{{\"type\":\"create_session\",\"parent_session\":null,\"name\":\"b\",\"target\":{target}}}}}\n\\
             {{\"id\":3,\"command\":{{\"type\":\"submit\",\"session_id\":\"session-1\",\"text\":\"one\"}}}}\n\\
             {{\"id\":4,\"command\":{{\"type\":\"submit\",\"session_id\":\"session-2\",\"text\":\"two\"}}}}\n"
        );
        server
            .serve_ndjson(std::io::Cursor::new(input), std::io::sink())
            .unwrap();
        let executions = server.runtime().snapshot().executions;
        assert_eq!(executions.len(), 2);
        assert!(
            executions
                .iter()
                .all(|execution| execution.state == ExecutionState::Completed),
            "independent execution states: {executions:?}"
        );
    }
