    #[test]
    fn cancelling_root_reaches_active_descendant_scope_without_crossing_unrelated_execution() {
        let descendant_calls = Arc::new(AtomicUsize::new(0));
        let unrelated_calls = Arc::new(AtomicUsize::new(0));
        let mut runtime = ConductorRuntime::new();
        runtime
            .register_agent(phenix_core::AgentDefinition::new(
                descriptor("agent.child", CallableKind::Agent),
                phenix_core::ExecutionAuthority::read_only(),
            ))
            .unwrap();
        runtime
            .register_orchestration(OrchestrationDefinition {
                output_bindings: Default::default(),
                interface_agent: None,
                descriptor: descriptor("orchestration.tree", CallableKind::Orchestration),
                nodes: vec![OrchestrationNode {
                    input_bindings: Default::default(),
                    id: OrchestrationNodeId::parse("child").unwrap(),
                    callable: CallableId::parse("agent.child").unwrap(),
                    depends_on: Vec::new(),
                    objective: Some("child".to_owned()),
                }],
            })
            .unwrap();

        let session = runtime
            .create_session(None, None, ExecutionTarget::Fixed(model_target()))
            .unwrap();
        let root = runtime.submit(&session.id, "root").unwrap();
        let orchestration = runtime
            .start_orchestration(
                &root.id,
                &CallableId::parse("orchestration.tree").unwrap(),
                json!({"objective": "tree"}),
            )
            .unwrap();
        let child = runtime
            .snapshot()
            .executions
            .into_iter()
            .find(|execution| execution.parent_execution.as_ref() == Some(&orchestration.id))
            .unwrap();
        runtime
            .set_state(&child.id, ExecutionState::Running)
            .unwrap();

        let unrelated_session = runtime
            .create_session(None, None, ExecutionTarget::Fixed(model_target()))
            .unwrap();
        let unrelated = runtime.submit(&unrelated_session.id, "unrelated").unwrap();
        runtime
            .set_state(&unrelated.id, ExecutionState::Running)
            .unwrap();

        let server = ConductorServer::new(runtime);
        {
            let mut scopes = server.active_scopes.lock().unwrap();
            scopes.insert(
                child.id.clone(),
                LiveExecutionScope::Backend(Arc::new(CancelOnlySession {
                    calls: descendant_calls.clone(),
                })),
            );
            scopes.insert(
                unrelated.id.clone(),
                LiveExecutionScope::Backend(Arc::new(CancelOnlySession {
                    calls: unrelated_calls.clone(),
                })),
            );
        }

        assert_eq!(server.cancel_execution(&root.id).unwrap(), Reply::Accepted);
        assert_eq!(descendant_calls.load(Ordering::SeqCst), 1);
        assert_eq!(unrelated_calls.load(Ordering::SeqCst), 0);

        let runtime = server.runtime();
        for id in [&root.id, &orchestration.id, &child.id] {
            assert_eq!(runtime.execution_state(id), Some(ExecutionState::Cancelled));
        }
        assert_eq!(
            runtime.execution_state(&unrelated.id),
            Some(ExecutionState::Running)
        );
    }

    #[test]
    fn execution_queue_allows_one_group_to_fan_out_without_admitting_another_session_group() {
        let queue = ExecutionQueue::default();
        let first_session = SessionId::parse("session-1").unwrap();
        let second_session = SessionId::parse("session-2").unwrap();
        queue
            .enqueue(job("execution-1", &first_session, "group-1"))
            .unwrap();
        queue
            .enqueue(job("execution-2", &first_session, "group-1"))
            .unwrap();
        queue
            .enqueue(job("execution-3", &first_session, "group-2"))
            .unwrap();
        queue
            .enqueue(job("execution-4", &second_session, "group-3"))
            .unwrap();

        let first = queue.next().unwrap().unwrap();
        assert_eq!(
            first.execution_id,
            ExecutionId::parse("execution-1").unwrap()
        );
        let sibling = queue.next().unwrap().unwrap();
        assert_eq!(
            sibling.execution_id,
            ExecutionId::parse("execution-2").unwrap()
        );
        let independent = queue.next().unwrap().unwrap();
        assert_eq!(
            independent.execution_id,
            ExecutionId::parse("execution-4").unwrap()
        );

        assert!(!queue.complete(&first, false).unwrap());
        assert!(queue.complete(&sibling, true).unwrap());
        let next_group = queue.next().unwrap().unwrap();
        assert_eq!(
            next_group.execution_id,
            ExecutionId::parse("execution-3").unwrap()
        );

        assert!(queue.complete(&next_group, true).unwrap());
        assert!(queue.complete(&independent, true).unwrap());
        queue.close().unwrap();
        assert!(queue.next().unwrap().is_none());
    }
