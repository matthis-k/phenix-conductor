#[path = "plan_relational.rs"]
mod plan_relational;

use crate::{
    journal::{apply_domain_event, DurableProjection},
    ConductorRuntime, ConfigRevisionFingerprint, ConfigRevisionSlot, DomainEvent,
    JournalExecutionPayload, ResolvedRoute, RuntimeJournal, WorkerProfileId,
};
use phenix_core::{
    AttemptGroup, AttemptGroupId, BackendId, CallableId, ContextDescriptor, ContextInjection,
    ContextInjectionLifetime, ContextInjectionRequester, ContextResourceId, ContextResourceKind,
    ContextResourceRevision, ContextRevision, ContextScope, ContextTier, DiagnosticWritePatch,
    ExactReference, ExecutionAuthority, ExecutionEvent, ExecutionEventKind, ExecutionId,
    ExecutionKind, ExecutionObjectiveAssignment, ExecutionState, ExecutionSummary, ExecutionTarget,
    ExecutionTerminationCause, FailureAttemptSummary, FileKind, FileObservation, FileObservationId,
    FileVersion, FilesystemAuthority, InferenceEffort, InferenceOptions, LanguageObservationId,
    ModelId, ModelTarget, NetworkAuthority, ObjectiveCriterion, ObjectiveCriterionEvidence,
    ObjectiveId, ObjectiveOrigin, ObjectiveRecord, ObjectiveState, ObjectiveTransition,
    ObjectiveTransitionCause, OrchestrationFailureDecision, OrchestrationFailureDecisionRecord,
    OrchestrationNodeId, PlanId, ProviderId, RepositoryAuthority, RoutingProfileId, SessionId,
    SessionState, SessionSummary, ToolCallId, WorkspaceId,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde_json::{Map, Number, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::path::{Path, PathBuf};

const DATABASE_SCHEMA_VERSION: i64 = 11;

#[derive(Debug)]
pub enum PersistenceError {
    Io(std::io::Error),
    Sql(rusqlite::Error),
    InvalidJournal(String),
}

impl Display for PersistenceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "persistence I/O error: {error}"),
            Self::Sql(error) => write!(f, "SQLite persistence error: {error}"),
            Self::InvalidJournal(message) => write!(f, "invalid runtime journal: {message}"),
        }
    }
}

impl Error for PersistenceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Sql(error) => Some(error),
            Self::InvalidJournal(_) => None,
        }
    }
}

impl From<std::io::Error> for PersistenceError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<rusqlite::Error> for PersistenceError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sql(value)
    }
}

#[derive(Clone, Debug)]
pub struct SqliteStore {
    path: PathBuf,
}

impl SqliteStore {
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn open(&self) -> Result<Connection, PersistenceError> {
        if let Some(parent) = self
            .path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)?;
        }
        let mut connection = Connection::open(&self.path)?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        migrate(&mut connection)?;
        Ok(connection)
    }

    pub fn save(&self, journal: &RuntimeJournal) -> Result<(), PersistenceError> {
        journal
            .validate_structure()
            .map_err(|error| invalid(error.to_string()))?;
        let mut connection = self.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        initialize_or_validate_database(&transaction, journal)?;

        let stored = load_entries(&transaction)?;
        if stored.len() > journal.entries.len() {
            return Err(invalid(format!(
                "database contains {} events but incoming journal contains {}",
                stored.len(),
                journal.entries.len()
            )));
        }
        for (stored, incoming) in stored.iter().zip(&journal.entries) {
            if stored != incoming {
                return Err(invalid(format!(
                    "database event {} does not match the runtime journal",
                    stored.sequence
                )));
            }
        }

        for entry in journal.entries.iter().skip(stored.len()) {
            let sequence = sql_u64(entry.sequence, "journal sequence")?;
            transaction.execute(
                "INSERT INTO domain_events(sequence, event_type) VALUES (?1, ?2)",
                params![sequence, event_type(&entry.event)],
            )?;
            insert_event(&transaction, entry.sequence, &entry.event)?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn load(&self) -> Result<RuntimeJournal, PersistenceError> {
        if !self.path.exists() {
            return Err(PersistenceError::Io(std::io::Error::from(
                std::io::ErrorKind::NotFound,
            )));
        }
        let connection = self.open()?;
        let format_version = metadata(&connection, "journal_format_version")?
            .ok_or_else(|| invalid("database is uninitialized"))?
            .parse::<u64>()
            .map_err(|_| invalid("database contains an invalid journal format version"))?;
        let config_revision = parse_id(
            metadata(&connection, "initial_config_revision")?
                .ok_or_else(|| invalid("missing initial config revision"))?,
            "initial config revision",
            phenix_core::ConfigRevisionId::parse,
        )?;
        let config_fingerprint = ConfigRevisionFingerprint(
            metadata(&connection, "initial_config_fingerprint")?
                .ok_or_else(|| invalid("missing initial config fingerprint"))?,
        );
        let journal = RuntimeJournal {
            format_version,
            config_revision,
            config_fingerprint,
            entries: load_entries(&connection)?,
        };
        journal
            .validate_structure()
            .map_err(|error| invalid(error.to_string()))?;
        Ok(journal)
    }
}

#[derive(Default)]
struct FrontendColumns {
    text: Option<String>,
    state: Option<String>,
    termination_kind: Option<String>,
    termination_execution_id: Option<String>,
    tool_call_id: Option<String>,
    callable_id: Option<String>,
    output: Option<String>,
    success: Option<i64>,
    child_execution_id: Option<String>,
    decision_parent_execution_id: Option<String>,
    decision_failed_child_execution_id: Option<String>,
    decision_decider_execution_id: Option<String>,
    decision_kind: Option<String>,
    decision_recovery_execution_id: Option<String>,
    error_code: Option<String>,
    error_message: Option<String>,
}

#[derive(Clone)]
struct StoredValueNode {
    id: i64,
    parent: Option<i64>,
    object_key: Option<String>,
    array_index: Option<i64>,
    kind: String,
    scalar: Option<String>,
}

impl ConductorRuntime {
    #[must_use]
    pub fn journal(&self) -> &RuntimeJournal {
        &self.journal
    }

    pub fn restore(journal: RuntimeJournal) -> Result<Self, PersistenceError> {
        journal
            .validate_structure()
            .map_err(|error| invalid(error.to_string()))?;

        let config_revision = journal.config_revision.clone();
        let config_fingerprint = journal.config_fingerprint.clone();
        let mut runtime = Self::new();
        runtime.config_revision = config_revision.clone();
        runtime.config_revisions.clear();
        runtime.config_revisions.insert(
            config_revision.clone(),
            ConfigRevisionSlot {
                fingerprint: config_fingerprint.clone(),
                configuration: None,
                ordinal: 1,
            },
        );
        runtime.next_config_revision = 1;
        runtime.journal = RuntimeJournal::new(config_revision, config_fingerprint);

        for entry in &journal.entries {
            let mut projection = DurableProjection {
                config_revisions: &mut runtime.config_revisions,
                current_config_revision: &mut runtime.config_revision,
                sessions: &mut runtime.sessions,
                executions: &mut runtime.executions,
                root_ingress: &mut runtime.root_ingress,
                next_root_ingress: &mut runtime.next_root_ingress,
                attempt_groups: &mut runtime.attempt_groups,
                orchestration_decisions: &mut runtime.orchestration_decisions,
                orchestration_interfaces: &mut runtime.orchestration_interfaces,
                orchestration_nodes: &mut runtime.orchestration_nodes,
                orchestration_node_inputs: &mut runtime.orchestration_node_inputs,
                orchestration_synthesis: &mut runtime.orchestration_synthesis,
                execution_outputs: &mut runtime.execution_outputs,
                diagnostic_write_patches: &mut runtime.diagnostic_write_patches,
                resolved_routes: &mut runtime.resolved_routes,
                read_sets: &mut runtime.read_sets,
                events: &mut runtime.events,
                next_config_revision: &mut runtime.next_config_revision,
                next_session: &mut runtime.next_session,
                next_execution: &mut runtime.next_execution,
                next_attempt_group: &mut runtime.next_attempt_group,
                next_event: &mut runtime.next_event,
                next_tool_call: &mut runtime.next_tool_call,
            };
            apply_domain_event(&mut projection, &entry.event)
                .map_err(|error| invalid(error.to_string()))?;
        }

        if runtime.executions.values().any(|execution| {
            execution.summary.parent_execution.is_none()
                && !runtime.root_ingress.contains_key(&execution.summary.id)
        }) {
            return Err(invalid("root execution is missing durable ingress order"));
        }
        if let Some(workspace_id) = runtime
            .sessions
            .values()
            .next()
            .map(|session| session.summary.workspace_id.clone())
        {
            runtime.workspace_id = workspace_id;
        }
        runtime.journal = journal;
        Ok(runtime)
    }
}

include!("relational/authority.rs");
include!("relational/context.rs");
include!("relational/events.rs");
include!("relational/objectives.rs");
include!("relational/read_events.rs");
include!("relational/read_events_2.rs");
include!("relational/read_events_3.rs");
include!("relational/read_events_4.rs");
include!("relational/read_events_5.rs");
include!("relational/schema.rs");
include!("relational/sql.rs");
include!("relational/targets.rs");
include!("relational/tokens.rs");
include!("relational/values.rs");
include!("relational/write_events.rs");
include!("relational/write_events_2.rs");
include!("relational/write_events_3.rs");

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixed_target(model: &str) -> ExecutionTarget {
        ExecutionTarget::Fixed(ModelTarget {
            backend: BackendId::parse("mock").unwrap(),
            provider: ProviderId::parse("mock").unwrap(),
            model: ModelId::parse(model).unwrap(),
            inference: InferenceOptions {
                effort: Some(InferenceEffort::High),
            },
        })
    }

    fn temporary_store(label: &str) -> (PathBuf, SqliteStore) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "phenix-relational-{label}-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("runtime.sqlite3");
        (directory, SqliteStore::new(path))
    }

    fn representative_journal() -> RuntimeJournal {
        let mut runtime = ConductorRuntime::new();
        let first_target = fixed_target("first");
        let session = runtime
            .create_session(None, Some("initial".to_owned()), first_target.clone())
            .unwrap();
        runtime
            .record_domain_event(DomainEvent::SessionRenamed {
                session_id: session.id.clone(),
                name: "renamed".to_owned(),
            })
            .unwrap();
        runtime
            .record_domain_event(DomainEvent::SessionTargetChanged {
                session_id: session.id.clone(),
                target: fixed_target("second"),
            })
            .unwrap();
        let execution = runtime.submit(&session.id, "persist this").unwrap();
        runtime
            .set_state(&execution.id, ExecutionState::Running)
            .unwrap();
        runtime.resolve_invocation(&execution.id).unwrap();
        runtime
            .record_file_observation(
                &execution.id,
                phenix_core::FileObservationInput {
                    path: PathBuf::from("src/lib.rs"),
                    version: FileVersion::Present {
                        content_hash: "first-hash".to_owned(),
                        kind: FileKind::Regular,
                    },
                },
            )
            .unwrap();
        runtime
            .record_file_observation(
                &execution.id,
                phenix_core::FileObservationInput {
                    path: PathBuf::from("src/lib.rs"),
                    version: FileVersion::Present {
                        content_hash: "later-hash".to_owned(),
                        kind: FileKind::Regular,
                    },
                },
            )
            .unwrap();
        runtime
            .record_domain_event(DomainEvent::DiagnosticWritePatchCaptured {
                patch: DiagnosticWritePatch {
                    execution_id: execution.id.clone(),
                    path: PathBuf::from("src/lib.rs"),
                    patch: "@@ -1 +1 @@\n-old\n+new\n".to_owned(),
                },
            })
            .unwrap();
        runtime
            .record_language_observation(phenix_core::LanguageObservationInput {
                execution: execution.id.clone(),
                workspace: session.workspace_id.clone(),
                service: phenix_core::LanguageServiceKind::parse("rust").unwrap(),
                provider: phenix_core::LanguageProviderId::parse("rust-analyzer").unwrap(),
                provider_epoch: 3,
                operation: phenix_core::LanguageOperation::Definition {
                    document: PathBuf::from("src/lib.rs"),
                    position: phenix_core::LanguagePosition {
                        line: 4,
                        character: 2,
                    },
                },
                result: phenix_core::LanguageOperationResult {
                    value: serde_json::json!({"locations": []}),
                    documents: vec![phenix_core::LanguageDocumentIdentity {
                        path: PathBuf::from("src/lib.rs"),
                        workspace_version: Some(FileVersion::Present {
                            content_hash: "later-hash".to_owned(),
                            kind: FileKind::Regular,
                        }),
                        provenance: phenix_core::LanguageDocumentProvenance::WorkspaceBacked,
                    }],
                },
            })
            .unwrap();
        runtime
            .record_execution_output(
                &execution.id,
                serde_json::json!({
                    "ok": true,
                    "nested": [null, 7, 1.25, "value", {"key": false}]
                }),
            )
            .unwrap();
        runtime
            .set_state(&execution.id, ExecutionState::Completed)
            .unwrap();
        runtime.close_session(&session.id).unwrap();
        runtime.journal().clone()
    }

    include!("relational/tests/context.rs");
    include!("relational/tests/storage.rs");
}
