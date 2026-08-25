    #[test]
    fn invocation_restrictions_are_attenuated_and_replayed() {
        let mut runtime = ConductorRuntime::new();
        let parent_authority = authority(
            FilesystemAuthority::Write,
            NetworkAuthority::Outbound,
            RepositoryAuthority::Write,
            &["/run/parent.sock"],
            &["TOKEN", "OTHER"],
            &["agent.child", "tool.write"],
        );
        let child_maximum = authority(
            FilesystemAuthority::Write,
            NetworkAuthority::Outbound,
            RepositoryAuthority::Write,
            &["/run/parent.sock", "/run/other.sock"],
            &["TOKEN"],
            &["tool.write"],
        );
        let restrictions = authority(
            FilesystemAuthority::ReadOnly,
            NetworkAuthority::None,
            RepositoryAuthority::Read,
            &["/run/parent.sock"],
            &["TOKEN"],
            &[],
        );
        runtime
            .register_agent(AgentDefinition::new(
                agent("agent.parent"),
                parent_authority,
            ))
            .unwrap();
        runtime
            .register_agent(AgentDefinition::new(
                agent("agent.child"),
                child_maximum.clone(),
            ))
            .unwrap();
        let session = runtime.create_session(None, None, fixed("fixed")).unwrap();
        let parent = runtime
            .start_session_callable(
                &session.id,
                &CallableId::parse("agent.parent").unwrap(),
                "parent",
            )
            .unwrap();
        let child = runtime
            .start_agent_with_restrictions(
                &parent.id,
                &CallableId::parse("agent.child").unwrap(),
                "child",
                &restrictions,
            )
            .unwrap();
        let expected = child_maximum.attenuate(&restrictions);
        assert_eq!(runtime.execution_authority(&child.id).unwrap(), expected);

        let restored = ConductorRuntime::restore(runtime.journal().clone()).unwrap();
        assert_eq!(restored.execution_authority(&child.id).unwrap(), expected);
    }

    #[test]
    fn resolved_invocation_filters_tools_by_execution_authority() {
        let mut runtime = ConductorRuntime::new();
        runtime
            .register_agent(AgentDefinition::new(
                agent("agent.reader"),
                ExecutionAuthority::read_only(),
            ))
            .unwrap();
        runtime
            .register_tool(tool("tool.read", &[CAPABILITY_FILESYSTEM_READ]), |_| {
                Ok("read".to_owned())
            })
            .unwrap();
        runtime
            .register_tool(tool("tool.write", &[CAPABILITY_FILESYSTEM_WRITE]), |_| {
                Ok("write".to_owned())
            })
            .unwrap();

        let session = runtime.create_session(None, None, fixed("fixed")).unwrap();
        let execution = runtime
            .start_session_callable(
                &session.id,
                &CallableId::parse("agent.reader").unwrap(),
                "inspect",
            )
            .unwrap();
        let resolved = runtime.resolve_invocation(&execution.id).unwrap();
        let tools = resolved
            .tools
            .callables
            .iter()
            .map(|descriptor| descriptor.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(tools, vec!["tool.read"]);
    }

    #[test]
    fn resolved_invocation_is_journaled_once_and_reused() {
        let mut runtime = ConductorRuntime::new();
        let profile = RoutingProfileId::parse("default").unwrap();
        let concrete = ModelTarget {
            backend: BackendId::parse("mock").unwrap(),
            provider: ProviderId::parse("mock").unwrap(),
            model: ModelId::parse("routed").unwrap(),
            inference: InferenceOptions::default(),
        };
        runtime
            .register_routing_profile(RoutingProfile {
                id: profile.clone(),
                default_target: concrete.clone(),
                callable_targets: BTreeMap::new(),
            })
            .unwrap();
        let session = runtime
            .create_session(None, None, ExecutionTarget::Routed(profile.clone()))
            .unwrap();
        let execution = runtime.submit(&session.id, "work").unwrap();
        let first = runtime.resolve_invocation(&execution.id).unwrap();
        let journal_len = runtime.journal.entries.len();
        let second = runtime.resolve_invocation(&execution.id).unwrap();

        assert_eq!(first.model, concrete);
        assert_eq!(first, second);
        assert_eq!(runtime.journal.entries.len(), journal_len);
        assert!(runtime.journal.entries.iter().any(|entry| {
            matches!(
                &entry.event,
                DomainEvent::InvocationResolved { execution_id, .. }
                    if execution_id == &execution.id
            )
        }));
    }
