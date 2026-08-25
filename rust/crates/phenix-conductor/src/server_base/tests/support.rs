    #[test]
    fn objective_errors_have_stable_protocol_classes() {
        let unknown =
            map_conductor_error(ConductorError::Objective(ObjectiveError::UnknownObjective(
                phenix_core::ObjectiveId::parse("objective-missing").unwrap(),
            )));
        assert_eq!(unknown.code, ErrorCode::UnknownId);

        let immutable =
            map_conductor_error(ConductorError::Objective(ObjectiveError::RootIsImmutable(
                phenix_core::ObjectiveId::parse("objective-1").unwrap(),
            )));
        assert_eq!(immutable.code, ErrorCode::InvalidRequest);
    }

    #[test]
    fn closed_queue_waits_for_active_group_and_accepts_generated_descendants() {
        let queue = ExecutionQueue::default();
        let session = SessionId::parse("session-1").unwrap();
        queue
            .enqueue(job("execution-1", &session, "group-1"))
            .unwrap();
        let active = queue.next().unwrap().unwrap();
        queue.close().unwrap();

        let waiter = queue.clone();
        let (sender, receiver) = mpsc::channel();
        let thread = std::thread::spawn(move || sender.send(waiter.next().unwrap()).unwrap());
        assert!(receiver.recv_timeout(Duration::from_millis(100)).is_err());

        queue
            .enqueue(job("execution-2", &session, "group-1"))
            .unwrap();
        let generated = receiver
            .recv_timeout(Duration::from_secs(1))
            .unwrap()
            .unwrap();
        assert_eq!(
            generated.execution_id,
            ExecutionId::parse("execution-2").unwrap()
        );

        assert!(!queue.complete(&active, false).unwrap());
        assert!(queue.complete(&generated, true).unwrap());
        thread.join().unwrap();
        assert!(queue.next().unwrap().is_none());
    }

    #[test]
    fn ready_dag_siblings_share_workers_and_generated_join_runs_after_input_eof() {
        let gate = ConcurrentGate {
            state: Arc::new((Mutex::new(0), Condvar::new())),
        };
        let mut runtime = ConductorRuntime::new();
        for callable in ["agent.alpha", "agent.beta", "agent.join"] {
            runtime
                .register_agent(phenix_core::AgentDefinition::new(
                    descriptor(callable, CallableKind::Agent),
                    phenix_core::ExecutionAuthority::read_only(),
                ))
                .unwrap();
        }
        runtime
            .register_orchestration(OrchestrationDefinition {
                output_bindings: Default::default(),
                interface_agent: None,
                descriptor: descriptor("orchestration.parallel", CallableKind::Orchestration),
                nodes: vec![
                    node("alpha", "agent.alpha", &[]),
                    node("beta", "agent.beta", &[]),
                    node("join", "agent.join", &["alpha", "beta"]),
                ],
            })
            .unwrap();

        let mut server = ConductorServer::new(runtime);
        server
            .register_backend(
                BackendId::parse("fixture").unwrap(),
                Box::new(ConcurrentBackend { gate }),
            )
            .unwrap();
        let target = serde_json::to_string(&ExecutionTarget::Fixed(model_target())).unwrap();
        let input = format!(
            "{{\"id\":1,\"command\":{{\"type\":\"create_session\",\"parent_session\":null,\"name\":\"dag\",\"target\":{target}}}}}\n\\
             {{\"id\":2,\"command\":{{\"type\":\"start_callable\",\"session_id\":\"session-1\",\"callable\":\"orchestration.parallel\",\"input\":{{\"objective\":\"run\"}}}}}}\n"
        );
        server
            .serve_ndjson(std::io::Cursor::new(input), std::io::sink())
            .unwrap();

        let executions = server.runtime().snapshot().executions;
        assert_eq!(executions.len(), 4);
        assert!(
            executions
                .iter()
                .all(|execution| execution.state == ExecutionState::Completed),
            "DAG execution states: {executions:?}"
        );
    }
