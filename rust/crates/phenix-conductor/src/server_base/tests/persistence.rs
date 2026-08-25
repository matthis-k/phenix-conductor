    #[test]
    fn shared_service_routes_responses_per_connection_and_persists_ingress_order() {
        let database = temporary_database();
        let store = SqliteStore::new(&database);
        let workspace_id = WorkspaceId::parse("workspace:multi-client").unwrap();
        let mut runtime = ConductorRuntime::new();
        runtime.bind_workspace(workspace_id).unwrap();
        let session = runtime
            .create_session(
                None,
                Some("shared".to_owned()),
                ExecutionTarget::Fixed(model_target()),
            )
            .unwrap();
        let mut server = ConductorServer::new(runtime);
        server.store = Some(store.clone());
        server.persist().unwrap();
        server
            .register_backend(
                BackendId::parse("fixture").unwrap(),
                Box::new(ImmediateBackend),
            )
            .unwrap();
        let service = ConductorService::new(server).unwrap();

        let first_service = service.clone();
        let first_session = session.id.clone();
        let first = thread::spawn(move || {
            connection_request(
                first_service,
                ClientMessage {
                    id: 7,
                    command: Command::Submit {
                        session_id: first_session,
                        text: "first".to_owned(),
                    },
                },
            )
        });
        let second_service = service.clone();
        let second_session = session.id.clone();
        let second = thread::spawn(move || {
            connection_request(
                second_service,
                ClientMessage {
                    id: 7,
                    command: Command::Submit {
                        session_id: second_session,
                        text: "second".to_owned(),
                    },
                },
            )
        });
        let Reply::Execution { execution: first } = first.join().unwrap() else {
            panic!("first frontend received the wrong reply");
        };
        let Reply::Execution { execution: second } = second.join().unwrap() else {
            panic!("second frontend received the wrong reply");
        };
        assert_ne!(first.id, second.id);

        let connection = rusqlite::Connection::open(&database).unwrap();
        let accepted = connection
            .prepare(
                "SELECT execution_id FROM accepted_root_submissions
                 WHERE session_id = ?1 ORDER BY ingress_order",
            )
            .unwrap()
            .query_map(params![session.id.to_string()], |row| {
                row.get::<_, String>(0)
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(accepted.len(), 2);
        assert!(accepted.contains(&first.id.to_string()));
        assert!(accepted.contains(&second.id.to_string()));

        let cursor = service
            .inner
            .server
            .lock()
            .unwrap()
            .runtime()
            .events_since(0)[0]
            .sequence;
        let Reply::Initialized { events, .. } = connection_request(
            service.clone(),
            ClientMessage {
                id: 7,
                command: Command::Initialize {
                    after_sequence: Some(cursor),
                },
            },
        ) else {
            panic!("reconnecting frontend received the wrong reply");
        };
        assert!(!events.is_empty());
        assert!(events.iter().all(|event| event.sequence > cursor));
        assert_eq!(
            service
                .inner
                .server
                .lock()
                .unwrap()
                .runtime()
                .event_subscription_count(),
            0
        );

        drop(connection);
        drop(service);
        std::fs::remove_file(database).unwrap();
    }
