    #[test]
    fn observation_ids_are_conductor_owned_and_survive_sqlite_restore() {
        use phenix_core::{
            ExactReference, FileKind, LanguageOperationResult, LanguageProviderId,
            LanguageServiceKind,
        };

        let mut runtime = ConductorRuntime::new();
        let session = runtime
            .create_session(None, None, fixed("observation"))
            .unwrap();
        let execution = runtime.submit(&session.id, "observe").unwrap();
        runtime
            .set_state(&execution.id, ExecutionState::Running)
            .unwrap();

        let file = runtime
            .record_file_observation(
                &execution.id,
                FileObservationInput {
                    path: PathBuf::from("src/lib.rs"),
                    version: FileVersion::Present {
                        content_hash: "sha256:file".to_owned(),
                        kind: FileKind::Regular,
                    },
                },
            )
            .unwrap();
        let language = runtime
            .record_language_observation(LanguageObservationInput {
                execution: execution.id.clone(),
                workspace: session.workspace_id.clone(),
                service: LanguageServiceKind::parse("rust").unwrap(),
                provider: LanguageProviderId::parse("rust-analyzer").unwrap(),
                provider_epoch: 1,
                operation: LanguageOperation::WorkspaceSymbols {
                    query: "symbol".to_owned(),
                },
                result: LanguageOperationResult {
                    value: json!({"symbols": []}),
                    documents: Vec::new(),
                },
            })
            .unwrap();

        let file_ref = ExactReference::FileObservation(file.id.clone());
        let language_ref = ExactReference::LanguageObservation(language.id.clone());
        assert_eq!(
            runtime
                .resolve_exact_reference(&file_ref)
                .unwrap()
                .file_observation(),
            Some(&file)
        );
        assert_eq!(
            runtime
                .resolve_exact_reference(&language_ref)
                .unwrap()
                .language_observation(),
            Some(&language)
        );

        let db = std::env::temp_dir().join(format!(
            "phenix-observation-identity-{}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&db);
        let store = SqliteStore::new(&db);
        store.save(runtime.journal()).unwrap();
        let restored = ConductorRuntime::restore(store.load().unwrap()).unwrap();
        assert_eq!(
            restored
                .resolve_exact_reference(&file_ref)
                .unwrap()
                .file_observation(),
            Some(&file)
        );
        assert_eq!(
            restored
                .resolve_exact_reference(&language_ref)
                .unwrap()
                .language_observation(),
            Some(&language)
        );
        let _ = std::fs::remove_file(&db);
        let _ = std::fs::remove_file(db.with_extension("sqlite-wal"));
        let _ = std::fs::remove_file(db.with_extension("sqlite-shm"));
    }
