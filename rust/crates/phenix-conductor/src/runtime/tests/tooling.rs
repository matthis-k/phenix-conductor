    #[test]
    fn fixed_parent_forces_callable_child_target() {
        let mut runtime = ConductorRuntime::new();
        runtime
            .register_agent(AgentDefinition::new(
                agent("scout"),
                ExecutionAuthority::read_only(),
            ))
            .unwrap();
        let session = runtime.create_session(None, None, fixed("fixed")).unwrap();
        let root = runtime.submit(&session.id, "work").unwrap();
        let child = runtime
            .start_agent(&root.id, &CallableId::parse("scout").unwrap(), "child")
            .unwrap();
        assert_eq!(child.target, fixed("fixed"));
    }

    #[test]
    fn child_creation_requires_parent_callable_delegation() {
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
                    &["agent.allowed"],
                ),
            ))
            .unwrap();
        for callable in ["agent.allowed", "agent.denied"] {
            runtime
                .register_agent(AgentDefinition::new(
                    agent(callable),
                    ExecutionAuthority::read_only(),
                ))
                .unwrap();
        }
        let session = runtime.create_session(None, None, fixed("fixed")).unwrap();
        let parent = runtime
            .start_session_callable(
                &session.id,
                &CallableId::parse("agent.parent").unwrap(),
                "parent",
            )
            .unwrap();
        let before = runtime.snapshot().executions.len();
        let denied = CallableId::parse("agent.denied").unwrap();

        assert_eq!(
            runtime
                .start_agent(&parent.id, &denied, "denied child")
                .unwrap_err(),
            ConductorError::DelegationDenied {
                parent_execution: parent.id,
                callable: denied,
            }
        );
        assert_eq!(runtime.snapshot().executions.len(), before);
    }
