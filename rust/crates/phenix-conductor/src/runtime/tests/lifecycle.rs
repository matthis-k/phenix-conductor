    #[test]
    fn debug_bundle_is_complete_and_redacts_granted_secret_fields() {
        let mut runtime = ConductorRuntime::new();
        runtime
            .register_agent(AgentDefinition::new(
                agent("agent.debug"),
                authority(
                    FilesystemAuthority::Write,
                    NetworkAuthority::None,
                    RepositoryAuthority::Read,
                    &[],
                    &["TOKEN"],
                    &[],
                ),
            ))
            .unwrap();
        runtime
            .register_agent(AgentDefinition::new(
                agent("agent.audit"),
                ExecutionAuthority::read_only(),
            ))
            .unwrap();
        let workspace_id = WorkspaceId::parse("workspace:debug").unwrap();
        runtime.bind_workspace(workspace_id.clone()).unwrap();
        let session = runtime.create_session(None, None, fixed("fixed")).unwrap();
        let root = runtime.submit(&session.id, "inspect").unwrap();
        let audit = runtime
            .start_agent(
                &root.id,
                &CallableId::parse("agent.audit").unwrap(),
                "attempt a diagnostic edit",
            )
            .unwrap();
        runtime
            .record_domain_event(DomainEvent::DiagnosticWritePatchCaptured {
                patch: DiagnosticWritePatch {
                    execution_id: audit.id,
                    path: PathBuf::from("src/lib.rs"),
                    patch: "+diagnostic only\n".to_owned(),
                },
            })
            .unwrap();
        runtime.resolve_invocation(&root.id).unwrap();
        let tool_call_id = runtime.new_tool_call_id();
        runtime
            .push_event(
                &root.id,
                ExecutionEventKind::ToolCallStarted {
                    tool_call_id: tool_call_id.clone(),
                    callable: CallableId::parse("debug.tool").unwrap(),
                },
            )
            .unwrap();
        runtime
            .push_event(
                &root.id,
                ExecutionEventKind::ToolCallArguments {
                    tool_call_id: tool_call_id.clone(),
                    arguments: json!({"TOKEN": "credential-value", "safe": true}).to_string(),
                },
            )
            .unwrap();
        runtime
            .push_event(
                &root.id,
                ExecutionEventKind::ToolCallFinished {
                    tool_call_id,
                    output: "done".to_owned(),
                    success: true,
                },
            )
            .unwrap();
        runtime
            .push_event(
                &root.id,
                ExecutionEventKind::AssistantContentDelta {
                    text: "result".to_owned(),
                },
            )
            .unwrap();
        runtime
            .record_domain_event(DomainEvent::WorkspaceCheckpointCaptured {
                execution_id: root.id.clone(),
                workspace_id: workspace_id.clone(),
                files: BTreeMap::new(),
            })
            .unwrap();
        let bundle = runtime
            .build_session_debug_bundle(
                &session.id,
                WorkspaceDescriptor {
                    id: workspace_id,
                    root: PathBuf::from("/debug-workspace"),
                    scratch_paths: BTreeSet::new(),
                },
                &BTreeMap::new(),
            )
            .unwrap();
        let serialized = serde_json::to_string(&bundle).unwrap();

        assert_eq!(bundle.executions.len(), 2);
        assert_eq!(bundle.resolved_routing.len(), 1);
        assert_eq!(bundle.tool_activity.len(), 3);
        assert_eq!(bundle.checkpoints.len(), 1);
        assert_eq!(bundle.diagnostic_write_patches.len(), 1);
        assert_eq!(bundle.conversation.len(), 2);
        assert!(bundle.workspace_authority[&root.id].secrets.is_empty());
        assert!(!serialized.contains("credential-value"));
        assert!(serialized.contains("[REDACTED]"));
    }

    #[test]
    fn cancellation_cascades_to_descendants() {
        let mut runtime = ConductorRuntime::new();
        runtime
            .register_agent(AgentDefinition::new(
                agent("scout"),
                ExecutionAuthority::read_only(),
            ))
            .unwrap();
        let session = runtime.create_session(None, None, fixed("a")).unwrap();
        let root = runtime.submit(&session.id, "work").unwrap();
        let child = runtime
            .start_agent(&root.id, &CallableId::parse("scout").unwrap(), "child")
            .unwrap();
        runtime.cancel_execution(&root.id).unwrap();
        let snapshot = runtime.snapshot();
        assert!(snapshot
            .executions
            .iter()
            .filter(|execution| execution.id == root.id || execution.id == child.id)
            .all(|execution| execution.state == ExecutionState::Cancelled));
    }

    #[test]
    fn failed_parent_cancels_deep_active_subtree() {
        let mut runtime = ConductorRuntime::new();
        runtime
            .register_agent(AgentDefinition::new(
                agent("agent.parent"),
                authority(
                    FilesystemAuthority::ReadOnly,
                    NetworkAuthority::None,
                    RepositoryAuthority::Read,
                    &[],
                    &[],
                    &["agent.child"],
                ),
            ))
            .unwrap();
        runtime
            .register_agent(AgentDefinition::new(
                agent("agent.child"),
                ExecutionAuthority::read_only(),
            ))
            .unwrap();
        let session = runtime.create_session(None, None, fixed("a")).unwrap();
        let root = runtime.submit(&session.id, "root").unwrap();
        let parent = runtime
            .start_agent(
                &root.id,
                &CallableId::parse("agent.parent").unwrap(),
                "parent",
            )
            .unwrap();
        let child = runtime
            .start_agent(
                &parent.id,
                &CallableId::parse("agent.child").unwrap(),
                "child",
            )
            .unwrap();
        runtime
            .set_state(&parent.id, ExecutionState::Running)
            .unwrap();
        runtime
            .set_state(&child.id, ExecutionState::Running)
            .unwrap();

        runtime.set_state(&root.id, ExecutionState::Failed).unwrap();

        let snapshot = runtime.snapshot();
        let state = |id: &ExecutionId| {
            snapshot
                .executions
                .iter()
                .find(|execution| &execution.id == id)
                .unwrap()
                .state
                .clone()
        };
        assert_eq!(state(&root.id), ExecutionState::Failed);
        assert_eq!(state(&parent.id), ExecutionState::Cancelled);
        assert_eq!(state(&child.id), ExecutionState::Cancelled);
    }
