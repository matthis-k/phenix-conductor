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
    fn worker_profile_lookup_uses_parent_pinned_configuration() {
        let mut runtime = ConductorRuntime::new();
        let parent_authority = authority(
            FilesystemAuthority::Write,
            NetworkAuthority::Outbound,
            RepositoryAuthority::Write,
            &[],
            &[],
            &["agent.child"],
        );
        let child_authority = authority(
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
                parent_authority,
            ))
            .unwrap();
        runtime
            .register_agent(AgentDefinition::new(
                agent("agent.child"),
                child_authority,
            ))
            .unwrap();

        let old_session = runtime.create_session(None, None, fixed("fixed")).unwrap();
        let old_parent = runtime
            .start_session_callable(
                &old_session.id,
                &CallableId::parse("agent.parent").unwrap(),
                "parent",
            )
            .unwrap();
        let profile_id = WorkerProfileId::parse("worker.review").unwrap();
        runtime
            .register_worker_profile(WorkerProfileDefinition {
                id: profile_id.clone(),
                role: "review".to_owned(),
                agent: CallableId::parse("agent.child").unwrap(),
                authority_maximum: ExecutionAuthority::read_only(),
            })
            .unwrap();

        assert!(matches!(
            runtime.start_worker_profile(&old_parent.id, &profile_id, "review"),
            Err(ConductorError::WorkerProfile(WorkerProfileError::Unknown(id))) if id == profile_id
        ));

        let new_session = runtime.create_session(None, None, fixed("fixed")).unwrap();
        let new_parent = runtime
            .start_session_callable(
                &new_session.id,
                &CallableId::parse("agent.parent").unwrap(),
                "parent",
            )
            .unwrap();
        assert!(runtime
            .start_worker_profile(&new_parent.id, &profile_id, "review")
            .is_ok());
    }

    #[test]
    fn worker_profile_and_invocation_authority_only_attenuate() {
        let mut runtime = ConductorRuntime::new();
        let parent_authority = authority(
            FilesystemAuthority::Write,
            NetworkAuthority::Outbound,
            RepositoryAuthority::Write,
            &["docker"],
            &["github"],
            &["agent.child", "tool.write"],
        );
        let child_authority = parent_authority.clone();
        let profile_maximum = authority(
            FilesystemAuthority::ReadOnly,
            NetworkAuthority::Outbound,
            RepositoryAuthority::Read,
            &[],
            &["github"],
            &["tool.write"],
        );
        let invocation_restrictions = authority(
            FilesystemAuthority::Write,
            NetworkAuthority::None,
            RepositoryAuthority::Write,
            &[],
            &[],
            &["tool.write"],
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
                child_authority.clone(),
            ))
            .unwrap();
        let profile_id = WorkerProfileId::parse("worker.read-mostly").unwrap();
        runtime
            .register_worker_profile(WorkerProfileDefinition {
                id: profile_id.clone(),
                role: "read-mostly".to_owned(),
                agent: CallableId::parse("agent.child").unwrap(),
                authority_maximum: profile_maximum.clone(),
            })
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
            .start_worker_profile_with_restrictions(
                &parent.id,
                &profile_id,
                "child",
                &invocation_restrictions,
            )
            .unwrap();
        let expected = parent_authority
            .attenuate(&child_authority)
            .attenuate(&profile_maximum)
            .attenuate(&invocation_restrictions);

        assert_eq!(runtime.execution_authority(&child.id).unwrap(), expected);
        assert_eq!(
            runtime.execution_authority(&child.id).unwrap().filesystem,
            FilesystemAuthority::ReadOnly
        );
        assert_eq!(
            runtime.execution_authority(&child.id).unwrap().network,
            NetworkAuthority::None
        );
    }

    #[test]
    fn worker_profile_registration_changes_fingerprint_deterministically() {
        fn configured() -> ConductorRuntime {
            let mut runtime = ConductorRuntime::new();
            runtime
                .register_agent(AgentDefinition::new(
                    agent("agent.worker"),
                    ExecutionAuthority::read_only(),
                ))
                .unwrap();
            runtime
        }

        let mut left = configured();
        let before = left
            .current_compiled_configuration()
            .unwrap()
            .fingerprint();
        left.register_worker_profile(WorkerProfileDefinition {
            id: WorkerProfileId::parse("worker.deterministic").unwrap(),
            role: "deterministic".to_owned(),
            agent: CallableId::parse("agent.worker").unwrap(),
            authority_maximum: ExecutionAuthority::read_only(),
        })
        .unwrap();
        let after = left
            .current_compiled_configuration()
            .unwrap()
            .fingerprint();
        assert_ne!(before, after);

        let mut right = configured();
        right
            .register_worker_profile(WorkerProfileDefinition {
                id: WorkerProfileId::parse("worker.deterministic").unwrap(),
                role: "deterministic".to_owned(),
                agent: CallableId::parse("agent.worker").unwrap(),
                authority_maximum: ExecutionAuthority::read_only(),
            })
            .unwrap();
        assert_eq!(
            after,
            right
                .current_compiled_configuration()
                .unwrap()
                .fingerprint()
        );
    }

    #[test]
    fn worker_profile_binding_survives_journal_and_relational_restore() {
        let mut runtime = ConductorRuntime::new();
        let parent_authority = authority(
            FilesystemAuthority::ReadOnly,
            NetworkAuthority::None,
            RepositoryAuthority::Read,
            &[],
            &[],
            &["agent.child"],
        );
        runtime.register_agent(AgentDefinition::new(agent("agent.parent"), parent_authority)).unwrap();
        runtime.register_agent(AgentDefinition::new(agent("agent.child"), ExecutionAuthority::read_only())).unwrap();
        let profile_id = WorkerProfileId::parse("worker.restore").unwrap();
        runtime.register_worker_profile(WorkerProfileDefinition {
            id: profile_id.clone(),
            role: "restore".to_owned(),
            agent: CallableId::parse("agent.child").unwrap(),
            authority_maximum: ExecutionAuthority::read_only(),
        }).unwrap();
        let session = runtime.create_session(None, None, fixed("fixed")).unwrap();
        let parent = runtime.start_session_callable(&session.id, &CallableId::parse("agent.parent").unwrap(), "parent").unwrap();
        let worker = runtime.start_worker_profile(&parent.id, &profile_id, "restore").unwrap();
        assert_eq!(
            runtime.execution_worker_profile(&worker.id).unwrap(),
            Some(profile_id.clone())
        );
        let expected_revision = runtime.execution_config_revision(&worker.id).unwrap();

        let restored = ConductorRuntime::restore(runtime.journal().clone()).unwrap();
        assert_eq!(
            restored.execution_worker_profile(&worker.id).unwrap(),
            Some(profile_id.clone())
        );
        assert_eq!(
            restored.execution_config_revision(&worker.id).unwrap(),
            expected_revision
        );

        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("phenix-worker-profile-{nonce}"));
        std::fs::create_dir_all(&root).unwrap();
        let store = SqliteStore::new(root.join("state.sqlite"));
        store.save(runtime.journal()).unwrap();
        let relational = ConductorRuntime::restore(store.load().unwrap()).unwrap();
        assert_eq!(
            relational.execution_worker_profile(&worker.id).unwrap(),
            Some(profile_id)
        );
        assert_eq!(
            relational.execution_config_revision(&worker.id).unwrap(),
            expected_revision
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn worker_profiles_get_independent_context_projections() {
        let mut runtime = ConductorRuntime::new();
        let parent_authority = authority(
            FilesystemAuthority::ReadOnly,
            NetworkAuthority::None,
            RepositoryAuthority::Read,
            &[],
            &[],
            &["agent.child"],
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
                ExecutionAuthority::read_only(),
            ))
            .unwrap();
        let profile_id = WorkerProfileId::parse("worker.context").unwrap();
        runtime
            .register_worker_profile(WorkerProfileDefinition {
                id: profile_id.clone(),
                role: "context".to_owned(),
                agent: CallableId::parse("agent.child").unwrap(),
                authority_maximum: ExecutionAuthority::read_only(),
            })
            .unwrap();

        let session = runtime.create_session(None, None, fixed("fixed")).unwrap();
        let parent = runtime
            .start_session_callable(
                &session.id,
                &CallableId::parse("agent.parent").unwrap(),
                "parent",
            )
            .unwrap();
        let left = runtime
            .start_worker_profile(&parent.id, &profile_id, "left")
            .unwrap();
        let right = runtime
            .start_worker_profile(&parent.id, &profile_id, "right")
            .unwrap();
        let objective = runtime
            .context_descriptors_for_execution(&left.id)
            .unwrap()
            .into_iter()
            .find(|descriptor| descriptor.kind == ContextResourceKind::Objective)
            .expect("worker execution has a durable objective context resource");
        runtime
            .load_context_for_execution(
                &left.id,
                &objective.id,
                &objective.revision,
                phenix_core::ContextInjectionRequester::Agent,
                phenix_core::ContextInjectionLifetime::Execution,
                "left worker only",
            )
            .unwrap();

        let left_projection = runtime.project_execution_context(&left.id).unwrap();
        let right_projection = runtime.project_execution_context(&right.id).unwrap();
        assert_eq!(left_projection.execution_id, left.id);
        assert_eq!(right_projection.execution_id, right.id);
        assert_eq!(left_projection.injections.len(), 1);
        assert_eq!(left_projection.injections[0].reason, "left worker only");
        assert!(right_projection.injections.is_empty());
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
