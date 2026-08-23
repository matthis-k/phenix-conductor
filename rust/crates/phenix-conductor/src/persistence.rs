mod relational;

pub use relational::{PersistenceError, SqliteStore};

use phenix_core::WorkspaceDescriptor;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

impl SqliteStore {
    /// Bind this database to one canonical workspace identity.
    ///
    /// Older databases are backfilled only when every persisted session already
    /// belongs to the requested workspace. Once present, the identity is
    /// immutable; an explicit state-path override changes location, not identity.
    pub fn ensure_workspace_identity(
        &self,
        workspace: &WorkspaceDescriptor,
    ) -> Result<(), PersistenceError> {
        let canonical_root = workspace.root.to_str().ok_or_else(|| {
            PersistenceError::InvalidJournal(
                "canonical workspace root is not valid UTF-8 and cannot be persisted".to_owned(),
            )
        })?;
        let workspace_id = workspace.id.to_string();
        let mut connection = Connection::open(self.path())?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;

        let stored_id = workspace_metadata(&transaction, "workspace_id")?;
        let stored_root = workspace_metadata(&transaction, "workspace_root")?;
        match (stored_id, stored_root) {
            (None, None) => {
                let incompatible_session = transaction
                    .query_row(
                        "SELECT workspace_id FROM sessions WHERE workspace_id <> ?1 LIMIT 1",
                        params![workspace_id],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()?;
                if let Some(other) = incompatible_session {
                    return Err(PersistenceError::InvalidJournal(format!(
                        "workspace database contains session state for {other}, not {workspace_id}"
                    )));
                }
                transaction.execute(
                    "INSERT INTO runtime_metadata(key, value) VALUES
                     ('workspace_id', ?1),
                     ('workspace_root', ?2)",
                    params![workspace_id, canonical_root],
                )?;
            }
            (Some(stored_id), Some(stored_root))
                if stored_id == workspace_id && stored_root == canonical_root => {}
            (Some(stored_id), Some(stored_root)) => {
                return Err(PersistenceError::InvalidJournal(format!(
                    "workspace database belongs to {stored_id} at {stored_root}, not {workspace_id} at {canonical_root}"
                )));
            }
            _ => {
                return Err(PersistenceError::InvalidJournal(
                    "workspace database contains incomplete workspace identity metadata".to_owned(),
                ));
            }
        }

        transaction.commit()?;
        Ok(())
    }
}

fn workspace_metadata(
    connection: &Connection,
    key: &str,
) -> Result<Option<String>, PersistenceError> {
    Ok(connection
        .query_row(
            "SELECT value FROM runtime_metadata WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_core::WorkspaceId;
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn workspace(id: &str, root: &str) -> WorkspaceDescriptor {
        WorkspaceDescriptor {
            id: WorkspaceId::parse(id).unwrap(),
            root: PathBuf::from(root),
            scratch_paths: BTreeSet::new(),
        }
    }

    fn database() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "phenix-workspace-identity-{}-{nonce}.db",
            std::process::id()
        ));
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE runtime_metadata (
                     key TEXT PRIMARY KEY,
                     value TEXT NOT NULL
                 ) STRICT;
                 CREATE TABLE sessions (
                     session_id TEXT PRIMARY KEY,
                     workspace_id TEXT NOT NULL
                 ) STRICT;",
            )
            .unwrap();
        path
    }

    #[test]
    fn workspace_identity_is_backfilled_and_then_immutable() {
        let path = database();
        let store = SqliteStore::new(&path);
        let first = workspace("workspace:/repo", "/repo");
        store.ensure_workspace_identity(&first).unwrap();
        store.ensure_workspace_identity(&first).unwrap();

        let error = store
            .ensure_workspace_identity(&workspace("workspace:/other", "/other"))
            .unwrap_err();
        assert!(matches!(error, PersistenceError::InvalidJournal(_)));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn legacy_database_is_not_rebound_across_existing_session_state() {
        let path = database();
        let connection = Connection::open(&path).unwrap();
        connection
            .execute(
                "INSERT INTO sessions(session_id, workspace_id) VALUES ('session-1', 'workspace:/old')",
                [],
            )
            .unwrap();
        drop(connection);

        let error = SqliteStore::new(&path)
            .ensure_workspace_identity(&workspace("workspace:/new", "/new"))
            .unwrap_err();
        assert!(matches!(error, PersistenceError::InvalidJournal(_)));
        fs::remove_file(path).unwrap();
    }
}
