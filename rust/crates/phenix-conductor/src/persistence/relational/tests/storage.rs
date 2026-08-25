    #[test]
    fn failed_migration_rolls_back_schema_and_version() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE schema_migrations (
                     version INTEGER PRIMARY KEY,
                     applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                 );",
            )
            .unwrap();

        let error = apply_migration(
            &mut connection,
            1,
            "CREATE TABLE migration_probe(value INTEGER);
             INSERT INTO missing_table(value) VALUES (1);",
        )
        .unwrap_err();
        assert!(matches!(error, PersistenceError::Sql(_)));

        let probe_exists = connection
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'migration_probe'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .unwrap();
        let version = connection
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap();
        assert_eq!(probe_exists, None);
        assert_eq!(version, 0);
    }

    #[test]
    fn relational_rows_roundtrip_the_complete_representative_journal() {
        let (directory, store) = temporary_store("roundtrip");
        let journal = representative_journal();

        store.save(&journal).unwrap();
        let loaded = store.load().unwrap();

        assert_eq!(loaded, journal);
        ConductorRuntime::restore(loaded).unwrap();
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn schema_contains_no_json_persistence_columns() {
        let (directory, store) = temporary_store("schema");
        store.save(&representative_journal()).unwrap();
        let connection = Connection::open(store.path()).unwrap();
        let json_columns = connection
            .prepare(
                "SELECT m.name, p.name
                 FROM sqlite_master AS m, pragma_table_info(m.name) AS p
                 WHERE m.type = 'table' AND lower(p.name) LIKE '%json%'
                 ORDER BY m.name, p.name",
            )
            .unwrap()
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let event_columns = connection
            .prepare("SELECT name FROM pragma_table_info('domain_events') ORDER BY cid")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert!(
            json_columns.is_empty(),
            "JSON columns remain: {json_columns:?}"
        );
        assert_eq!(event_columns, ["sequence", "event_type"]);
        drop(connection);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn recovery_reads_relational_facts_and_rejects_missing_facts() {
        let (directory, store) = temporary_store("authority");
        let journal = representative_journal();
        store.save(&journal).unwrap();
        let connection = Connection::open(store.path()).unwrap();
        connection
            .execute("UPDATE sessions SET name = 'database-authority'", [])
            .unwrap();
        drop(connection);

        let loaded = store.load().unwrap();
        assert_ne!(loaded, journal);
        assert!(loaded.entries.iter().any(|entry| matches!(
            &entry.event,
            DomainEvent::SessionCreated { session }
                if session.name.as_deref() == Some("database-authority")
        )));

        let connection = Connection::open(store.path()).unwrap();
        connection
            .execute("DELETE FROM session_renames", [])
            .unwrap();
        drop(connection);
        assert!(store.load().is_err());
        std::fs::remove_dir_all(directory).unwrap();
    }
