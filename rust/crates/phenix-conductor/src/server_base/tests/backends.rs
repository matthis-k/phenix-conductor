    #[test]
    fn cancel_only_session_type_satisfies_backend_session_contract() {
        let session: Arc<dyn BackendSession> = Arc::new(CancelOnlySession {
            calls: Arc::new(AtomicUsize::new(0)),
        });
        let _ = BackendSessionRequest {
            model: model_target(),
            tools: phenix_backend::ToolProvision::default()
                .prepare(&phenix_backend::BackendCapabilities {
                    tool_presentations: BTreeSet::new(),
                    images: false,
                    persistent_sessions: false,
                })
                .unwrap(),
        };
        assert!(Arc::strong_count(&session) >= 1);
    }
