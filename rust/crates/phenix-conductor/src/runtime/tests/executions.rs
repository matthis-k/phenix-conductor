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


    #[test]
    fn worker_task_runtime_preserves_scope_authority_and_restore() {
        let mut runtime = ConductorRuntime::new();
        let parent_authority = authority(
            FilesystemAuthority::Write,
            NetworkAuthority::Outbound,
            RepositoryAuthority::Write,
            &[],
            &[],
            &["agent.child"],
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
                parent_authority.clone(),
            ))
            .unwrap();
        let profile_id = WorkerProfileId::parse("worker.task").unwrap();
        runtime
            .register_worker_profile(WorkerProfileDefinition {
                id: profile_id.clone(),
                role: "bounded task".to_owned(),
                agent: CallableId::parse("agent.child").unwrap(),
                authority_maximum: ExecutionAuthority::read_only(),
            })
            .unwrap();

        let session = runtime.create_session(None, None, fixed("fixed")).unwrap();
        let parent = runtime
            .start_session_callable(
                &session.id,
                &CallableId::parse("agent.parent").unwrap(),
                "parent objective",
            )
            .unwrap();
        runtime.ensure_objective_semantics_active().unwrap();
        let assignment = runtime
            .execution_objectives(&parent.id)
            .unwrap()
            .expect("root execution has an objective");

        let step_id = phenix_core::PlanStepId::parse("step-1").unwrap();
        let plan = runtime
            .create_plan(
                std::collections::BTreeSet::from([assignment.primary.clone()]),
                vec![phenix_core::PlanStep {
                    id: step_id.clone(),
                    description: "bounded worker task".to_owned(),
                    state: phenix_core::PlanStepState::Proposed,
                    revisability: phenix_core::PlanStepRevisability::Revisable,
                    depends_on: std::collections::BTreeSet::new(),
                    objective_refs: std::collections::BTreeSet::from([
                        assignment.primary.clone(),
                    ]),
                }],
            )
            .unwrap();
        runtime
            .assign_execution_to_plan_step(&parent.id, &plan.id, &step_id)
            .unwrap();

        let request = crate::WorkerTaskRequest {
            primary_objective: assignment.primary.clone(),
            supporting_objectives: std::collections::BTreeSet::new(),
            plan_step: Some(crate::WorkerPlanStepRef {
                plan_id: plan.id.clone(),
                step_id: step_id.clone(),
            }),
            description: "review the bounded change".to_owned(),
            profile_id: profile_id.clone(),
            depends_on: std::collections::BTreeSet::new(),
            input_refs: Vec::new(),
            expected_result_schema: serde_json::json!({"type": "object"}),
            delegated_authority: ExecutionAuthority::read_only(),
        };
        let task = runtime.create_worker_task(&parent.id, request).unwrap();
        assert!(runtime.worker_task_is_runnable(&task.id).unwrap());

        let child = runtime.start_worker_task(&task.id).unwrap();
        assert_eq!(child.parent_execution.as_ref(), Some(&parent.id));
        assert_eq!(
            runtime.execution_authority(&child.id).unwrap(),
            parent_authority.attenuate(&ExecutionAuthority::read_only())
        );
        assert_eq!(
            runtime.execution_plan(&child.id).unwrap().unwrap().plan_id,
            plan.id
        );
        assert!(matches!(
            runtime.worker_task(&task.id).unwrap().state,
            crate::WorkerTaskState::Running { execution_id } if execution_id == child.id
        ));
        runtime
            .set_state(&child.id, ExecutionState::Completed)
            .unwrap();
        let result_ref = phenix_core::ExactReference::Execution(child.id.clone());
        runtime
            .record_worker_result(
                &task.id,
                crate::WorkerResultEnvelope {
                    task_id: task.id.clone(),
                    execution_id: child.id.clone(),
                    output: serde_json::json!({"summary": "completed"}),
                    evidence_refs: vec![result_ref.clone()],
                    artifact_refs: Vec::new(),
                },
            )
            .unwrap();
        runtime
            .complete_worker_task(&task.id, vec![result_ref.clone()])
            .unwrap();

        let restored = ConductorRuntime::restore(runtime.journal().clone()).unwrap();
        assert!(matches!(
            restored.worker_task(&task.id).unwrap().state,
            crate::WorkerTaskState::Completed { execution_id, result_refs }
                if execution_id == child.id && result_refs == vec![result_ref.clone()]
        ));

        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("phenix-worker-task-{nonce}"));
        std::fs::create_dir_all(&root).unwrap();
        let store = SqliteStore::new(root.join("state.sqlite"));
        store.save(runtime.journal()).unwrap();
        let relational = ConductorRuntime::restore(store.load().unwrap()).unwrap();
        assert!(matches!(
            relational.worker_task(&task.id).unwrap().state,
            crate::WorkerTaskState::Completed { execution_id, result_refs }
                if execution_id == child.id && result_refs == vec![result_ref]
        ));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn worker_task_dependencies_and_failed_attempts_keep_distinct_identity() {
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
        let profile_id = WorkerProfileId::parse("worker.dependencies").unwrap();
        runtime
            .register_worker_profile(WorkerProfileDefinition {
                id: profile_id.clone(),
                role: "dependency worker".to_owned(),
                agent: CallableId::parse("agent.child").unwrap(),
                authority_maximum: ExecutionAuthority::read_only(),
            })
            .unwrap();
        let session = runtime.create_session(None, None, fixed("fixed")).unwrap();
        let parent = runtime
            .start_session_callable(
                &session.id,
                &CallableId::parse("agent.parent").unwrap(),
                "dependency objective",
            )
            .unwrap();
        runtime.ensure_objective_semantics_active().unwrap();
        let objective = runtime
            .execution_objectives(&parent.id)
            .unwrap()
            .unwrap()
            .primary;

        let request = |depends_on| crate::WorkerTaskRequest {
            primary_objective: objective.clone(),
            supporting_objectives: std::collections::BTreeSet::new(),
            plan_step: None,
            description: "bounded dependency work".to_owned(),
            profile_id: profile_id.clone(),
            depends_on,
            input_refs: Vec::new(),
            expected_result_schema: serde_json::json!({"type": "object"}),
            delegated_authority: ExecutionAuthority::read_only(),
        };

        let first = runtime
            .create_worker_task(&parent.id, request(std::collections::BTreeSet::new()))
            .unwrap();
        let second = runtime
            .create_worker_task(
                &parent.id,
                request(std::collections::BTreeSet::from([first.id.clone()])),
            )
            .unwrap();
        assert!(runtime.worker_task_is_runnable(&first.id).unwrap());
        assert!(!runtime.worker_task_is_runnable(&second.id).unwrap());
        assert_eq!(
            runtime
                .runnable_worker_tasks()
                .unwrap()
                .into_iter()
                .map(|task| task.id)
                .collect::<Vec<_>>(),
            vec![first.id.clone()]
        );

        let child = runtime.start_worker_task(&first.id).unwrap();
        runtime
            .set_state(&child.id, ExecutionState::Completed)
            .unwrap();
        runtime
            .record_worker_result(
                &first.id,
                crate::WorkerResultEnvelope {
                    task_id: first.id.clone(),
                    execution_id: child.id.clone(),
                    output: serde_json::json!({}),
                    evidence_refs: Vec::new(),
                    artifact_refs: Vec::new(),
                },
            )
            .unwrap();
        runtime.complete_worker_task(&first.id, Vec::new()).unwrap();
        assert!(runtime.worker_task_is_runnable(&second.id).unwrap());
        assert_eq!(
            runtime
                .runnable_worker_tasks()
                .unwrap()
                .into_iter()
                .map(|task| task.id)
                .collect::<Vec<_>>(),
            vec![second.id.clone()]
        );

        let second_child = runtime.start_worker_task(&second.id).unwrap();
        runtime
            .set_state(&second_child.id, ExecutionState::Failed)
            .unwrap();
        runtime
            .fail_worker_task(&second.id, "approach failed")
            .unwrap();
        assert!(matches!(
            runtime.start_worker_task(&second.id),
            Err(ConductorError::WorkerTask(crate::WorkerTaskError::Blocked(id))) if id == second.id
        ));

        let successor = runtime
            .create_worker_task(&parent.id, request(std::collections::BTreeSet::new()))
            .unwrap();
        assert_ne!(successor.id, second.id);
    }

    #[test]
    fn worker_task_rejects_objective_outside_parent_scope() {
        let mut runtime = ConductorRuntime::new();
        runtime
            .register_agent(AgentDefinition::new(
                agent("agent.parent"),
                ExecutionAuthority::read_only(),
            ))
            .unwrap();
        let session = runtime.create_session(None, None, fixed("fixed")).unwrap();
        let parent = runtime
            .start_session_callable(
                &session.id,
                &CallableId::parse("agent.parent").unwrap(),
                "scoped objective",
            )
            .unwrap();
        runtime.ensure_objective_semantics_active().unwrap();
        let outside = phenix_core::ObjectiveId::parse("objective-outside").unwrap();
        let result = runtime.create_worker_task(
            &parent.id,
            crate::WorkerTaskRequest {
                primary_objective: outside.clone(),
                supporting_objectives: std::collections::BTreeSet::new(),
                plan_step: None,
                description: "must be rejected".to_owned(),
                profile_id: WorkerProfileId::parse("worker.missing").unwrap(),
                depends_on: std::collections::BTreeSet::new(),
                input_refs: Vec::new(),
                expected_result_schema: serde_json::json!({"type": "object"}),
                delegated_authority: ExecutionAuthority::read_only(),
            },
        );
        assert!(matches!(
            result,
            Err(ConductorError::WorkerTask(crate::WorkerTaskError::ObjectiveScope(id))) if id == outside
        ));
    }


    #[test]
    fn worker_task_profile_resolution_uses_parent_pinned_configuration() {
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
        let session = runtime.create_session(None, None, fixed("fixed")).unwrap();
        let parent = runtime
            .start_session_callable(
                &session.id,
                &CallableId::parse("agent.parent").unwrap(),
                "pinned configuration objective",
            )
            .unwrap();
        runtime.ensure_objective_semantics_active().unwrap();
        let objective = runtime
            .execution_objectives(&parent.id)
            .unwrap()
            .unwrap()
            .primary;

        let profile_id = WorkerProfileId::parse("worker.late").unwrap();
        runtime
            .register_worker_profile(WorkerProfileDefinition {
                id: profile_id.clone(),
                role: "registered after parent creation".to_owned(),
                agent: CallableId::parse("agent.child").unwrap(),
                authority_maximum: ExecutionAuthority::read_only(),
            })
            .unwrap();

        let request = crate::WorkerTaskRequest {
            primary_objective: objective,
            supporting_objectives: std::collections::BTreeSet::new(),
            plan_step: None,
            description: "must use parent-pinned configuration".to_owned(),
            profile_id: profile_id.clone(),
            depends_on: std::collections::BTreeSet::new(),
            input_refs: Vec::new(),
            expected_result_schema: serde_json::json!({"type": "object"}),
            delegated_authority: ExecutionAuthority::read_only(),
        };
        assert!(matches!(
            runtime.create_worker_task(&parent.id, request),
            Err(ConductorError::WorkerProfile(WorkerProfileError::Unknown(id))) if id == profile_id
        ));
    }

    #[test]
    fn worker_task_rejects_plan_step_that_is_not_enacted_and_runnable() {
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
        let profile_id = WorkerProfileId::parse("worker.plan-scope").unwrap();
        runtime
            .register_worker_profile(WorkerProfileDefinition {
                id: profile_id.clone(),
                role: "plan scoped worker".to_owned(),
                agent: CallableId::parse("agent.child").unwrap(),
                authority_maximum: ExecutionAuthority::read_only(),
            })
            .unwrap();
        let session = runtime.create_session(None, None, fixed("fixed")).unwrap();
        let parent = runtime
            .start_session_callable(
                &session.id,
                &CallableId::parse("agent.parent").unwrap(),
                "plan scope objective",
            )
            .unwrap();
        runtime.ensure_objective_semantics_active().unwrap();
        let objective = runtime
            .execution_objectives(&parent.id)
            .unwrap()
            .unwrap()
            .primary;
        let step_id = phenix_core::PlanStepId::parse("step-draft").unwrap();
        let plan = runtime
            .create_plan(
                std::collections::BTreeSet::from([objective.clone()]),
                vec![phenix_core::PlanStep {
                    id: step_id.clone(),
                    description: "still prospective".to_owned(),
                    state: phenix_core::PlanStepState::Proposed,
                    revisability: phenix_core::PlanStepRevisability::Revisable,
                    depends_on: std::collections::BTreeSet::new(),
                    objective_refs: std::collections::BTreeSet::from([objective.clone()]),
                }],
            )
            .unwrap();

        let request = crate::WorkerTaskRequest {
            primary_objective: objective,
            supporting_objectives: std::collections::BTreeSet::new(),
            plan_step: Some(crate::WorkerPlanStepRef {
                plan_id: plan.id.clone(),
                step_id: step_id.clone(),
            }),
            description: "must not enact a draft step implicitly".to_owned(),
            profile_id,
            depends_on: std::collections::BTreeSet::new(),
            input_refs: Vec::new(),
            expected_result_schema: serde_json::json!({"type": "object"}),
            delegated_authority: ExecutionAuthority::read_only(),
        };
        assert!(matches!(
            runtime.create_worker_task(&parent.id, request),
            Err(ConductorError::WorkerTask(crate::WorkerTaskError::PlanScope {
                plan_id,
                step_id: rejected_step,
            })) if plan_id == plan.id && rejected_step == step_id
        ));
    }


    #[test]
    fn worker_tasks_use_canonical_workspace_lease_modes() {
        let mut runtime = ConductorRuntime::new();
        let maximum = authority(
            FilesystemAuthority::Write,
            NetworkAuthority::None,
            RepositoryAuthority::Write,
            &[],
            &[],
            &["agent.child"],
        );
        runtime
            .register_agent(AgentDefinition::new(
                agent("agent.parent"),
                maximum.clone(),
            ))
            .unwrap();
        runtime
            .register_agent(AgentDefinition::new(
                agent("agent.child"),
                maximum.clone(),
            ))
            .unwrap();
        let profile_id = WorkerProfileId::parse("worker.lease").unwrap();
        runtime
            .register_worker_profile(WorkerProfileDefinition {
                id: profile_id.clone(),
                role: "lease semantics".to_owned(),
                agent: CallableId::parse("agent.child").unwrap(),
                authority_maximum: maximum,
            })
            .unwrap();
        let session = runtime.create_session(None, None, fixed("fixed")).unwrap();
        let parent = runtime
            .start_session_callable(
                &session.id,
                &CallableId::parse("agent.parent").unwrap(),
                "lease objective",
            )
            .unwrap();
        runtime.ensure_objective_semantics_active().unwrap();
        let objective = runtime
            .execution_objectives(&parent.id)
            .unwrap()
            .unwrap()
            .primary;
        let request = |authority| crate::WorkerTaskRequest {
            primary_objective: objective.clone(),
            supporting_objectives: std::collections::BTreeSet::new(),
            plan_step: None,
            description: "bounded lease work".to_owned(),
            profile_id: profile_id.clone(),
            depends_on: std::collections::BTreeSet::new(),
            input_refs: Vec::new(),
            expected_result_schema: serde_json::json!({"type": "object"}),
            delegated_authority: authority,
        };

        let read_task = runtime
            .create_worker_task(&parent.id, request(ExecutionAuthority::read_only()))
            .unwrap();
        let read_child = runtime.start_worker_task(&read_task.id).unwrap();
        assert_eq!(
            runtime
                .workspace_lease_request(&read_child.id)
                .unwrap()
                .mode,
            phenix_core::WorkspaceLeaseMode::Read
        );

        let mut write_authority = ExecutionAuthority::read_only();
        write_authority.filesystem = FilesystemAuthority::Write;
        write_authority.repository = RepositoryAuthority::Write;
        let write_task = runtime
            .create_worker_task(&parent.id, request(write_authority))
            .unwrap();
        let write_child = runtime.start_worker_task(&write_task.id).unwrap();
        assert_eq!(
            runtime
                .workspace_lease_request(&write_child.id)
                .unwrap()
                .mode,
            phenix_core::WorkspaceLeaseMode::Write
        );
    }


    #[test]
    fn worker_result_schema_and_required_verification_gate_completion() {
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
        let profile_id = WorkerProfileId::parse("worker.verified").unwrap();
        runtime
            .register_worker_profile(WorkerProfileDefinition {
                id: profile_id.clone(),
                role: "verified worker".to_owned(),
                agent: CallableId::parse("agent.child").unwrap(),
                authority_maximum: ExecutionAuthority::read_only(),
            })
            .unwrap();
        let session = runtime.create_session(None, None, fixed("fixed")).unwrap();
        let parent = runtime
            .start_session_callable(
                &session.id,
                &CallableId::parse("agent.parent").unwrap(),
                "verified objective",
            )
            .unwrap();
        runtime.ensure_objective_semantics_active().unwrap();
        let objective = runtime.execution_objectives(&parent.id).unwrap().unwrap().primary;
        let task = runtime
            .create_worker_task(
                &parent.id,
                crate::WorkerTaskRequest {
                    primary_objective: objective,
                    supporting_objectives: std::collections::BTreeSet::new(),
                    plan_step: None,
                    description: "return structured evidence".to_owned(),
                    profile_id,
                    depends_on: std::collections::BTreeSet::new(),
                    input_refs: Vec::new(),
                    expected_result_schema: serde_json::json!({
                        "type": "object",
                        "required": ["summary"],
                        "properties": {"summary": {"type": "string"}}
                    }),
                    delegated_authority: ExecutionAuthority::read_only(),
                },
            )
            .unwrap();
        runtime.require_worker_task_verification(&task.id).unwrap();
        let child = runtime.start_worker_task(&task.id).unwrap();
        runtime.set_state(&child.id, ExecutionState::Completed).unwrap();

        assert!(matches!(
            runtime.record_worker_result(
                &task.id,
                crate::WorkerResultEnvelope {
                    task_id: task.id.clone(),
                    execution_id: child.id.clone(),
                    output: serde_json::json!({"wrong": true}),
                    evidence_refs: Vec::new(),
                    artifact_refs: Vec::new(),
                },
            ),
            Err(ConductorError::WorkerTask(crate::WorkerTaskError::InvalidResult(message)))
                if message.contains("schema")
        ));

        let evidence = phenix_core::ExactReference::Execution(child.id.clone());
        runtime
            .record_worker_result(
                &task.id,
                crate::WorkerResultEnvelope {
                    task_id: task.id.clone(),
                    execution_id: child.id.clone(),
                    output: serde_json::json!({"summary": "done"}),
                    evidence_refs: vec![evidence.clone()],
                    artifact_refs: Vec::new(),
                },
            )
            .unwrap();
        assert!(matches!(
            runtime.complete_worker_task(&task.id, vec![evidence.clone()]),
            Err(ConductorError::WorkerTask(crate::WorkerTaskError::InvalidResult(message)))
                if message.contains("verification")
        ));

        let verifier_authority = runtime.execution_authority(&parent.id).unwrap();
        runtime
            .record_worker_verification(
                &task.id,
                crate::WorkerVerificationResult::Passed {
                    verifier_execution_id: parent.id.clone(),
                    evidence_refs: vec![evidence.clone()],
                },
            )
            .unwrap();
        assert_eq!(runtime.execution_authority(&parent.id).unwrap(), verifier_authority);
        runtime
            .complete_worker_task(&task.id, vec![evidence.clone()])
            .unwrap();

        let parent_projection = runtime.worker_result_for_parent(&task.id).unwrap();
        assert_eq!(parent_projection.output, serde_json::json!({"summary": "done"}));
        assert_eq!(parent_projection.evidence_refs, vec![evidence.clone()]);
        assert!(matches!(
            parent_projection.verification,
            Some(crate::WorkerVerificationResult::Passed { .. })
        ));

        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("phenix-worker-result-{nonce}"));
        std::fs::create_dir_all(&root).unwrap();
        let store = SqliteStore::new(root.join("state.sqlite"));
        store.save(runtime.journal()).unwrap();
        let restored = ConductorRuntime::restore(store.load().unwrap()).unwrap();
        assert_eq!(
            restored.worker_result_for_parent(&task.id).unwrap(),
            parent_projection
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn worker_failure_analysis_is_proposal_only() {
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
        let profile_id = WorkerProfileId::parse("worker.failure-analysis").unwrap();
        runtime
            .register_worker_profile(WorkerProfileDefinition {
                id: profile_id.clone(),
                role: "failure worker".to_owned(),
                agent: CallableId::parse("agent.child").unwrap(),
                authority_maximum: ExecutionAuthority::read_only(),
            })
            .unwrap();
        let session = runtime.create_session(None, None, fixed("fixed")).unwrap();
        let parent = runtime
            .start_session_callable(
                &session.id,
                &CallableId::parse("agent.parent").unwrap(),
                "failure objective",
            )
            .unwrap();
        runtime.ensure_objective_semantics_active().unwrap();
        let objective = runtime.execution_objectives(&parent.id).unwrap().unwrap().primary;
        let before_objective = runtime.objective(&objective).unwrap();
        let task = runtime
            .create_worker_task(
                &parent.id,
                crate::WorkerTaskRequest {
                    primary_objective: objective.clone(),
                    supporting_objectives: std::collections::BTreeSet::new(),
                    plan_step: None,
                    description: "attempt bounded work".to_owned(),
                    profile_id,
                    depends_on: std::collections::BTreeSet::new(),
                    input_refs: Vec::new(),
                    expected_result_schema: serde_json::json!({"type": "object"}),
                    delegated_authority: ExecutionAuthority::read_only(),
                },
            )
            .unwrap();
        let child = runtime.start_worker_task(&task.id).unwrap();
        runtime.set_state(&child.id, ExecutionState::Failed).unwrap();
        runtime.fail_worker_task(&task.id, "failed attempt").unwrap();
        let parent_authority = runtime.execution_authority(&parent.id).unwrap();
        runtime
            .record_worker_failure_analysis(
                &task.id,
                crate::WorkerFailureAnalysis {
                    analyzer_execution_id: parent.id.clone(),
                    diagnosis: "retry with a different approach".to_owned(),
                    evidence_refs: vec![phenix_core::ExactReference::Execution(child.id.clone())],
                    proposed_action: crate::WorkerFailureAction::SuccessorTask,
                },
            )
            .unwrap();
        assert_eq!(runtime.objective(&objective).unwrap(), before_objective);
        assert_eq!(runtime.execution_authority(&parent.id).unwrap(), parent_authority);
        assert!(matches!(
            runtime.worker_task(&task.id).unwrap().state,
            crate::WorkerTaskState::Failed { .. }
        ));
        assert_eq!(runtime.worker_failure_analyses(&task.id).unwrap().len(), 1);
    }
