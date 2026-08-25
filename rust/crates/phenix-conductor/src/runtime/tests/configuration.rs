    #[test]
    fn root_authority_is_the_configured_agent_envelope() {
        let mut runtime = ConductorRuntime::new();
        let scout = authority(
            FilesystemAuthority::ReadOnly,
            NetworkAuthority::Outbound,
            RepositoryAuthority::Read,
            &["dbus"],
            &[],
            &["agent.worker"],
        );
        let worker = authority(
            FilesystemAuthority::Write,
            NetworkAuthority::None,
            RepositoryAuthority::Write,
            &[],
            &["github"],
            &["tool.write"],
        );
        runtime
            .register_agent(AgentDefinition::new(agent("agent.scout"), scout.clone()))
            .unwrap();
        runtime
            .register_agent(AgentDefinition::new(agent("agent.worker"), worker.clone()))
            .unwrap();
        let session = runtime.create_session(None, None, fixed("fixed")).unwrap();
        let root = runtime.submit(&session.id, "work").unwrap();

        let mut expected = authority_envelope([&scout, &worker]);
        expected.callables.extend([
            CallableId::parse("agent.scout").unwrap(),
            CallableId::parse("agent.worker").unwrap(),
        ]);
        assert_eq!(runtime.execution_authority(&root.id).unwrap(), expected);
    }
