    #[test]
    fn close_rejects_nonterminal_execution() {
        let mut runtime = ConductorRuntime::new();
        let session = runtime.create_session(None, None, fixed("a")).unwrap();
        runtime.submit(&session.id, "work").unwrap();
        assert!(matches!(
            runtime.close_session(&session.id),
            Err(ConductorError::SessionHasActiveExecutions(id)) if id == session.id
        ));
    }

    #[test]
    fn child_authority_is_attenuated_by_parent() {
        let mut runtime = ConductorRuntime::new();
        let parent_authority = authority(
            FilesystemAuthority::ReadOnly,
            NetworkAuthority::Outbound,
            RepositoryAuthority::Read,
            &["dbus"],
            &["github"],
            &["agent.child", "tool.read"],
        );
        let child_maximum = authority(
            FilesystemAuthority::Write,
            NetworkAuthority::Outbound,
            RepositoryAuthority::Write,
            &["dbus", "docker"],
            &["github", "other"],
            &["tool.read", "tool.write"],
        );
        runtime
            .register_agent(AgentDefinition::new(
                agent("agent.parent"),
                parent_authority.clone(),
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
            .start_agent(
                &parent.id,
                &CallableId::parse("agent.child").unwrap(),
                "child",
            )
            .unwrap();

        assert_eq!(
            runtime.execution_authority(&parent.id).unwrap(),
            parent_authority
        );
        assert_eq!(
            runtime.execution_authority(&child.id).unwrap(),
            parent_authority.attenuate(&child_maximum)
        );
    }

    #[test]
    fn execution_authority_roundtrips_and_rejects_parent_expansion() {
        let mut runtime = ConductorRuntime::new();
        let parent_authority = authority(
            FilesystemAuthority::ReadOnly,
            NetworkAuthority::None,
            RepositoryAuthority::Read,
            &[],
            &[],
            &["agent.child"],
        );
        let child_maximum = authority(
            FilesystemAuthority::Write,
            NetworkAuthority::Outbound,
            RepositoryAuthority::Write,
            &[],
            &[],
            &[],
        );
        runtime
            .register_agent(AgentDefinition::new(
                agent("agent.parent"),
                parent_authority.clone(),
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
            .start_agent(
                &parent.id,
                &CallableId::parse("agent.child").unwrap(),
                "child",
            )
            .unwrap();
        let journal = runtime.journal().clone();
        let restored = ConductorRuntime::restore(journal.clone()).unwrap();
        assert_eq!(
            restored.execution_authority(&child.id).unwrap(),
            parent_authority.attenuate(&child_maximum)
        );

        let mut corrupted = journal.clone();
        let child_payload = corrupted
            .entries
            .iter_mut()
            .find_map(|entry| match &mut entry.event {
                DomainEvent::ExecutionCreated { execution, payload }
                    if execution.id == child.id =>
                {
                    Some(payload)
                }
                _ => None,
            })
            .expect("child creation is durable");
        child_payload.set_authority(authority(
            FilesystemAuthority::Write,
            NetworkAuthority::None,
            RepositoryAuthority::Read,
            &[],
            &[],
            &[],
        ));
        assert!(matches!(
            ConductorRuntime::restore(corrupted),
            Err(PersistenceError::InvalidJournal(message)) if message.contains("authority exceeds parent")
        ));

        let mut corrupted = journal;
        let child_execution = corrupted
            .entries
            .iter_mut()
            .find_map(|entry| match &mut entry.event {
                DomainEvent::ExecutionCreated { execution, .. } if execution.id == child.id => {
                    Some(execution)
                }
                _ => None,
            })
            .expect("child creation is durable");
        child_execution.callable = Some(CallableId::parse("agent.other").unwrap());
        assert!(matches!(
            ConductorRuntime::restore(corrupted),
            Err(PersistenceError::InvalidJournal(message)) if message.contains("not delegated by parent")
        ));
    }
