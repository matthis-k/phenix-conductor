fn migrate(connection: &mut Connection) -> Result<(), PersistenceError> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
             version INTEGER PRIMARY KEY,
             applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
         );",
    )?;
    let version = connection.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get::<_, i64>(0),
    )?;
    if version > DATABASE_SCHEMA_VERSION {
        return Err(invalid(format!(
            "database schema version {version} is newer than supported version {DATABASE_SCHEMA_VERSION}"
        )));
    }
    for (target, sql) in [
        (1, include_str!("../../migrations/0001_runtime.sql")),
        (
            2,
            include_str!("../../migrations/0002_orchestration_data.sql"),
        ),
        (
            3,
            include_str!("../../migrations/0003_diagnostic_patches.sql"),
        ),
        (
            4,
            include_str!("../../migrations/0004_language_observations.sql"),
        ),
        (5, include_str!("../../migrations/0005_objectives.sql")),
        (6, include_str!("../../migrations/0006_plans.sql")),
        (
            7,
            include_str!("../../migrations/0007_context_injections.sql"),
        ),
        (
            8,
            include_str!("../../migrations/0008_context_resource_revisions.sql"),
        ),
    ] {
        if version < target {
            apply_migration(connection, target, sql)?;
        }
    }
    Ok(())
}

fn apply_migration(
    connection: &mut Connection,
    version: i64,
    sql: &str,
) -> Result<(), PersistenceError> {
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    transaction.execute_batch(sql)?;
    transaction.execute(
        "INSERT INTO schema_migrations(version) VALUES (?1)",
        params![version],
    )?;
    transaction.commit()?;
    Ok(())
}

fn metadata(connection: &Connection, key: &str) -> Result<Option<String>, PersistenceError> {
    Ok(connection
        .query_row(
            "SELECT value FROM runtime_metadata WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()?)
}

fn initialize_or_validate_database(
    transaction: &Transaction<'_>,
    journal: &RuntimeJournal,
) -> Result<(), PersistenceError> {
    let existing = transaction
        .query_row(
            "SELECT value FROM runtime_metadata WHERE key = 'initial_config_revision'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let revision = journal.config_revision.to_string();
    let fingerprint = journal.config_fingerprint.to_string();
    match existing {
        None => {
            transaction.execute(
                "INSERT INTO runtime_metadata(key, value) VALUES
                 ('journal_format_version', ?1),
                 ('initial_config_revision', ?2),
                 ('initial_config_fingerprint', ?3)",
                params![journal.format_version.to_string(), revision, fingerprint],
            )?;
            transaction.execute(
                "INSERT INTO configuration_revisions(revision_id, fingerprint, activated_sequence)
                 VALUES (?1, ?2, 0)",
                params![
                    journal.config_revision.to_string(),
                    journal.config_fingerprint.to_string()
                ],
            )?;
        }
        Some(existing_revision) => {
            let existing_format = transaction.query_row(
                "SELECT value FROM runtime_metadata WHERE key = 'journal_format_version'",
                [],
                |row| row.get::<_, String>(0),
            )?;
            let existing_fingerprint = transaction.query_row(
                "SELECT value FROM runtime_metadata WHERE key = 'initial_config_fingerprint'",
                [],
                |row| row.get::<_, String>(0),
            )?;
            if existing_revision != revision
                || existing_format != journal.format_version.to_string()
                || existing_fingerprint != fingerprint
            {
                return Err(invalid(
                    "runtime journal does not match the database identity",
                ));
            }
        }
    }
    Ok(())
}
