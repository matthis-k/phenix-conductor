    #[test]
    fn child_orchestration_accepts_and_replays_invocation_restrictions() {
        let mut runtime = ConductorRuntime::new();
        let worker = CallableId::parse("agent.worker").unwrap();
        let orchestration = CallableId::parse("orchestration.restricted").unwrap();
        let maximum = authority(
            FilesystemAuthority::Write,
            NetworkAuthority::Outbound,
            RepositoryAuthority::Write,
            &[],
            &[],
            &[],
        );
        runtime
            .register_agent(AgentDefinition::new(agent("agent.worker"), maximum))
            .unwrap();
        runtime
            .register_orchestration(OrchestrationDefinition {
                descriptor: CallableDescriptor {
                    id: orchestration.clone(),
                    kind: CallableKind::Orchestration,
                    description: "restricted orchestration".to_owned(),
                    input_schema: json!({"type": "object"}),
                    output_schema: json!({"type": "object"}),
                    capabilities: CapabilitySet::default(),
                    policy: CallablePolicy::default(),
                },
                interface_agent: None,
                nodes: vec![phenix_core::OrchestrationNode {
                    id: OrchestrationNodeId::parse("work").unwrap(),
                    callable: worker.clone(),
                    depends_on: Vec::new(),
                    objective: Some("work".to_owned()),
                    input_bindings: BTreeMap::new(),
                }],
                output_bindings: BTreeMap::new(),
            })
            .unwrap();
        let session = runtime.create_session(None, None, fixed("fixed")).unwrap();
        let root = runtime.submit(&session.id, "root").unwrap();
        let mut restrictions = ExecutionAuthority::read_only();
        restrictions.callables.insert(worker);
        let execution = runtime
            .start_orchestration_with_restrictions(
                &root.id,
                &orchestration,
                json!({}),
                &restrictions,
            )
            .unwrap();
        let node = runtime
            .snapshot()
            .executions
            .into_iter()
            .find(|candidate| candidate.parent_execution.as_ref() == Some(&execution.id))
            .unwrap();

        assert_eq!(
            runtime.execution_authority(&execution.id).unwrap(),
            restrictions
        );
        assert_eq!(
            runtime.execution_authority(&node.id).unwrap().filesystem,
            FilesystemAuthority::ReadOnly
        );
        let restored = ConductorRuntime::restore(runtime.journal().clone()).unwrap();
        assert_eq!(
            restored.execution_authority(&execution.id).unwrap(),
            restrictions
        );
    }

    #[test]
    fn granted_secret_text_is_scrubbed_before_retry_persistence() {
        let secret_name = "PHENIX_DURABLE_REDACTION_TEST_TOKEN";
        let secret_value = "durable-redaction-value-7f03";
        std::env::set_var(secret_name, secret_value);
        let mut runtime = ConductorRuntime::new();
        let callable = CallableId::parse("agent.secret").unwrap();
        runtime
            .register_agent(AgentDefinition::new(
                agent(callable.as_str()),
                authority(
                    FilesystemAuthority::ReadOnly,
                    NetworkAuthority::None,
                    RepositoryAuthority::Read,
                    &[],
                    &[secret_name],
                    &[],
                ),
            ))
            .unwrap();
        let session = runtime.create_session(None, None, fixed("fixed")).unwrap();
        let root = runtime.submit(&session.id, "root").unwrap();
        let child = runtime
            .start_agent(
                &root.id,
                &callable,
                format!("use {secret_value} without persisting it"),
            )
            .unwrap();
        let tool_call_id = runtime.new_tool_call_id();
        runtime
            .push_event(
                &child.id,
                ExecutionEventKind::ToolCallFinished {
                    tool_call_id,
                    output: format!("failed with credential {secret_value}"),
                    success: false,
                },
            )
            .unwrap();
        runtime
            .set_state(&child.id, ExecutionState::Failed)
            .unwrap();
        let failure = runtime.runtime_failure_summary(&child.id, 1).unwrap();
        let group = AttemptGroup::from_first_failure(
            runtime.new_attempt_group_id(),
            root.id,
            callable,
            format!("goal containing {secret_value}"),
            failure,
        );
        runtime
            .record_domain_event(DomainEvent::AttemptGroupCreated { group })
            .unwrap();
        std::env::remove_var(secret_name);

        let journal = serde_json::to_string(runtime.journal()).unwrap();
        let bundle = runtime
            .build_session_debug_bundle(
                &session.id,
                WorkspaceDescriptor {
                    id: WorkspaceId::parse("workspace:in-memory").unwrap(),
                    root: PathBuf::new(),
                    scratch_paths: BTreeSet::new(),
                },
                &BTreeMap::new(),
            )
            .unwrap();
        let exported = serde_json::to_string(&bundle).unwrap();

        assert!(!journal.contains(secret_value));
        assert!(!exported.contains(secret_value));
        assert_eq!(
            bundle.attempt_groups[0].failures[0].reason,
            "failed with credential [REDACTED]"
        );
    }

    #[test]
    fn failed_orchestration_cancels_active_siblings_and_preserves_terminal_children() {
        let mut runtime = ConductorRuntime::new();
        for callable in ["agent.fail", "agent.active", "agent.done"] {
            runtime
                .register_agent(AgentDefinition::new(
                    agent(callable),
                    ExecutionAuthority::read_only(),
                ))
                .unwrap();
        }
        runtime
            .register_orchestration(OrchestrationDefinition {
                output_bindings: Default::default(),
                interface_agent: None,
                descriptor: CallableDescriptor {
                    id: CallableId::parse("orchestration.parallel").unwrap(),
                    kind: CallableKind::Orchestration,
                    description: "parallel failure fixture".to_owned(),
                    input_schema: json!({"type": "object"}),
                    output_schema: json!({"type": "object"}),
                    capabilities: CapabilitySet::default(),
                    policy: CallablePolicy::default(),
                },
                nodes: vec![
                    phenix_core::OrchestrationNode {
                        input_bindings: Default::default(),
                        id: OrchestrationNodeId::parse("fail").unwrap(),
                        callable: CallableId::parse("agent.fail").unwrap(),
                        depends_on: Vec::new(),
                        objective: None,
                    },
                    phenix_core::OrchestrationNode {
                        input_bindings: Default::default(),
                        id: OrchestrationNodeId::parse("active").unwrap(),
                        callable: CallableId::parse("agent.active").unwrap(),
                        depends_on: Vec::new(),
                        objective: None,
                    },
                    phenix_core::OrchestrationNode {
                        input_bindings: Default::default(),
                        id: OrchestrationNodeId::parse("done").unwrap(),
                        callable: CallableId::parse("agent.done").unwrap(),
                        depends_on: Vec::new(),
                        objective: None,
                    },
                ],
            })
            .unwrap();

        let session = runtime.create_session(None, None, fixed("a")).unwrap();
        let root = runtime.submit(&session.id, "root").unwrap();
        let orchestration = runtime
            .start_orchestration(
                &root.id,
                &CallableId::parse("orchestration.parallel").unwrap(),
                json!({"objective": "parallel work"}),
            )
            .unwrap();
        let children = runtime
            .snapshot()
            .executions
            .into_iter()
            .filter(|execution| execution.parent_execution.as_ref() == Some(&orchestration.id))
            .collect::<Vec<_>>();
        let child = |callable: &str| {
            children
                .iter()
                .find(|execution| {
                    execution
                        .callable
                        .as_ref()
                        .is_some_and(|id| id.as_str() == callable)
                })
                .unwrap()
                .id
                .clone()
        };
        let failing = child("agent.fail");
        let active = child("agent.active");
        let done = child("agent.done");

        runtime.set_state(&done, ExecutionState::Completed).unwrap();
        runtime.set_state(&active, ExecutionState::Running).unwrap();
        runtime
            .set_state(&failing, ExecutionState::Running)
            .unwrap();
        runtime.set_state(&failing, ExecutionState::Failed).unwrap();

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
        assert_eq!(state(&failing), ExecutionState::Failed);
        assert_eq!(state(&active), ExecutionState::Cancelled);
        assert_eq!(state(&done), ExecutionState::Completed);
        assert_eq!(state(&orchestration.id), ExecutionState::Failed);
        assert!(runtime.events_since(0).iter().any(|event| {
            event.execution_id == root.id
                && matches!(
                    &event.kind,
                    ExecutionEventKind::ChildExecutionFinished { child, state }
                        if child == &orchestration.id && *state == ExecutionState::Failed
                )
        }));
    }
