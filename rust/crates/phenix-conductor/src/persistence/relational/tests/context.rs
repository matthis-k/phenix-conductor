    #[test]
    fn context_injections_roundtrip_all_typed_relational_variants() {
        let (directory, store) = temporary_store("context-injections");
        let mut runtime = ConductorRuntime::new();
        let session = runtime
            .create_session(None, None, fixed_target("context"))
            .unwrap();
        let execution = runtime.submit(&session.id, "context persistence").unwrap();
        let sources = vec![
            ExactReference::Objective(ObjectiveId::parse("objective-source").unwrap()),
            ExactReference::Plan(PlanId::parse("plan-source").unwrap()),
            ExactReference::Execution(execution.id.clone()),
            ExactReference::Event(7),
            ExactReference::FileObservation(FileObservationId::parse("file-source").unwrap()),
            ExactReference::LanguageObservation(
                LanguageObservationId::parse("language-source").unwrap(),
            ),
            ExactReference::Context {
                resource_id: ContextResourceId::parse("context-source").unwrap(),
                revision: ContextRevision::parse("revision-6").unwrap(),
            },
        ];
        let requesters = [
            ContextInjectionRequester::Agent,
            ContextInjectionRequester::User,
            ContextInjectionRequester::Orchestration,
            ContextInjectionRequester::ContextPolicy,
            ContextInjectionRequester::Hook,
            ContextInjectionRequester::Frontend,
        ];
        let lifetimes = [
            ContextInjectionLifetime::SingleRequest,
            ContextInjectionLifetime::Execution,
            ContextInjectionLifetime::Objective,
        ];

        for (index, source_ref) in sources.into_iter().enumerate() {
            runtime
                .record_domain_event(DomainEvent::ContextInjectionRecorded {
                    injection: ContextInjection {
                        execution_id: execution.id.clone(),
                        source_ref,
                        source_revision: ContextRevision::parse(format!("revision-{index}"))
                            .unwrap(),
                        requested_by: requesters[index % requesters.len()].clone(),
                        reason: format!("reason-{index}"),
                        lifetime: lifetimes[index % lifetimes.len()].clone(),
                        content_identity: ContextRevision::parse(format!("content-{index}"))
                            .unwrap(),
                    },
                })
                .unwrap();
        }

        let journal = runtime.journal().clone();
        store.save(&journal).unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded, journal);
        ConductorRuntime::restore(loaded).unwrap();
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn relational_context_injection_rejects_unknown_tokens_and_negative_event_sequences() {
        let (directory, store) = temporary_store("context-invalid");
        let mut runtime = ConductorRuntime::new();
        let session = runtime
            .create_session(None, None, fixed_target("context-invalid"))
            .unwrap();
        let execution = runtime
            .submit(&session.id, "invalid context persistence")
            .unwrap();
        runtime
            .record_domain_event(DomainEvent::ContextInjectionRecorded {
                injection: ContextInjection {
                    execution_id: execution.id.clone(),
                    source_ref: ExactReference::Context {
                        resource_id: ContextResourceId::parse("context-source").unwrap(),
                        revision: ContextRevision::parse("revision").unwrap(),
                    },
                    source_revision: ContextRevision::parse("revision").unwrap(),
                    requested_by: ContextInjectionRequester::Agent,
                    reason: "reason".to_owned(),
                    lifetime: ContextInjectionLifetime::Execution,
                    content_identity: ContextRevision::parse("content").unwrap(),
                },
            })
            .unwrap();
        store.save(runtime.journal()).unwrap();

        let connection = Connection::open(store.path()).unwrap();
        connection
            .execute("UPDATE context_injections SET source_kind = 'unknown'", [])
            .unwrap();
        drop(connection);
        assert!(store.load().is_err());

        std::fs::remove_file(store.path()).unwrap();
        store.save(runtime.journal()).unwrap();
        let connection = Connection::open(store.path()).unwrap();
        connection
            .execute(
                "UPDATE context_injections
                 SET source_kind = 'event', source_id = NULL, source_event_sequence = -1",
                [],
            )
            .unwrap();
        drop(connection);
        assert!(store.load().is_err());
        std::fs::remove_dir_all(directory).unwrap();
    }


    #[test]
    fn context_checkpoints_roundtrip_and_restore_raw_provenance() {
        let (directory, store) = temporary_store("context-checkpoints");
        let mut runtime = ConductorRuntime::new();
        let model = ModelTarget {
            backend: BackendId::parse("mock").unwrap(),
            provider: ProviderId::parse("mock").unwrap(),
            model: ModelId::parse("compactor").unwrap(),
            inference: InferenceOptions::default(),
        };
        let session = runtime
            .create_session(
                None,
                None,
                ExecutionTarget::Fixed(model.clone()),
            )
            .unwrap();
        let execution = runtime.submit(&session.id, "checkpoint persistence").unwrap();
        let covered = vec![ContextHistoryRange {
            start_sequence: 1,
            end_sequence: runtime.journal().entries.last().unwrap().sequence,
        }];
        runtime
            .record_domain_event(DomainEvent::ContextCheckpointRecorded {
                checkpoint: ContextCheckpoint {
                    execution_id: execution.id.clone(),
                    summary: "persisted summary".to_owned(),
                    covered_history: covered.clone(),
                    retained_refs: vec![ExactReference::Execution(execution.id.clone())],
                    generation: ContextCheckpointGeneration {
                        model,
                        previous_checkpoint_sequence: None,
                    },
                },
            })
            .unwrap();
        let expected = runtime.latest_context_checkpoint(&execution.id).unwrap().1.clone();

        store.save(runtime.journal()).unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded, *runtime.journal());
        let restored = ConductorRuntime::restore(loaded).unwrap();
        let restored_checkpoint = restored.latest_context_checkpoint(&execution.id).unwrap().1;
        assert_eq!(restored_checkpoint, &expected);
        assert_eq!(restored_checkpoint.covered_history, covered);
        assert_eq!(
            restored_checkpoint.retained_refs,
            vec![ExactReference::Execution(execution.id)]
        );
        std::fs::remove_dir_all(directory).unwrap();
    }
