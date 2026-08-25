#[path = "plan_relational.rs"]
mod plan_relational;

use crate::{
    journal::{apply_domain_event, DurableProjection},
    ConductorRuntime, ConfigRevisionFingerprint, ConfigRevisionSlot, DomainEvent,
    JournalExecutionPayload, ResolvedRoute, RuntimeJournal,
};
use phenix_core::{
    AttemptGroup, AttemptGroupId, BackendId, CallableId, ContextInjection,
    ContextInjectionLifetime, ContextInjectionRequester, ContextResourceId, ContextRevision,
    DiagnosticWritePatch, ExactReference, ExecutionAuthority, ExecutionEvent, ExecutionEventKind,
    ExecutionId, ExecutionKind, ExecutionObjectiveAssignment, ExecutionState, ExecutionSummary,
    ExecutionTarget, ExecutionTerminationCause, FailureAttemptSummary, FileKind, FileObservation,
    FileObservationId, FileVersion, FilesystemAuthority, InferenceEffort, InferenceOptions,
    LanguageObservationId, ModelId, ModelTarget, NetworkAuthority, ObjectiveCriterion,
    ObjectiveCriterionEvidence, ObjectiveId, ObjectiveOrigin, ObjectiveRecord, ObjectiveState,
    ObjectiveTransition, ObjectiveTransitionCause, OrchestrationFailureDecision,
    OrchestrationFailureDecisionRecord, OrchestrationNodeId, PlanId, ProviderId,
    RepositoryAuthority, RoutingProfileId, SessionId, SessionState, SessionSummary, ToolCallId,
    WorkspaceId,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde_json::{Map, Number, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::path::{Path, PathBuf};

const DATABASE_SCHEMA_VERSION: i64 = 7;

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

fn event_type(event: &DomainEvent) -> &'static str {
    match event {
        DomainEvent::ConfigurationRevisionActivated { .. } => "configuration_revision_activated",
        DomainEvent::SessionCreated { .. } => "session_created",
        DomainEvent::SessionConfigRebased { .. } => "session_config_rebased",
        DomainEvent::SessionRenamed { .. } => "session_renamed",
        DomainEvent::SessionTargetChanged { .. } => "session_target_changed",
        DomainEvent::SessionClosed { .. } => "session_closed",
        DomainEvent::ExecutionCreated { .. } => "execution_created",
        DomainEvent::RootSubmissionAccepted { .. } => "root_submission_accepted",
        DomainEvent::ExecutionStateChanged { .. } => "execution_state_changed",
        DomainEvent::AttemptGroupCreated { .. } => "attempt_group_created",
        DomainEvent::AttemptFailureRecorded { .. } => "attempt_failure_recorded",
        DomainEvent::AttemptRetryStarted { .. } => "attempt_retry_started",
        DomainEvent::OrchestrationFailureInterfaceStarted { .. } => {
            "orchestration_failure_interface_started"
        }
        DomainEvent::OrchestrationDecisionMade { .. } => "orchestration_decision_made",
        DomainEvent::OrchestrationNodeStarted { .. } => "orchestration_node_started",
        DomainEvent::OrchestrationNodeInputBound { .. } => "orchestration_node_input_bound",
        DomainEvent::OrchestrationSynthesisStarted { .. } => "orchestration_synthesis_started",
        DomainEvent::ExecutionOutputRecorded { .. } => "execution_output_recorded",
        DomainEvent::DiagnosticWritePatchCaptured { .. } => "diagnostic_write_patch_captured",
        DomainEvent::LanguageObservationRecorded { .. } => "language_observation_recorded",
        DomainEvent::ContextInjectionRecorded { .. } => "context_injection_recorded",
        DomainEvent::ObjectiveSemanticsActivated => "objective_semantics_activated",
        DomainEvent::ObjectiveCreated { .. } => "objective_created",
        DomainEvent::ObjectiveDraftRevised { .. } => "objective_draft_revised",
        DomainEvent::ObjectiveEvidenceRecorded { .. } => "objective_evidence_recorded",
        DomainEvent::ObjectiveStateChanged { .. } => "objective_state_changed",
        DomainEvent::ExecutionObjectivesAssigned { .. } => "execution_objectives_assigned",
        DomainEvent::PlanCreated { .. } => "plan_created",
        DomainEvent::PlanDraftRevised { .. } => "plan_draft_revised",
        DomainEvent::PlanStateChanged { .. } => "plan_state_changed",
        DomainEvent::PlanStepStateChanged { .. } => "plan_step_state_changed",
        DomainEvent::ExecutionPlanAssigned { .. } => "execution_plan_assigned",
        DomainEvent::InvocationResolved { .. } => "invocation_resolved",
        DomainEvent::WorkspaceCheckpointCaptured { .. } => "workspace_checkpoint_captured",
        DomainEvent::WorkspaceFileObserved { .. } => "workspace_file_observed",
        DomainEvent::FrontendEvent { .. } => "frontend_event",
    }
}

fn insert_event(
    transaction: &Transaction<'_>,
    sequence: u64,
    event: &DomainEvent,
) -> Result<(), PersistenceError> {
    let sequence = sql_u64(sequence, "journal sequence")?;
    match event {
        DomainEvent::ConfigurationRevisionActivated {
            revision,
            fingerprint,
        } => {
            transaction.execute(
                "INSERT INTO configuration_revisions(revision_id, fingerprint, activated_sequence)
                 VALUES (?1, ?2, ?3)",
                params![revision.to_string(), fingerprint.to_string(), sequence],
            )?;
        }
        DomainEvent::SessionCreated { session } => {
            let target = insert_target(transaction, &session.default_target)?;
            transaction.execute(
                "INSERT INTO sessions(
                     session_id, parent_session_id, workspace_id, config_revision_id, name,
                     default_target_id, state, created_sequence
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    session.id.to_string(),
                    session.parent_session.as_ref().map(ToString::to_string),
                    session.workspace_id.to_string(),
                    session.config_revision.to_string(),
                    session.name.as_deref(),
                    target,
                    session_state_token(&session.state),
                    sequence,
                ],
            )?;
        }
        DomainEvent::SessionConfigRebased {
            session_id,
            config_revision,
        } => {
            transaction.execute(
                "INSERT INTO session_config_rebases(sequence, session_id, config_revision_id)
                 VALUES (?1, ?2, ?3)",
                params![
                    sequence,
                    session_id.to_string(),
                    config_revision.to_string()
                ],
            )?;
        }
        DomainEvent::SessionRenamed { session_id, name } => {
            transaction.execute(
                "INSERT INTO session_renames(sequence, session_id, name) VALUES (?1, ?2, ?3)",
                params![sequence, session_id.to_string(), name],
            )?;
        }
        DomainEvent::SessionTargetChanged { session_id, target } => {
            let target = insert_target(transaction, target)?;
            transaction.execute(
                "INSERT INTO session_target_changes(sequence, session_id, target_id)
                 VALUES (?1, ?2, ?3)",
                params![sequence, session_id.to_string(), target],
            )?;
        }
        DomainEvent::SessionClosed { session_id } => {
            transaction.execute(
                "INSERT INTO session_closures(sequence, session_id) VALUES (?1, ?2)",
                params![sequence, session_id.to_string()],
            )?;
        }
        DomainEvent::ExecutionCreated { execution, payload } => {
            insert_execution(transaction, sequence, execution, payload)?;
        }
        DomainEvent::RootSubmissionAccepted {
            session_id,
            execution_id,
            ingress_order,
        } => {
            transaction.execute(
                "INSERT INTO accepted_root_submissions(
                     session_id, ingress_order, execution_id, accepted_sequence
                 ) VALUES (?1, ?2, ?3, ?4)",
                params![
                    session_id.to_string(),
                    sql_u64(*ingress_order, "root ingress order")?,
                    execution_id.to_string(),
                    sequence,
                ],
            )?;
        }
        DomainEvent::ExecutionStateChanged {
            execution_id,
            state,
        } => {
            transaction.execute(
                "INSERT INTO execution_state_changes(sequence, execution_id, state)
                 VALUES (?1, ?2, ?3)",
                params![
                    sequence,
                    execution_id.to_string(),
                    execution_state_token(state)
                ],
            )?;
        }
        DomainEvent::AttemptGroupCreated { group } => {
            transaction.execute(
                "INSERT INTO attempt_groups(
                     attempt_group_id, parent_execution_id, callable_id, invariant_goal,
                     created_sequence
                 ) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    group.id.to_string(),
                    group.parent_execution.to_string(),
                    group.callable.to_string(),
                    group.goal.as_str(),
                    sequence,
                ],
            )?;
            for (index, execution_id) in group.attempts.iter().enumerate() {
                transaction.execute(
                    "INSERT INTO attempt_executions(
                         attempt_group_id, attempt_number, execution_id, started_sequence
                     ) VALUES (?1, ?2, ?3, ?4)",
                    params![
                        group.id.to_string(),
                        sql_usize(index + 1, "attempt number")?,
                        execution_id.to_string(),
                        sequence,
                    ],
                )?;
            }
            for failure in &group.failures {
                insert_attempt_failure(transaction, sequence, &group.id, failure)?;
            }
        }
        DomainEvent::AttemptFailureRecorded { group_id, failure } => {
            insert_attempt_failure(transaction, sequence, group_id, failure)?;
        }
        DomainEvent::AttemptRetryStarted {
            group_id,
            execution_id,
        } => {
            let attempt = transaction.query_row(
                "SELECT COALESCE(MAX(attempt_number), 0) + 1
                 FROM attempt_executions WHERE attempt_group_id = ?1",
                params![group_id.to_string()],
                |row| row.get::<_, i64>(0),
            )?;
            transaction.execute(
                "INSERT INTO attempt_executions(
                     attempt_group_id, attempt_number, execution_id, started_sequence
                 ) VALUES (?1, ?2, ?3, ?4)",
                params![
                    group_id.to_string(),
                    attempt,
                    execution_id.to_string(),
                    sequence
                ],
            )?;
        }
        DomainEvent::OrchestrationFailureInterfaceStarted {
            parent_execution,
            failed_child,
            interface_execution,
        } => {
            transaction.execute(
                "INSERT INTO orchestration_failure_interfaces(
                     failed_child_execution_id, parent_execution_id, interface_execution_id,
                     started_sequence
                 ) VALUES (?1, ?2, ?3, ?4)",
                params![
                    failed_child.to_string(),
                    parent_execution.to_string(),
                    interface_execution.to_string(),
                    sequence,
                ],
            )?;
        }
        DomainEvent::OrchestrationDecisionMade { decision } => {
            let (kind, recovery) = decision_columns(&decision.decision);
            transaction.execute(
                "INSERT INTO parent_failure_decisions(
                     failed_child_execution_id, parent_execution_id, decider_execution_id,
                     decision_kind, recovery_execution_id, decided_sequence
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    decision.failed_child.to_string(),
                    decision.parent_execution.to_string(),
                    decision.decider_execution.as_ref().map(ToString::to_string),
                    kind,
                    recovery,
                    sequence,
                ],
            )?;
        }
        DomainEvent::OrchestrationNodeStarted {
            execution_id,
            node_id,
            child_execution_id,
        } => {
            transaction.execute(
                "INSERT INTO orchestration_node_bindings(
                     orchestration_execution_id, node_id, child_execution_id, bound_sequence
                 ) VALUES (?1, ?2, ?3, ?4)",
                params![
                    execution_id.to_string(),
                    node_id.to_string(),
                    child_execution_id.to_string(),
                    sequence,
                ],
            )?;
        }
        DomainEvent::OrchestrationNodeInputBound {
            execution_id,
            node_id,
            input,
        } => {
            let input = insert_value(transaction, input)?;
            transaction.execute(
                "INSERT INTO orchestration_node_inputs(
                     orchestration_execution_id, node_id, input_value_id, bound_sequence
                 ) VALUES (?1, ?2, ?3, ?4)",
                params![
                    execution_id.to_string(),
                    node_id.to_string(),
                    input,
                    sequence
                ],
            )?;
        }
        DomainEvent::OrchestrationSynthesisStarted {
            execution_id,
            interface_execution_id,
        } => {
            transaction.execute(
                "INSERT INTO orchestration_synthesis(
                     orchestration_execution_id, interface_execution_id, started_sequence
                 ) VALUES (?1, ?2, ?3)",
                params![
                    execution_id.to_string(),
                    interface_execution_id.to_string(),
                    sequence,
                ],
            )?;
        }
        DomainEvent::ExecutionOutputRecorded {
            execution_id,
            output,
        } => {
            let output = insert_value(transaction, output)?;
            transaction.execute(
                "INSERT INTO execution_outputs(
                     execution_id, output_value_id, recorded_sequence
                 ) VALUES (?1, ?2, ?3)",
                params![execution_id.to_string(), output, sequence],
            )?;
        }
        DomainEvent::DiagnosticWritePatchCaptured { patch } => {
            transaction.execute(
                "INSERT INTO diagnostic_write_patches(
                     execution_id, path, patch, captured_sequence
                 ) VALUES (?1, ?2, ?3, ?4)",
                params![
                    patch.execution_id.to_string(),
                    patch.path.to_string_lossy(),
                    patch.patch.as_str(),
                    sequence,
                ],
            )?;
        }
        DomainEvent::LanguageObservationRecorded { observation } => {
            let operation_value = serde_json::to_value(&observation.operation)
                .map_err(|error| invalid(format!("cannot encode language operation: {error}")))?;
            let result_value = serde_json::to_value(&observation.result)
                .map_err(|error| invalid(format!("cannot encode language result: {error}")))?;
            let operation = insert_value(transaction, &operation_value)?;
            let result = insert_value(transaction, &result_value)?;
            transaction.execute(
                "INSERT INTO language_observations(
                     recorded_sequence, execution_id, workspace_id, service_kind, provider_id,
                     provider_epoch, operation_value_id, result_value_id
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    sequence,
                    observation.execution.to_string(),
                    observation.workspace.to_string(),
                    observation.service.to_string(),
                    observation.provider.to_string(),
                    sql_u64(observation.provider_epoch, "language provider epoch")?,
                    operation,
                    result,
                ],
            )?;
        }
        DomainEvent::ContextInjectionRecorded { injection } => {
            if let ExactReference::Context { revision, .. } = &injection.source_ref {
                if revision != &injection.source_revision {
                    return Err(invalid(
                        "context source reference revision does not match injection source revision",
                    ));
                }
            }
            let (source_kind, source_id, source_event_sequence) =
                context_source_columns(&injection.source_ref)?;
            transaction.execute(
                "INSERT INTO context_injections(
                     recorded_sequence, execution_id, source_kind, source_id, source_event_sequence,
                     source_revision, requester, reason, lifetime, content_identity
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    sequence,
                    injection.execution_id.to_string(),
                    source_kind,
                    source_id,
                    source_event_sequence,
                    injection.source_revision.to_string(),
                    context_requester_token(&injection.requested_by),
                    injection.reason.as_str(),
                    context_lifetime_token(&injection.lifetime),
                    injection.content_identity.to_string(),
                ],
            )?;
        }
        DomainEvent::ObjectiveSemanticsActivated => {}
        DomainEvent::ObjectiveCreated { objective } => {
            let (origin, parent) = objective_origin_columns(&objective.origin);
            transaction.execute(
                "INSERT INTO objective_creations(
                     created_sequence, objective_id, workspace_id, origin_kind, parent_objective_id,
                     statement, state, supersedes_objective_id
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    sequence,
                    objective.id.to_string(),
                    objective.workspace.to_string(),
                    origin,
                    parent,
                    objective.statement,
                    objective_state_token(&objective.state),
                    objective.supersedes.as_ref().map(ToString::to_string),
                ],
            )?;
            insert_objective_criteria(
                transaction,
                "objective_creation_criteria",
                "created_sequence",
                sequence,
                &objective.criteria,
            )?;
        }
        DomainEvent::ObjectiveDraftRevised { objective } => {
            transaction.execute(
                "INSERT INTO objective_draft_revisions(sequence, objective_id, statement)
                 VALUES (?1, ?2, ?3)",
                params![sequence, objective.id.to_string(), objective.statement],
            )?;
            insert_objective_criteria(
                transaction,
                "objective_draft_revision_criteria",
                "sequence",
                sequence,
                &objective.criteria,
            )?;
        }
        DomainEvent::ObjectiveEvidenceRecorded {
            objective_id,
            evidence,
        } => {
            transaction.execute(
                "INSERT INTO objective_evidence(sequence, objective_id, criterion_id, evidence_ref)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    sequence,
                    objective_id.to_string(),
                    evidence.criterion_id.to_string(),
                    evidence.evidence_ref,
                ],
            )?;
        }
        DomainEvent::ObjectiveStateChanged { transition } => {
            let (kind, execution, detail) = objective_cause_columns(&transition.cause);
            transaction.execute(
                "INSERT INTO objective_state_changes(
                     sequence, objective_id, from_state, to_state, cause_kind,
                     cause_execution_id, cause_detail
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    sequence,
                    transition.objective_id.to_string(),
                    objective_state_token(&transition.from),
                    objective_state_token(&transition.to),
                    kind,
                    execution,
                    detail,
                ],
            )?;
        }
        DomainEvent::ExecutionObjectivesAssigned { assignment } => {
            transaction.execute(
                "INSERT INTO execution_objective_assignments(
                     sequence, execution_id, primary_objective_id
                 ) VALUES (?1, ?2, ?3)",
                params![
                    sequence,
                    assignment.execution_id.to_string(),
                    assignment.primary.to_string(),
                ],
            )?;
            for objective in &assignment.supporting {
                transaction.execute(
                    "INSERT INTO execution_supporting_objectives(sequence, objective_id)
                     VALUES (?1, ?2)",
                    params![sequence, objective.to_string()],
                )?;
            }
        }
        event @ (DomainEvent::PlanCreated { .. }
        | DomainEvent::PlanDraftRevised { .. }
        | DomainEvent::PlanStateChanged { .. }
        | DomainEvent::PlanStepStateChanged { .. }
        | DomainEvent::ExecutionPlanAssigned { .. }) => {
            plan_relational::insert_event(transaction, sequence, event)?;
        }
        DomainEvent::InvocationResolved {
            execution_id,
            route,
        } => {
            let requested = insert_target(transaction, &route.requested_target)?;
            let model = insert_target(transaction, &ExecutionTarget::Fixed(route.model.clone()))?;
            transaction.execute(
                "INSERT INTO resolved_routing(
                     execution_id, requested_target_id, model_target_id, config_revision_id,
                     resolved_sequence
                 ) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    execution_id.to_string(),
                    requested,
                    model,
                    route.config_revision.to_string(),
                    sequence,
                ],
            )?;
        }
        DomainEvent::WorkspaceCheckpointCaptured {
            execution_id,
            workspace_id,
            files,
        } => {
            transaction.execute(
                "INSERT INTO workspace_checkpoints(
                     checkpoint_sequence, execution_id, workspace_id
                 ) VALUES (?1, ?2, ?3)",
                params![sequence, execution_id.to_string(), workspace_id.to_string()],
            )?;
            for (path, version) in files {
                let (state, hash, kind) = file_version_columns(version);
                transaction.execute(
                    "INSERT INTO workspace_checkpoint_files(
                         checkpoint_sequence, path, version_state, content_hash, file_kind
                     ) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![sequence, path.to_string_lossy(), state, hash, kind],
                )?;
            }
        }
        DomainEvent::WorkspaceFileObserved {
            execution_id,
            observation,
        } => {
            let (state, hash, kind) = file_version_columns(&observation.version);
            transaction.execute(
                "INSERT INTO workspace_observation_events(
                     execution_id, path, version_state, content_hash, file_kind,
                     observed_sequence
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    execution_id.to_string(),
                    observation.path.to_string_lossy(),
                    state,
                    hash,
                    kind,
                    sequence,
                ],
            )?;
        }
        DomainEvent::FrontendEvent { event } => {
            insert_frontend_event(transaction, sequence, event)?
        }
    }
    Ok(())
}

fn insert_execution(
    transaction: &Transaction<'_>,
    sequence: i64,
    execution: &ExecutionSummary,
    payload: &JournalExecutionPayload,
) -> Result<(), PersistenceError> {
    let config_revision = execution_config_revision(transaction, execution, sequence)?;
    let target = insert_target(transaction, &execution.target)?;
    let (payload_kind, input_text, input_value) = match payload {
        JournalExecutionPayload::Invocation { input, .. } => {
            ("invocation", Some(input.as_str()), None)
        }
        JournalExecutionPayload::Orchestration { input, .. } => (
            "orchestration",
            None,
            Some(insert_value(transaction, input)?),
        ),
    };
    let authority = payload.authority();
    transaction.execute(
        "INSERT INTO executions(
             execution_id, session_id, parent_execution_id, kind, callable_id, target_id,
             state, config_revision_id, payload_kind, input_text, input_value_id,
             authority_filesystem, authority_network, authority_repository, created_sequence
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        params![
            execution.id.to_string(),
            execution.session_id.to_string(),
            execution.parent_execution.as_ref().map(ToString::to_string),
            execution_kind_token(&execution.kind),
            execution.callable.as_ref().map(ToString::to_string),
            target,
            execution_state_token(&execution.state),
            config_revision,
            payload_kind,
            input_text,
            input_value,
            filesystem_token(authority.filesystem),
            network_token(authority.network),
            repository_token(authority.repository),
            sequence,
        ],
    )?;
    for endpoint in &authority.ipc {
        transaction.execute(
            "INSERT INTO execution_authority_ipc(execution_id, endpoint) VALUES (?1, ?2)",
            params![execution.id.to_string(), endpoint],
        )?;
    }
    for secret in &authority.secrets {
        transaction.execute(
            "INSERT INTO execution_authority_secrets(execution_id, secret_name) VALUES (?1, ?2)",
            params![execution.id.to_string(), secret],
        )?;
    }
    for callable in &authority.callables {
        transaction.execute(
            "INSERT INTO execution_authority_callables(execution_id, callable_id) VALUES (?1, ?2)",
            params![execution.id.to_string(), callable.to_string()],
        )?;
    }
    Ok(())
}

fn execution_config_revision(
    transaction: &Transaction<'_>,
    execution: &ExecutionSummary,
    sequence: i64,
) -> Result<String, PersistenceError> {
    if let Some(parent) = execution.parent_execution.as_ref() {
        return Ok(transaction.query_row(
            "SELECT config_revision_id FROM executions WHERE execution_id = ?1",
            params![parent.to_string()],
            |row| row.get(0),
        )?);
    }
    let rebased = transaction
        .query_row(
            "SELECT config_revision_id FROM session_config_rebases
             WHERE session_id = ?1 AND sequence < ?2 ORDER BY sequence DESC LIMIT 1",
            params![execution.session_id.to_string(), sequence],
            |row| row.get(0),
        )
        .optional()?;
    match rebased {
        Some(revision) => Ok(revision),
        None => Ok(transaction.query_row(
            "SELECT config_revision_id FROM sessions WHERE session_id = ?1",
            params![execution.session_id.to_string()],
            |row| row.get(0),
        )?),
    }
}

fn insert_attempt_failure(
    transaction: &Transaction<'_>,
    sequence: i64,
    group_id: &AttemptGroupId,
    failure: &FailureAttemptSummary,
) -> Result<(), PersistenceError> {
    transaction.execute(
        "INSERT INTO attempt_failures(
             attempt_group_id, attempt_number, execution_id, approach, failure_at, reason,
             recorded_sequence
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            group_id.to_string(),
            i64::from(failure.attempt),
            failure.execution_id.to_string(),
            failure.approach.as_str(),
            failure.failure_at.as_str(),
            failure.reason.as_str(),
            sequence,
        ],
    )?;
    for (index, item) in failure.completed_work.iter().enumerate() {
        transaction.execute(
            "INSERT INTO attempt_completed_work(
                 attempt_group_id, attempt_number, item_order, item
             ) VALUES (?1, ?2, ?3, ?4)",
            params![
                group_id.to_string(),
                i64::from(failure.attempt),
                sql_usize(index, "completed-work item order")?,
                item,
            ],
        )?;
    }
    Ok(())
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

fn insert_frontend_event(
    transaction: &Transaction<'_>,
    journal_sequence: i64,
    event: &ExecutionEvent,
) -> Result<(), PersistenceError> {
    let mut columns = FrontendColumns::default();
    match &event.kind {
        ExecutionEventKind::UserInput { text }
        | ExecutionEventKind::AssistantContentDelta { text }
        | ExecutionEventKind::ReasoningDelta { text } => columns.text = Some(text.clone()),
        ExecutionEventKind::ExecutionStateChanged { state } => {
            columns.state = Some(execution_state_token(state).to_owned());
        }
        ExecutionEventKind::ExecutionTerminated { cause } => {
            let (kind, execution) = termination_columns(cause);
            columns.termination_kind = Some(kind.to_owned());
            columns.termination_execution_id = Some(execution.to_string());
        }
        ExecutionEventKind::ToolCallStarted {
            tool_call_id,
            callable,
        } => {
            columns.tool_call_id = Some(tool_call_id.to_string());
            columns.callable_id = Some(callable.to_string());
        }
        ExecutionEventKind::ToolCallArguments {
            tool_call_id,
            arguments,
        } => {
            columns.tool_call_id = Some(tool_call_id.to_string());
            columns.text = Some(arguments.clone());
        }
        ExecutionEventKind::ToolCallFinished {
            tool_call_id,
            output,
            success,
        } => {
            columns.tool_call_id = Some(tool_call_id.to_string());
            columns.output = Some(output.clone());
            columns.success = Some(i64::from(*success));
        }
        ExecutionEventKind::ChildExecutionStarted { child } => {
            columns.child_execution_id = Some(child.to_string());
        }
        ExecutionEventKind::ChildExecutionFinished { child, state } => {
            columns.child_execution_id = Some(child.to_string());
            columns.state = Some(execution_state_token(state).to_owned());
        }
        ExecutionEventKind::OrchestrationDecisionMade { decision } => {
            let (kind, recovery) = decision_columns(&decision.decision);
            columns.decision_parent_execution_id = Some(decision.parent_execution.to_string());
            columns.decision_failed_child_execution_id = Some(decision.failed_child.to_string());
            columns.decision_decider_execution_id =
                decision.decider_execution.as_ref().map(ToString::to_string);
            columns.decision_kind = Some(kind.to_owned());
            columns.decision_recovery_execution_id = recovery;
        }
        ExecutionEventKind::Error { code, message } => {
            columns.error_code = Some(code.clone());
            columns.error_message = Some(message.clone());
        }
    }
    transaction.execute(
        "INSERT INTO canonical_events(
             event_sequence, journal_sequence, session_id, execution_id, kind, text, state,
             termination_kind, termination_execution_id, tool_call_id, callable_id, output,
             success, child_execution_id, decision_parent_execution_id,
             decision_failed_child_execution_id, decision_decider_execution_id, decision_kind,
             decision_recovery_execution_id, error_code, error_message
         ) VALUES (
             ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
             ?17, ?18, ?19, ?20, ?21
         )",
        params![
            sql_u64(event.sequence, "frontend event sequence")?,
            journal_sequence,
            event.session_id.to_string(),
            event.execution_id.to_string(),
            execution_event_type(&event.kind),
            columns.text,
            columns.state,
            columns.termination_kind,
            columns.termination_execution_id,
            columns.tool_call_id,
            columns.callable_id,
            columns.output,
            columns.success,
            columns.child_execution_id,
            columns.decision_parent_execution_id,
            columns.decision_failed_child_execution_id,
            columns.decision_decider_execution_id,
            columns.decision_kind,
            columns.decision_recovery_execution_id,
            columns.error_code,
            columns.error_message,
        ],
    )?;
    Ok(())
}

fn insert_target(
    transaction: &Transaction<'_>,
    target: &ExecutionTarget,
) -> Result<i64, PersistenceError> {
    match target {
        ExecutionTarget::Fixed(model) => {
            transaction.execute(
                "INSERT INTO targets(
                     kind, backend_id, provider_id, model_id, inference_effort
                 ) VALUES ('fixed', ?1, ?2, ?3, ?4)",
                params![
                    model.backend.to_string(),
                    model.provider.to_string(),
                    model.model.to_string(),
                    model.inference.effort.as_ref().map(inference_effort_token),
                ],
            )?;
        }
        ExecutionTarget::Routed(profile) => {
            transaction.execute(
                "INSERT INTO targets(kind, routing_profile_id) VALUES ('routed', ?1)",
                params![profile.to_string()],
            )?;
        }
    }
    Ok(transaction.last_insert_rowid())
}

fn insert_value(transaction: &Transaction<'_>, value: &Value) -> Result<i64, PersistenceError> {
    transaction.execute("INSERT INTO structured_values DEFAULT VALUES", [])?;
    let value_id = transaction.last_insert_rowid();
    insert_value_node(transaction, value_id, None, None, None, value)?;
    Ok(value_id)
}

fn insert_value_node(
    transaction: &Transaction<'_>,
    value_id: i64,
    parent: Option<i64>,
    object_key: Option<&str>,
    array_index: Option<i64>,
    value: &Value,
) -> Result<i64, PersistenceError> {
    let (kind, scalar) = match value {
        Value::Null => ("null", None),
        Value::Bool(value) => ("boolean", Some(value.to_string())),
        Value::Number(value) => ("number", Some(value.to_string())),
        Value::String(value) => ("string", Some(value.clone())),
        Value::Array(_) => ("array", None),
        Value::Object(_) => ("object", None),
    };
    transaction.execute(
        "INSERT INTO structured_value_nodes(
             value_id, parent_node_id, object_key, array_index, kind, scalar
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![value_id, parent, object_key, array_index, kind, scalar],
    )?;
    let node = transaction.last_insert_rowid();
    match value {
        Value::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                insert_value_node(
                    transaction,
                    value_id,
                    Some(node),
                    None,
                    Some(sql_usize(index, "structured array index")?),
                    value,
                )?;
            }
        }
        Value::Object(values) => {
            for (key, value) in values {
                insert_value_node(transaction, value_id, Some(node), Some(key), None, value)?;
            }
        }
        _ => {}
    }
    Ok(node)
}

fn file_version_columns(
    version: &FileVersion,
) -> (&'static str, Option<&str>, Option<&'static str>) {
    match version {
        FileVersion::Absent => ("absent", None, None),
        FileVersion::Present { content_hash, kind } => {
            ("present", Some(content_hash), Some(file_kind_token(kind)))
        }
    }
}

fn decision_columns(decision: &OrchestrationFailureDecision) -> (&'static str, Option<String>) {
    match decision {
        OrchestrationFailureDecision::Retry { execution_id } => {
            ("retry", Some(execution_id.to_string()))
        }
        OrchestrationFailureDecision::ChooseAnotherChild { execution_id } => {
            ("choose_another_child", Some(execution_id.to_string()))
        }
        OrchestrationFailureDecision::Continue => ("continue", None),
        OrchestrationFailureDecision::Fail => ("fail", None),
    }
}

fn termination_columns(cause: &ExecutionTerminationCause) -> (&'static str, &ExecutionId) {
    match cause {
        ExecutionTerminationCause::ExplicitCancellation {
            requested_execution,
        } => ("explicit_cancellation", requested_execution),
        ExecutionTerminationCause::AncestorFailure { failed_ancestor } => {
            ("ancestor_failure", failed_ancestor)
        }
    }
}

fn context_source_columns(
    source: &ExactReference,
) -> Result<(&'static str, Option<String>, Option<i64>), PersistenceError> {
    match source {
        ExactReference::Objective(id) => Ok(("objective", Some(id.to_string()), None)),
        ExactReference::Plan(id) => Ok(("plan", Some(id.to_string()), None)),
        ExactReference::Execution(id) => Ok(("execution", Some(id.to_string()), None)),
        ExactReference::Event(sequence) => Ok((
            "event",
            None,
            Some(sql_u64(*sequence, "context source event sequence")?),
        )),
        ExactReference::FileObservation(id) => Ok(("file_observation", Some(id.to_string()), None)),
        ExactReference::LanguageObservation(id) => {
            Ok(("language_observation", Some(id.to_string()), None))
        }
        ExactReference::Context { resource_id, .. } => {
            Ok(("context", Some(resource_id.to_string()), None))
        }
    }
}

fn context_source_id(
    id: Option<String>,
    event_sequence: Option<i64>,
    kind: &str,
) -> Result<String, PersistenceError> {
    if event_sequence.is_some() {
        return Err(invalid(format!(
            "context {kind} source must not contain an event sequence"
        )));
    }
    required_column(id, "context source id")
}

fn parse_context_source(
    kind: &str,
    id: Option<String>,
    event_sequence: Option<i64>,
    source_revision: &ContextRevision,
) -> Result<ExactReference, PersistenceError> {
    match kind {
        "objective" => Ok(ExactReference::Objective(parse_id(
            context_source_id(id, event_sequence, kind)?,
            "context objective source",
            ObjectiveId::parse,
        )?)),
        "plan" => Ok(ExactReference::Plan(parse_id(
            context_source_id(id, event_sequence, kind)?,
            "context plan source",
            PlanId::parse,
        )?)),
        "execution" => Ok(ExactReference::Execution(parse_id(
            context_source_id(id, event_sequence, kind)?,
            "context execution source",
            ExecutionId::parse,
        )?)),
        "event" => {
            if id.is_some() {
                return Err(invalid("context event source must not contain a source id"));
            }
            Ok(ExactReference::Event(runtime_u64(
                event_sequence
                    .ok_or_else(|| invalid("context event source is missing its sequence"))?,
                "context source event sequence",
            )?))
        }
        "file_observation" => Ok(ExactReference::FileObservation(parse_id(
            context_source_id(id, event_sequence, kind)?,
            "context file observation source",
            FileObservationId::parse,
        )?)),
        "language_observation" => Ok(ExactReference::LanguageObservation(parse_id(
            context_source_id(id, event_sequence, kind)?,
            "context language observation source",
            LanguageObservationId::parse,
        )?)),
        "context" => Ok(ExactReference::Context {
            resource_id: parse_id(
                context_source_id(id, event_sequence, kind)?,
                "context resource source",
                ContextResourceId::parse,
            )?,
            revision: source_revision.clone(),
        }),
        other => Err(invalid(format!("unknown context source kind: {other}"))),
    }
}

fn context_requester_token(requester: &ContextInjectionRequester) -> &'static str {
    match requester {
        ContextInjectionRequester::Agent => "agent",
        ContextInjectionRequester::User => "user",
        ContextInjectionRequester::Orchestration => "orchestration",
        ContextInjectionRequester::ContextPolicy => "context_policy",
        ContextInjectionRequester::Hook => "hook",
        ContextInjectionRequester::Frontend => "frontend",
    }
}

fn parse_context_requester(value: &str) -> Result<ContextInjectionRequester, PersistenceError> {
    match value {
        "agent" => Ok(ContextInjectionRequester::Agent),
        "user" => Ok(ContextInjectionRequester::User),
        "orchestration" => Ok(ContextInjectionRequester::Orchestration),
        "context_policy" => Ok(ContextInjectionRequester::ContextPolicy),
        "hook" => Ok(ContextInjectionRequester::Hook),
        "frontend" => Ok(ContextInjectionRequester::Frontend),
        other => Err(invalid(format!(
            "unknown context injection requester: {other}"
        ))),
    }
}

fn context_lifetime_token(lifetime: &ContextInjectionLifetime) -> &'static str {
    match lifetime {
        ContextInjectionLifetime::SingleRequest => "single_request",
        ContextInjectionLifetime::Execution => "execution",
        ContextInjectionLifetime::Objective => "objective",
    }
}

fn parse_context_lifetime(value: &str) -> Result<ContextInjectionLifetime, PersistenceError> {
    match value {
        "single_request" => Ok(ContextInjectionLifetime::SingleRequest),
        "execution" => Ok(ContextInjectionLifetime::Execution),
        "objective" => Ok(ContextInjectionLifetime::Objective),
        other => Err(invalid(format!(
            "unknown context injection lifetime: {other}"
        ))),
    }
}

fn load_entries(connection: &Connection) -> Result<Vec<crate::JournalEntry>, PersistenceError> {
    let mut statement =
        connection.prepare("SELECT sequence, event_type FROM domain_events ORDER BY sequence")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut entries = Vec::new();
    for row in rows {
        let (sequence, event_type) = row?;
        entries.push(crate::JournalEntry {
            sequence: runtime_u64(sequence, "journal sequence")?,
            event: load_event(connection, sequence, &event_type)?,
        });
    }
    Ok(entries)
}

fn load_event(
    connection: &Connection,
    sequence: i64,
    event_type: &str,
) -> Result<DomainEvent, PersistenceError> {
    match event_type {
        "configuration_revision_activated" => {
            let (revision, fingerprint) = connection.query_row(
                "SELECT revision_id, fingerprint FROM configuration_revisions
                 WHERE activated_sequence = ?1",
                params![sequence],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )?;
            Ok(DomainEvent::ConfigurationRevisionActivated {
                revision: parse_id(
                    revision,
                    "configuration revision",
                    phenix_core::ConfigRevisionId::parse,
                )?,
                fingerprint: ConfigRevisionFingerprint(fingerprint),
            })
        }
        "session_created" => load_session_created(connection, sequence),
        "session_config_rebased" => {
            let (session, revision) = connection.query_row(
                "SELECT session_id, config_revision_id FROM session_config_rebases
                 WHERE sequence = ?1",
                params![sequence],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )?;
            Ok(DomainEvent::SessionConfigRebased {
                session_id: parse_id(session, "session", SessionId::parse)?,
                config_revision: parse_id(
                    revision,
                    "configuration revision",
                    phenix_core::ConfigRevisionId::parse,
                )?,
            })
        }
        "session_renamed" => {
            let (session, name) = connection.query_row(
                "SELECT session_id, name FROM session_renames WHERE sequence = ?1",
                params![sequence],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )?;
            Ok(DomainEvent::SessionRenamed {
                session_id: parse_id(session, "session", SessionId::parse)?,
                name,
            })
        }
        "session_target_changed" => {
            let (session, target) = connection.query_row(
                "SELECT session_id, target_id FROM session_target_changes WHERE sequence = ?1",
                params![sequence],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
            )?;
            Ok(DomainEvent::SessionTargetChanged {
                session_id: parse_id(session, "session", SessionId::parse)?,
                target: load_target(connection, target)?,
            })
        }
        "session_closed" => {
            let session = connection.query_row(
                "SELECT session_id FROM session_closures WHERE sequence = ?1",
                params![sequence],
                |row| row.get::<_, String>(0),
            )?;
            Ok(DomainEvent::SessionClosed {
                session_id: parse_id(session, "session", SessionId::parse)?,
            })
        }
        "execution_created" => load_execution_created(connection, sequence),
        "root_submission_accepted" => {
            let (session, execution, ingress) = connection.query_row(
                "SELECT session_id, execution_id, ingress_order
                 FROM accepted_root_submissions WHERE accepted_sequence = ?1",
                params![sequence],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                },
            )?;
            Ok(DomainEvent::RootSubmissionAccepted {
                session_id: parse_id(session, "session", SessionId::parse)?,
                execution_id: parse_id(execution, "execution", ExecutionId::parse)?,
                ingress_order: runtime_u64(ingress, "root ingress order")?,
            })
        }
        "execution_state_changed" => {
            let (execution, state) = connection.query_row(
                "SELECT execution_id, state FROM execution_state_changes WHERE sequence = ?1",
                params![sequence],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )?;
            Ok(DomainEvent::ExecutionStateChanged {
                execution_id: parse_id(execution, "execution", ExecutionId::parse)?,
                state: parse_execution_state(&state)?,
            })
        }
        "attempt_group_created" => load_attempt_group_created(connection, sequence),
        "attempt_failure_recorded" => {
            let (group, failure) = load_attempt_failure(connection, sequence)?;
            Ok(DomainEvent::AttemptFailureRecorded {
                group_id: group,
                failure,
            })
        }
        "attempt_retry_started" => {
            let (group, execution) = connection.query_row(
                "SELECT attempt_group_id, execution_id FROM attempt_executions
                 WHERE started_sequence = ?1",
                params![sequence],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )?;
            Ok(DomainEvent::AttemptRetryStarted {
                group_id: parse_id(group, "attempt group", AttemptGroupId::parse)?,
                execution_id: parse_id(execution, "execution", ExecutionId::parse)?,
            })
        }
        "orchestration_failure_interface_started" => {
            let (parent, failed, interface) = connection.query_row(
                "SELECT parent_execution_id, failed_child_execution_id, interface_execution_id
                 FROM orchestration_failure_interfaces WHERE started_sequence = ?1",
                params![sequence],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )?;
            Ok(DomainEvent::OrchestrationFailureInterfaceStarted {
                parent_execution: parse_id(parent, "execution", ExecutionId::parse)?,
                failed_child: parse_id(failed, "execution", ExecutionId::parse)?,
                interface_execution: parse_id(interface, "execution", ExecutionId::parse)?,
            })
        }
        "orchestration_decision_made" => Ok(DomainEvent::OrchestrationDecisionMade {
            decision: load_decision(connection, sequence)?,
        }),
        "orchestration_node_started" => {
            let (execution, node, child) = connection.query_row(
                "SELECT orchestration_execution_id, node_id, child_execution_id
                 FROM orchestration_node_bindings WHERE bound_sequence = ?1",
                params![sequence],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )?;
            Ok(DomainEvent::OrchestrationNodeStarted {
                execution_id: parse_id(execution, "execution", ExecutionId::parse)?,
                node_id: parse_id(node, "orchestration node", OrchestrationNodeId::parse)?,
                child_execution_id: parse_id(child, "execution", ExecutionId::parse)?,
            })
        }
        "orchestration_node_input_bound" => {
            let (execution, node, value) = connection.query_row(
                "SELECT orchestration_execution_id, node_id, input_value_id
                 FROM orchestration_node_inputs WHERE bound_sequence = ?1",
                params![sequence],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                },
            )?;
            Ok(DomainEvent::OrchestrationNodeInputBound {
                execution_id: parse_id(execution, "execution", ExecutionId::parse)?,
                node_id: parse_id(node, "orchestration node", OrchestrationNodeId::parse)?,
                input: load_value(connection, value)?,
            })
        }
        "orchestration_synthesis_started" => {
            let (execution, interface) = connection.query_row(
                "SELECT orchestration_execution_id, interface_execution_id
                 FROM orchestration_synthesis WHERE started_sequence = ?1",
                params![sequence],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )?;
            Ok(DomainEvent::OrchestrationSynthesisStarted {
                execution_id: parse_id(execution, "execution", ExecutionId::parse)?,
                interface_execution_id: parse_id(interface, "execution", ExecutionId::parse)?,
            })
        }
        "execution_output_recorded" => {
            let (execution, value) = connection.query_row(
                "SELECT execution_id, output_value_id FROM execution_outputs
                 WHERE recorded_sequence = ?1",
                params![sequence],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
            )?;
            Ok(DomainEvent::ExecutionOutputRecorded {
                execution_id: parse_id(execution, "execution", ExecutionId::parse)?,
                output: load_value(connection, value)?,
            })
        }
        "diagnostic_write_patch_captured" => {
            let (execution, path, patch) = connection.query_row(
                "SELECT execution_id, path, patch FROM diagnostic_write_patches
                 WHERE captured_sequence = ?1",
                params![sequence],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )?;
            Ok(DomainEvent::DiagnosticWritePatchCaptured {
                patch: DiagnosticWritePatch {
                    execution_id: parse_id(execution, "execution", ExecutionId::parse)?,
                    path: PathBuf::from(path),
                    patch,
                },
            })
        }
        "language_observation_recorded" => {
            let row = connection.query_row(
                "SELECT execution_id, workspace_id, service_kind, provider_id, provider_epoch,
                        operation_value_id, result_value_id
                 FROM language_observations WHERE recorded_sequence = ?1",
                params![sequence],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, i64>(6)?,
                    ))
                },
            )?;
            let operation = serde_json::from_value(load_value(connection, row.5)?)
                .map_err(|error| invalid(format!("invalid stored language operation: {error}")))?;
            let result = serde_json::from_value(load_value(connection, row.6)?)
                .map_err(|error| invalid(format!("invalid stored language result: {error}")))?;
            Ok(DomainEvent::LanguageObservationRecorded {
                observation: phenix_core::LanguageObservation {
                    execution: parse_id(row.0, "execution", ExecutionId::parse)?,
                    workspace: parse_id(row.1, "workspace", WorkspaceId::parse)?,
                    service: parse_id(
                        row.2,
                        "language service",
                        phenix_core::LanguageServiceKind::parse,
                    )?,
                    provider: parse_id(
                        row.3,
                        "language provider",
                        phenix_core::LanguageProviderId::parse,
                    )?,
                    provider_epoch: runtime_u64(row.4, "language provider epoch")?,
                    operation,
                    result,
                },
            })
        }
        "context_injection_recorded" => {
            let row = connection.query_row(
                "SELECT execution_id, source_kind, source_id, source_event_sequence,
                        source_revision, requester, reason, lifetime, content_identity
                 FROM context_injections WHERE recorded_sequence = ?1",
                params![sequence],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<i64>>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, String>(8)?,
                    ))
                },
            )?;
            let source_revision =
                parse_id(row.4, "context source revision", ContextRevision::parse)?;
            Ok(DomainEvent::ContextInjectionRecorded {
                injection: ContextInjection {
                    execution_id: parse_id(row.0, "execution", ExecutionId::parse)?,
                    source_ref: parse_context_source(&row.1, row.2, row.3, &source_revision)?,
                    source_revision,
                    requested_by: parse_context_requester(&row.5)?,
                    reason: row.6,
                    lifetime: parse_context_lifetime(&row.7)?,
                    content_identity: parse_id(
                        row.8,
                        "context content identity",
                        ContextRevision::parse,
                    )?,
                },
            })
        }
        "objective_semantics_activated" => Ok(DomainEvent::ObjectiveSemanticsActivated),
        "objective_created" => Ok(DomainEvent::ObjectiveCreated {
            objective: load_objective_creation(connection, sequence)?,
        }),
        "objective_draft_revised" => Ok(DomainEvent::ObjectiveDraftRevised {
            objective: load_objective_draft_revision(connection, sequence)?,
        }),
        "objective_evidence_recorded" => {
            let (objective, criterion, evidence_ref) = connection.query_row(
                "SELECT objective_id, criterion_id, evidence_ref FROM objective_evidence
                 WHERE sequence = ?1",
                params![sequence],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )?;
            Ok(DomainEvent::ObjectiveEvidenceRecorded {
                objective_id: parse_id(objective, "objective", ObjectiveId::parse)?,
                evidence: ObjectiveCriterionEvidence {
                    criterion_id: parse_id(
                        criterion,
                        "objective criterion",
                        phenix_core::ObjectiveCriterionId::parse,
                    )?,
                    evidence_ref,
                },
            })
        }
        "objective_state_changed" => Ok(DomainEvent::ObjectiveStateChanged {
            transition: load_objective_transition(connection, sequence)?,
        }),
        "execution_objectives_assigned" => Ok(DomainEvent::ExecutionObjectivesAssigned {
            assignment: load_execution_objective_assignment(connection, sequence)?,
        }),
        "plan_created"
        | "plan_draft_revised"
        | "plan_state_changed"
        | "plan_step_state_changed"
        | "execution_plan_assigned" => {
            plan_relational::load_event(connection, sequence, event_type)
        }
        "invocation_resolved" => load_invocation_resolved(connection, sequence),
        "workspace_checkpoint_captured" => load_checkpoint(connection, sequence),
        "workspace_file_observed" => load_observation(connection, sequence),
        "frontend_event" => Ok(DomainEvent::FrontendEvent {
            event: load_frontend_event(connection, sequence)?,
        }),
        other => Err(invalid(format!("unknown relational event type: {other}"))),
    }
}

fn load_session_created(
    connection: &Connection,
    sequence: i64,
) -> Result<DomainEvent, PersistenceError> {
    let row = connection.query_row(
        "SELECT session_id, parent_session_id, workspace_id, config_revision_id, name,
                default_target_id, state
         FROM sessions WHERE created_sequence = ?1",
        params![sequence],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, String>(6)?,
            ))
        },
    )?;
    Ok(DomainEvent::SessionCreated {
        session: SessionSummary {
            id: parse_id(row.0, "session", SessionId::parse)?,
            parent_session: row
                .1
                .map(|id| parse_id(id, "parent session", SessionId::parse))
                .transpose()?,
            workspace_id: parse_id(row.2, "workspace", WorkspaceId::parse)?,
            config_revision: parse_id(
                row.3,
                "configuration revision",
                phenix_core::ConfigRevisionId::parse,
            )?,
            name: row.4,
            default_target: load_target(connection, row.5)?,
            state: parse_session_state(&row.6)?,
        },
    })
}

fn load_execution_created(
    connection: &Connection,
    sequence: i64,
) -> Result<DomainEvent, PersistenceError> {
    let row = connection.query_row(
        "SELECT execution_id, session_id, parent_execution_id, kind, callable_id, target_id,
                state, payload_kind, input_text, input_value_id, authority_filesystem,
                authority_network, authority_repository
         FROM executions WHERE created_sequence = ?1",
        params![sequence],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, Option<i64>>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, String>(11)?,
                row.get::<_, String>(12)?,
            ))
        },
    )?;
    let execution_id = parse_id(row.0, "execution", ExecutionId::parse)?;
    let authority = ExecutionAuthority {
        filesystem: parse_filesystem(&row.10)?,
        network: parse_network(&row.11)?,
        repository: parse_repository(&row.12)?,
        ipc: load_string_set(
            connection,
            "SELECT endpoint FROM execution_authority_ipc WHERE execution_id = ?1 ORDER BY endpoint",
            &execution_id,
        )?,
        secrets: load_string_set(
            connection,
            "SELECT secret_name FROM execution_authority_secrets WHERE execution_id = ?1 ORDER BY secret_name",
            &execution_id,
        )?,
        callables: load_string_set(
            connection,
            "SELECT callable_id FROM execution_authority_callables WHERE execution_id = ?1 ORDER BY callable_id",
            &execution_id,
        )?
        .into_iter()
        .map(|id| parse_id(id, "callable", CallableId::parse))
        .collect::<Result<_, _>>()?,
    };
    let payload = match row.7.as_str() {
        "invocation" => JournalExecutionPayload::Invocation {
            input: row
                .8
                .ok_or_else(|| invalid("invocation execution has no input text"))?,
            authority,
        },
        "orchestration" => JournalExecutionPayload::Orchestration {
            input: load_value(
                connection,
                row.9
                    .ok_or_else(|| invalid("orchestration execution has no input value"))?,
            )?,
            authority,
        },
        other => return Err(invalid(format!("unknown execution payload kind: {other}"))),
    };
    Ok(DomainEvent::ExecutionCreated {
        execution: ExecutionSummary {
            id: execution_id,
            session_id: parse_id(row.1, "session", SessionId::parse)?,
            parent_execution: row
                .2
                .map(|id| parse_id(id, "parent execution", ExecutionId::parse))
                .transpose()?,
            kind: parse_execution_kind(&row.3)?,
            callable: row
                .4
                .map(|id| parse_id(id, "callable", CallableId::parse))
                .transpose()?,
            target: load_target(connection, row.5)?,
            state: parse_execution_state(&row.6)?,
        },
        payload,
    })
}

fn load_attempt_group_created(
    connection: &Connection,
    sequence: i64,
) -> Result<DomainEvent, PersistenceError> {
    let (id, parent, callable, goal) = connection.query_row(
        "SELECT attempt_group_id, parent_execution_id, callable_id, invariant_goal
         FROM attempt_groups WHERE created_sequence = ?1",
        params![sequence],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        },
    )?;
    let group_id = parse_id(id, "attempt group", AttemptGroupId::parse)?;
    let attempts = load_id_list(
        connection,
        "SELECT execution_id FROM attempt_executions
         WHERE attempt_group_id = ?1 AND started_sequence = ?2 ORDER BY attempt_number",
        params![group_id.to_string(), sequence],
        "execution",
        ExecutionId::parse,
    )?;
    let failures = load_attempt_failures_for_sequence(connection, sequence)?
        .into_iter()
        .map(|(_, failure)| failure)
        .collect();
    Ok(DomainEvent::AttemptGroupCreated {
        group: AttemptGroup {
            id: group_id,
            parent_execution: parse_id(parent, "execution", ExecutionId::parse)?,
            callable: parse_id(callable, "callable", CallableId::parse)?,
            goal,
            attempts,
            failures,
        },
    })
}

fn load_attempt_failure(
    connection: &Connection,
    sequence: i64,
) -> Result<(AttemptGroupId, FailureAttemptSummary), PersistenceError> {
    load_attempt_failures_for_sequence(connection, sequence)?
        .into_iter()
        .next()
        .ok_or_else(|| {
            invalid(format!(
                "attempt failure event {sequence} has no relational row"
            ))
        })
}

fn load_attempt_failures_for_sequence(
    connection: &Connection,
    sequence: i64,
) -> Result<Vec<(AttemptGroupId, FailureAttemptSummary)>, PersistenceError> {
    let mut statement = connection.prepare(
        "SELECT attempt_group_id, attempt_number, execution_id, approach, failure_at, reason
         FROM attempt_failures WHERE recorded_sequence = ?1 ORDER BY attempt_number",
    )?;
    let rows = statement.query_map(params![sequence], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
        ))
    })?;
    let mut failures = Vec::new();
    for row in rows {
        let (group, attempt, execution, approach, failure_at, reason) = row?;
        let mut completed = connection.prepare(
            "SELECT item FROM attempt_completed_work
             WHERE attempt_group_id = ?1 AND attempt_number = ?2 ORDER BY item_order",
        )?;
        let completed_work = completed
            .query_map(params![group.as_str(), attempt], |row| {
                row.get::<_, String>(0)
            })?
            .collect::<Result<Vec<_>, _>>()?;
        failures.push((
            parse_id(group, "attempt group", AttemptGroupId::parse)?,
            FailureAttemptSummary {
                execution_id: parse_id(execution, "execution", ExecutionId::parse)?,
                attempt: u32::try_from(attempt)
                    .map_err(|_| invalid("attempt number is outside u32 range"))?,
                approach,
                failure_at,
                reason,
                completed_work,
            },
        ));
    }
    Ok(failures)
}

fn load_decision(
    connection: &Connection,
    sequence: i64,
) -> Result<OrchestrationFailureDecisionRecord, PersistenceError> {
    let row = connection.query_row(
        "SELECT parent_execution_id, failed_child_execution_id, decider_execution_id,
                decision_kind, recovery_execution_id
         FROM parent_failure_decisions WHERE decided_sequence = ?1",
        params![sequence],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        },
    )?;
    Ok(OrchestrationFailureDecisionRecord {
        parent_execution: parse_id(row.0, "execution", ExecutionId::parse)?,
        failed_child: parse_id(row.1, "execution", ExecutionId::parse)?,
        decider_execution: row
            .2
            .map(|id| parse_id(id, "execution", ExecutionId::parse))
            .transpose()?,
        decision: parse_decision(&row.3, row.4)?,
    })
}

fn load_invocation_resolved(
    connection: &Connection,
    sequence: i64,
) -> Result<DomainEvent, PersistenceError> {
    let (execution, requested, model, revision) = connection.query_row(
        "SELECT execution_id, requested_target_id, model_target_id, config_revision_id
         FROM resolved_routing WHERE resolved_sequence = ?1",
        params![sequence],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
            ))
        },
    )?;
    let model = load_target(connection, model)?;
    let ExecutionTarget::Fixed(model) = model else {
        return Err(invalid("resolved model target is not fixed"));
    };
    Ok(DomainEvent::InvocationResolved {
        execution_id: parse_id(execution, "execution", ExecutionId::parse)?,
        route: ResolvedRoute {
            requested_target: load_target(connection, requested)?,
            model,
            config_revision: parse_id(
                revision,
                "configuration revision",
                phenix_core::ConfigRevisionId::parse,
            )?,
        },
    })
}

fn load_checkpoint(
    connection: &Connection,
    sequence: i64,
) -> Result<DomainEvent, PersistenceError> {
    let (execution, workspace) = connection.query_row(
        "SELECT execution_id, workspace_id FROM workspace_checkpoints
         WHERE checkpoint_sequence = ?1",
        params![sequence],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    )?;
    let mut statement = connection.prepare(
        "SELECT path, version_state, content_hash, file_kind
         FROM workspace_checkpoint_files WHERE checkpoint_sequence = ?1 ORDER BY path",
    )?;
    let rows = statement.query_map(params![sequence], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
        ))
    })?;
    let mut files = BTreeMap::new();
    for row in rows {
        let (path, state, hash, kind) = row?;
        files.insert(PathBuf::from(path), parse_file_version(&state, hash, kind)?);
    }
    Ok(DomainEvent::WorkspaceCheckpointCaptured {
        execution_id: parse_id(execution, "execution", ExecutionId::parse)?,
        workspace_id: parse_id(workspace, "workspace", WorkspaceId::parse)?,
        files,
    })
}

fn load_observation(
    connection: &Connection,
    sequence: i64,
) -> Result<DomainEvent, PersistenceError> {
    let (execution, path, state, hash, kind) = connection.query_row(
        "SELECT execution_id, path, version_state, content_hash, file_kind
         FROM workspace_observation_events WHERE observed_sequence = ?1",
        params![sequence],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        },
    )?;
    Ok(DomainEvent::WorkspaceFileObserved {
        execution_id: parse_id(execution, "execution", ExecutionId::parse)?,
        observation: FileObservation {
            path: PathBuf::from(path),
            version: parse_file_version(&state, hash, kind)?,
        },
    })
}

#[allow(clippy::type_complexity)]
fn load_frontend_event(
    connection: &Connection,
    journal_sequence: i64,
) -> Result<ExecutionEvent, PersistenceError> {
    let row = connection.query_row(
        "SELECT event_sequence, session_id, execution_id, kind, text, state,
                termination_kind, termination_execution_id, tool_call_id, callable_id, output,
                success, child_execution_id, decision_parent_execution_id,
                decision_failed_child_execution_id, decision_decider_execution_id, decision_kind,
                decision_recovery_execution_id, error_code, error_message
         FROM canonical_events WHERE journal_sequence = ?1",
        params![journal_sequence],
        |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, Option<String>>(9)?,
                row.get::<_, Option<String>>(10)?,
                row.get::<_, Option<i64>>(11)?,
                row.get::<_, Option<String>>(12)?,
                row.get::<_, Option<String>>(13)?,
                row.get::<_, Option<String>>(14)?,
                row.get::<_, Option<String>>(15)?,
                row.get::<_, Option<String>>(16)?,
                row.get::<_, Option<String>>(17)?,
                row.get::<_, Option<String>>(18)?,
                row.get::<_, Option<String>>(19)?,
            ))
        },
    )?;
    let required = |value: Option<String>, field: &str| {
        value.ok_or_else(|| invalid(format!("frontend event is missing {field}")))
    };
    let kind = match row.3.as_str() {
        "user_input" => ExecutionEventKind::UserInput {
            text: required(row.4, "text")?,
        },
        "execution_state_changed" => ExecutionEventKind::ExecutionStateChanged {
            state: parse_execution_state(&required(row.5, "state")?)?,
        },
        "execution_terminated" => ExecutionEventKind::ExecutionTerminated {
            cause: parse_termination(
                &required(row.6, "termination kind")?,
                required(row.7, "termination execution")?,
            )?,
        },
        "assistant_content_delta" => ExecutionEventKind::AssistantContentDelta {
            text: required(row.4, "text")?,
        },
        "reasoning_delta" => ExecutionEventKind::ReasoningDelta {
            text: required(row.4, "text")?,
        },
        "tool_call_started" => ExecutionEventKind::ToolCallStarted {
            tool_call_id: parse_id(
                required(row.8, "tool call")?,
                "tool call",
                ToolCallId::parse,
            )?,
            callable: parse_id(required(row.9, "callable")?, "callable", CallableId::parse)?,
        },
        "tool_call_arguments" => ExecutionEventKind::ToolCallArguments {
            tool_call_id: parse_id(
                required(row.8, "tool call")?,
                "tool call",
                ToolCallId::parse,
            )?,
            arguments: required(row.4, "arguments")?,
        },
        "tool_call_finished" => ExecutionEventKind::ToolCallFinished {
            tool_call_id: parse_id(
                required(row.8, "tool call")?,
                "tool call",
                ToolCallId::parse,
            )?,
            output: required(row.10, "output")?,
            success: row
                .11
                .ok_or_else(|| invalid("frontend event is missing success"))?
                != 0,
        },
        "child_execution_started" => ExecutionEventKind::ChildExecutionStarted {
            child: parse_id(
                required(row.12, "child execution")?,
                "execution",
                ExecutionId::parse,
            )?,
        },
        "child_execution_finished" => ExecutionEventKind::ChildExecutionFinished {
            child: parse_id(
                required(row.12, "child execution")?,
                "execution",
                ExecutionId::parse,
            )?,
            state: parse_execution_state(&required(row.5, "state")?)?,
        },
        "orchestration_decision_made" => ExecutionEventKind::OrchestrationDecisionMade {
            decision: OrchestrationFailureDecisionRecord {
                parent_execution: parse_id(
                    required(row.13, "decision parent")?,
                    "execution",
                    ExecutionId::parse,
                )?,
                failed_child: parse_id(
                    required(row.14, "decision failed child")?,
                    "execution",
                    ExecutionId::parse,
                )?,
                decider_execution: row
                    .15
                    .map(|id| parse_id(id, "execution", ExecutionId::parse))
                    .transpose()?,
                decision: parse_decision(&required(row.16, "decision kind")?, row.17)?,
            },
        },
        "error" => ExecutionEventKind::Error {
            code: required(row.18, "error code")?,
            message: required(row.19, "error message")?,
        },
        other => return Err(invalid(format!("unknown frontend event kind: {other}"))),
    };
    Ok(ExecutionEvent {
        sequence: runtime_u64(row.0, "frontend event sequence")?,
        session_id: parse_id(row.1, "session", SessionId::parse)?,
        execution_id: parse_id(row.2, "execution", ExecutionId::parse)?,
        kind,
    })
}

fn load_target(
    connection: &Connection,
    target_id: i64,
) -> Result<ExecutionTarget, PersistenceError> {
    let row = connection.query_row(
        "SELECT kind, backend_id, provider_id, model_id, inference_effort, routing_profile_id
         FROM targets WHERE target_id = ?1",
        params![target_id],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        },
    )?;
    match row.0.as_str() {
        "fixed" => Ok(ExecutionTarget::Fixed(ModelTarget {
            backend: parse_id(
                required_column(row.1, "target backend")?,
                "backend",
                BackendId::parse,
            )?,
            provider: parse_id(
                required_column(row.2, "target provider")?,
                "provider",
                ProviderId::parse,
            )?,
            model: parse_id(
                required_column(row.3, "target model")?,
                "model",
                ModelId::parse,
            )?,
            inference: InferenceOptions {
                effort: row
                    .4
                    .map(|value| parse_inference_effort(&value))
                    .transpose()?,
            },
        })),
        "routed" => Ok(ExecutionTarget::Routed(parse_id(
            required_column(row.5, "routing profile")?,
            "routing profile",
            RoutingProfileId::parse,
        )?)),
        other => Err(invalid(format!("unknown target kind: {other}"))),
    }
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

fn load_value(connection: &Connection, value_id: i64) -> Result<Value, PersistenceError> {
    let mut statement = connection.prepare(
        "SELECT node_id, parent_node_id, object_key, array_index, kind, scalar
         FROM structured_value_nodes WHERE value_id = ?1 ORDER BY node_id",
    )?;
    let rows = statement.query_map(params![value_id], |row| {
        Ok(StoredValueNode {
            id: row.get(0)?,
            parent: row.get(1)?,
            object_key: row.get(2)?,
            array_index: row.get(3)?,
            kind: row.get(4)?,
            scalar: row.get(5)?,
        })
    })?;
    let nodes = rows.collect::<Result<Vec<_>, _>>()?;
    let root = nodes
        .iter()
        .find(|node| node.parent.is_none())
        .ok_or_else(|| invalid(format!("structured value {value_id} has no root")))?;
    if nodes.iter().filter(|node| node.parent.is_none()).count() != 1 {
        return Err(invalid(format!(
            "structured value {value_id} has multiple roots"
        )));
    }
    let mut visited = BTreeSet::new();
    let value = materialize_value_node(root.id, &nodes, &mut visited)?;
    if visited.len() != nodes.len() {
        return Err(invalid(format!(
            "structured value {value_id} contains unreachable nodes"
        )));
    }
    Ok(value)
}

fn materialize_value_node(
    node_id: i64,
    nodes: &[StoredValueNode],
    visited: &mut BTreeSet<i64>,
) -> Result<Value, PersistenceError> {
    if !visited.insert(node_id) {
        return Err(invalid("structured value contains a cycle"));
    }
    let node = nodes
        .iter()
        .find(|node| node.id == node_id)
        .ok_or_else(|| invalid("structured value references a missing node"))?;
    let scalar = |name: &str| {
        node.scalar
            .clone()
            .ok_or_else(|| invalid(format!("{name} node has no scalar value")))
    };
    match node.kind.as_str() {
        "null" => Ok(Value::Null),
        "boolean" => match scalar("boolean")?.as_str() {
            "true" => Ok(Value::Bool(true)),
            "false" => Ok(Value::Bool(false)),
            other => Err(invalid(format!("invalid persisted boolean: {other}"))),
        },
        "number" => Ok(Value::Number(
            scalar("number")?
                .parse::<Number>()
                .map_err(|_| invalid("invalid persisted number"))?,
        )),
        "string" => Ok(Value::String(scalar("string")?)),
        "array" => {
            let mut children = nodes
                .iter()
                .filter(|child| child.parent == Some(node_id))
                .map(|child| {
                    child
                        .array_index
                        .ok_or_else(|| invalid("array child has no index"))
                        .map(|index| (index, child.id))
                })
                .collect::<Result<Vec<_>, _>>()?;
            children.sort_by_key(|(index, _)| *index);
            for (expected, (actual, _)) in children.iter().enumerate() {
                if *actual != sql_usize(expected, "structured array index")? {
                    return Err(invalid("structured array indexes are not contiguous"));
                }
            }
            Ok(Value::Array(
                children
                    .into_iter()
                    .map(|(_, child)| materialize_value_node(child, nodes, visited))
                    .collect::<Result<_, _>>()?,
            ))
        }
        "object" => {
            let mut children = nodes
                .iter()
                .filter(|child| child.parent == Some(node_id))
                .map(|child| {
                    child
                        .object_key
                        .clone()
                        .ok_or_else(|| invalid("object child has no key"))
                        .map(|key| (key, child.id))
                })
                .collect::<Result<Vec<_>, _>>()?;
            children.sort_by(|left, right| left.0.cmp(&right.0));
            let mut object = Map::new();
            for (key, child) in children {
                object.insert(key, materialize_value_node(child, nodes, visited)?);
            }
            Ok(Value::Object(object))
        }
        other => Err(invalid(format!(
            "unknown structured value node kind: {other}"
        ))),
    }
}

fn load_string_set(
    connection: &Connection,
    query: &str,
    execution_id: &ExecutionId,
) -> Result<BTreeSet<String>, PersistenceError> {
    let mut statement = connection.prepare(query)?;
    let values = statement
        .query_map(params![execution_id.to_string()], |row| {
            row.get::<_, String>(0)
        })?
        .collect::<Result<_, _>>()?;
    Ok(values)
}

fn load_id_list<T, F, P>(
    connection: &Connection,
    query: &str,
    query_params: P,
    label: &str,
    parse: F,
) -> Result<Vec<T>, PersistenceError>
where
    F: Fn(String) -> Result<T, phenix_core::InvalidId> + Copy,
    P: rusqlite::Params,
{
    let mut statement = connection.prepare(query)?;
    let rows = statement.query_map(query_params, |row| row.get::<_, String>(0))?;
    rows.map(|row| parse_id(row?, label, parse)).collect()
}

fn parse_id<T>(
    value: String,
    label: &str,
    parse: impl FnOnce(String) -> Result<T, phenix_core::InvalidId>,
) -> Result<T, PersistenceError> {
    parse(value).map_err(|_| invalid(format!("database contains an invalid {label}")))
}

fn required_column(value: Option<String>, field: &str) -> Result<String, PersistenceError> {
    value.ok_or_else(|| invalid(format!("database row is missing {field}")))
}

fn sql_u64(value: u64, field: &str) -> Result<i64, PersistenceError> {
    i64::try_from(value).map_err(|_| invalid(format!("{field} exceeds SQLite range")))
}

fn sql_usize(value: usize, field: &str) -> Result<i64, PersistenceError> {
    i64::try_from(value).map_err(|_| invalid(format!("{field} exceeds SQLite range")))
}

fn runtime_u64(value: i64, field: &str) -> Result<u64, PersistenceError> {
    u64::try_from(value).map_err(|_| invalid(format!("database contains an invalid {field}")))
}

fn invalid(message: impl Into<String>) -> PersistenceError {
    PersistenceError::InvalidJournal(message.into())
}

fn execution_event_type(kind: &ExecutionEventKind) -> &'static str {
    match kind {
        ExecutionEventKind::UserInput { .. } => "user_input",
        ExecutionEventKind::ExecutionStateChanged { .. } => "execution_state_changed",
        ExecutionEventKind::ExecutionTerminated { .. } => "execution_terminated",
        ExecutionEventKind::AssistantContentDelta { .. } => "assistant_content_delta",
        ExecutionEventKind::ReasoningDelta { .. } => "reasoning_delta",
        ExecutionEventKind::ToolCallStarted { .. } => "tool_call_started",
        ExecutionEventKind::ToolCallArguments { .. } => "tool_call_arguments",
        ExecutionEventKind::ToolCallFinished { .. } => "tool_call_finished",
        ExecutionEventKind::ChildExecutionStarted { .. } => "child_execution_started",
        ExecutionEventKind::ChildExecutionFinished { .. } => "child_execution_finished",
        ExecutionEventKind::OrchestrationDecisionMade { .. } => "orchestration_decision_made",
        ExecutionEventKind::Error { .. } => "error",
    }
}

fn execution_kind_token(kind: &ExecutionKind) -> &'static str {
    match kind {
        ExecutionKind::Root => "root",
        ExecutionKind::Agent => "agent",
        ExecutionKind::Orchestration => "orchestration",
    }
}

fn parse_execution_kind(value: &str) -> Result<ExecutionKind, PersistenceError> {
    match value {
        "root" => Ok(ExecutionKind::Root),
        "agent" => Ok(ExecutionKind::Agent),
        "orchestration" => Ok(ExecutionKind::Orchestration),
        other => Err(invalid(format!("unknown execution kind: {other}"))),
    }
}

fn execution_state_token(state: &ExecutionState) -> &'static str {
    match state {
        ExecutionState::Pending => "pending",
        ExecutionState::Running => "running",
        ExecutionState::Completed => "completed",
        ExecutionState::Failed => "failed",
        ExecutionState::Cancelled => "cancelled",
        ExecutionState::Interrupted => "interrupted",
    }
}

fn parse_execution_state(value: &str) -> Result<ExecutionState, PersistenceError> {
    match value {
        "pending" => Ok(ExecutionState::Pending),
        "running" => Ok(ExecutionState::Running),
        "completed" => Ok(ExecutionState::Completed),
        "failed" => Ok(ExecutionState::Failed),
        "cancelled" => Ok(ExecutionState::Cancelled),
        "interrupted" => Ok(ExecutionState::Interrupted),
        other => Err(invalid(format!("unknown execution state: {other}"))),
    }
}

fn session_state_token(state: &SessionState) -> &'static str {
    match state {
        SessionState::Active => "active",
        SessionState::Closed => "closed",
    }
}

fn parse_session_state(value: &str) -> Result<SessionState, PersistenceError> {
    match value {
        "active" => Ok(SessionState::Active),
        "closed" => Ok(SessionState::Closed),
        other => Err(invalid(format!("unknown session state: {other}"))),
    }
}

fn filesystem_token(authority: FilesystemAuthority) -> &'static str {
    match authority {
        FilesystemAuthority::ReadOnly => "read_only",
        FilesystemAuthority::Write => "write",
    }
}

fn parse_filesystem(value: &str) -> Result<FilesystemAuthority, PersistenceError> {
    match value {
        "read_only" => Ok(FilesystemAuthority::ReadOnly),
        "write" => Ok(FilesystemAuthority::Write),
        other => Err(invalid(format!("unknown filesystem authority: {other}"))),
    }
}

fn network_token(authority: NetworkAuthority) -> &'static str {
    match authority {
        NetworkAuthority::None => "none",
        NetworkAuthority::Outbound => "outbound",
    }
}

fn parse_network(value: &str) -> Result<NetworkAuthority, PersistenceError> {
    match value {
        "none" => Ok(NetworkAuthority::None),
        "outbound" => Ok(NetworkAuthority::Outbound),
        other => Err(invalid(format!("unknown network authority: {other}"))),
    }
}

fn repository_token(authority: RepositoryAuthority) -> &'static str {
    match authority {
        RepositoryAuthority::Read => "read",
        RepositoryAuthority::Write => "write",
    }
}

fn parse_repository(value: &str) -> Result<RepositoryAuthority, PersistenceError> {
    match value {
        "read" => Ok(RepositoryAuthority::Read),
        "write" => Ok(RepositoryAuthority::Write),
        other => Err(invalid(format!("unknown repository authority: {other}"))),
    }
}

fn inference_effort_token(effort: &InferenceEffort) -> &'static str {
    match effort {
        InferenceEffort::None => "none",
        InferenceEffort::Minimal => "minimal",
        InferenceEffort::Low => "low",
        InferenceEffort::Medium => "medium",
        InferenceEffort::High => "high",
        InferenceEffort::ExtraHigh => "extra_high",
        InferenceEffort::Max => "max",
    }
}

fn parse_inference_effort(value: &str) -> Result<InferenceEffort, PersistenceError> {
    match value {
        "none" => Ok(InferenceEffort::None),
        "minimal" => Ok(InferenceEffort::Minimal),
        "low" => Ok(InferenceEffort::Low),
        "medium" => Ok(InferenceEffort::Medium),
        "high" => Ok(InferenceEffort::High),
        "extra_high" => Ok(InferenceEffort::ExtraHigh),
        "max" => Ok(InferenceEffort::Max),
        other => Err(invalid(format!("unknown inference effort: {other}"))),
    }
}

fn file_kind_token(kind: &FileKind) -> &'static str {
    match kind {
        FileKind::Regular => "regular",
        FileKind::Directory => "directory",
        FileKind::Symlink => "symlink",
        FileKind::Other => "other",
    }
}

fn parse_file_kind(value: &str) -> Result<FileKind, PersistenceError> {
    match value {
        "regular" => Ok(FileKind::Regular),
        "directory" => Ok(FileKind::Directory),
        "symlink" => Ok(FileKind::Symlink),
        "other" => Ok(FileKind::Other),
        other => Err(invalid(format!("unknown file kind: {other}"))),
    }
}

fn parse_file_version(
    state: &str,
    hash: Option<String>,
    kind: Option<String>,
) -> Result<FileVersion, PersistenceError> {
    match state {
        "absent" if hash.is_none() && kind.is_none() => Ok(FileVersion::Absent),
        "present" => Ok(FileVersion::Present {
            content_hash: required_column(hash, "file content hash")?,
            kind: parse_file_kind(&required_column(kind, "file kind")?)?,
        }),
        other => Err(invalid(format!("invalid file version state: {other}"))),
    }
}

fn parse_decision(
    kind: &str,
    recovery: Option<String>,
) -> Result<OrchestrationFailureDecision, PersistenceError> {
    match kind {
        "retry" => Ok(OrchestrationFailureDecision::Retry {
            execution_id: parse_id(
                required_column(recovery, "retry execution")?,
                "execution",
                ExecutionId::parse,
            )?,
        }),
        "choose_another_child" => Ok(OrchestrationFailureDecision::ChooseAnotherChild {
            execution_id: parse_id(
                required_column(recovery, "replacement execution")?,
                "execution",
                ExecutionId::parse,
            )?,
        }),
        "continue" if recovery.is_none() => Ok(OrchestrationFailureDecision::Continue),
        "fail" if recovery.is_none() => Ok(OrchestrationFailureDecision::Fail),
        other => Err(invalid(format!("invalid orchestration decision: {other}"))),
    }
}

fn parse_termination(
    kind: &str,
    execution: String,
) -> Result<ExecutionTerminationCause, PersistenceError> {
    let execution = parse_id(execution, "execution", ExecutionId::parse)?;
    match kind {
        "explicit_cancellation" => Ok(ExecutionTerminationCause::ExplicitCancellation {
            requested_execution: execution,
        }),
        "ancestor_failure" => Ok(ExecutionTerminationCause::AncestorFailure {
            failed_ancestor: execution,
        }),
        other => Err(invalid(format!("unknown termination cause: {other}"))),
    }
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

fn insert_objective_criteria(
    transaction: &Transaction<'_>,
    table: &str,
    sequence_column: &str,
    sequence: i64,
    criteria: &[ObjectiveCriterion],
) -> Result<(), PersistenceError> {
    let sql = format!(
        "INSERT INTO {table}({sequence_column}, criterion_order, criterion_id, description, required) \
         VALUES (?1, ?2, ?3, ?4, ?5)"
    );
    for (index, criterion) in criteria.iter().enumerate() {
        transaction.execute(
            &sql,
            params![
                sequence,
                sql_usize(index, "objective criterion order")?,
                criterion.id.to_string(),
                criterion.description,
                i64::from(criterion.required),
            ],
        )?;
    }
    Ok(())
}

fn objective_origin_columns(origin: &ObjectiveOrigin) -> (&'static str, Option<String>) {
    match origin {
        ObjectiveOrigin::Root => ("root", None),
        ObjectiveOrigin::Derived { parent } => ("derived", Some(parent.to_string())),
    }
}

fn objective_cause_columns(
    cause: &ObjectiveTransitionCause,
) -> (&'static str, Option<String>, Option<String>) {
    match cause {
        ObjectiveTransitionCause::UserIntent => ("user_intent", None, None),
        ObjectiveTransitionCause::AgentAction { execution_id } => {
            ("agent_action", Some(execution_id.to_string()), None)
        }
        ObjectiveTransitionCause::ExecutionOutcome { execution_id } => {
            ("execution_outcome", Some(execution_id.to_string()), None)
        }
        ObjectiveTransitionCause::EvidenceAssessment { evidence_ref } => {
            ("evidence_assessment", None, Some(evidence_ref.clone()))
        }
        ObjectiveTransitionCause::Policy { description } => {
            ("policy", None, Some(description.clone()))
        }
    }
}

fn objective_state_token(state: &ObjectiveState) -> &'static str {
    match state {
        ObjectiveState::Draft => "draft",
        ObjectiveState::Active => "active",
        ObjectiveState::Completed => "completed",
        ObjectiveState::Failed => "failed",
        ObjectiveState::Invalidated => "invalidated",
        ObjectiveState::Abandoned => "abandoned",
        ObjectiveState::Superseded => "superseded",
    }
}

fn parse_objective_state(value: &str) -> Result<ObjectiveState, PersistenceError> {
    match value {
        "draft" => Ok(ObjectiveState::Draft),
        "active" => Ok(ObjectiveState::Active),
        "completed" => Ok(ObjectiveState::Completed),
        "failed" => Ok(ObjectiveState::Failed),
        "invalidated" => Ok(ObjectiveState::Invalidated),
        "abandoned" => Ok(ObjectiveState::Abandoned),
        "superseded" => Ok(ObjectiveState::Superseded),
        other => Err(invalid(format!("unknown objective state: {other}"))),
    }
}

fn load_objective_criteria(
    connection: &Connection,
    table: &str,
    sequence_column: &str,
    sequence: i64,
) -> Result<Vec<ObjectiveCriterion>, PersistenceError> {
    let sql = format!(
        "SELECT criterion_id, description, required FROM {table} \
         WHERE {sequence_column} = ?1 ORDER BY criterion_order"
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params![sequence], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
        ))
    })?;
    rows.map(|row| {
        let (id, description, required) = row?;
        Ok(ObjectiveCriterion {
            id: parse_id(
                id,
                "objective criterion",
                phenix_core::ObjectiveCriterionId::parse,
            )?,
            description,
            required: required != 0,
        })
    })
    .collect()
}

fn load_objective_creation(
    connection: &Connection,
    sequence: i64,
) -> Result<ObjectiveRecord, PersistenceError> {
    let row = connection.query_row(
        "SELECT objective_id, workspace_id, origin_kind, parent_objective_id, statement, state,\n                supersedes_objective_id\n         FROM objective_creations WHERE created_sequence = ?1",
        params![sequence],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<String>>(6)?,
            ))
        },
    )?;
    let origin = match row.2.as_str() {
        "root" if row.3.is_none() => ObjectiveOrigin::Root,
        "derived" => ObjectiveOrigin::Derived {
            parent: parse_id(
                row.3
                    .ok_or_else(|| invalid("derived objective has no parent"))?,
                "objective parent",
                ObjectiveId::parse,
            )?,
        },
        other => return Err(invalid(format!("invalid objective origin: {other}"))),
    };
    Ok(ObjectiveRecord {
        id: parse_id(row.0, "objective", ObjectiveId::parse)?,
        workspace: parse_id(row.1, "workspace", WorkspaceId::parse)?,
        origin,
        statement: row.4,
        criteria: load_objective_criteria(
            connection,
            "objective_creation_criteria",
            "created_sequence",
            sequence,
        )?,
        state: parse_objective_state(&row.5)?,
        supersedes: row
            .6
            .map(|id| parse_id(id, "superseded objective", ObjectiveId::parse))
            .transpose()?,
    })
}

fn load_objective_draft_revision(
    connection: &Connection,
    sequence: i64,
) -> Result<ObjectiveRecord, PersistenceError> {
    let (objective_id, statement) = connection.query_row(
        "SELECT objective_id, statement FROM objective_draft_revisions WHERE sequence = ?1",
        params![sequence],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    )?;
    let objective_id = parse_id(objective_id, "objective", ObjectiveId::parse)?;
    let creation_sequence = connection.query_row(
        "SELECT created_sequence FROM objective_creations WHERE objective_id = ?1",
        params![objective_id.to_string()],
        |row| row.get::<_, i64>(0),
    )?;
    let mut objective = load_objective_creation(connection, creation_sequence)?;
    objective.statement = statement;
    objective.criteria = load_objective_criteria(
        connection,
        "objective_draft_revision_criteria",
        "sequence",
        sequence,
    )?;
    objective.state = ObjectiveState::Draft;
    Ok(objective)
}

fn load_objective_transition(
    connection: &Connection,
    sequence: i64,
) -> Result<ObjectiveTransition, PersistenceError> {
    let row = connection.query_row(
        "SELECT objective_id, from_state, to_state, cause_kind, cause_execution_id, cause_detail\n         FROM objective_state_changes WHERE sequence = ?1",
        params![sequence],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        },
    )?;
    let execution = row
        .4
        .map(|id| parse_id(id, "execution", ExecutionId::parse))
        .transpose()?;
    let cause = match row.3.as_str() {
        "user_intent" => ObjectiveTransitionCause::UserIntent,
        "agent_action" => ObjectiveTransitionCause::AgentAction {
            execution_id: execution
                .ok_or_else(|| invalid("agent objective cause has no execution"))?,
        },
        "execution_outcome" => ObjectiveTransitionCause::ExecutionOutcome {
            execution_id: execution
                .ok_or_else(|| invalid("execution outcome objective cause has no execution"))?,
        },
        "evidence_assessment" => ObjectiveTransitionCause::EvidenceAssessment {
            evidence_ref: row
                .5
                .ok_or_else(|| invalid("evidence objective cause has no reference"))?,
        },
        "policy" => ObjectiveTransitionCause::Policy {
            description: row
                .5
                .ok_or_else(|| invalid("policy objective cause has no description"))?,
        },
        other => return Err(invalid(format!("unknown objective cause: {other}"))),
    };
    Ok(ObjectiveTransition {
        objective_id: parse_id(row.0, "objective", ObjectiveId::parse)?,
        from: parse_objective_state(&row.1)?,
        to: parse_objective_state(&row.2)?,
        cause,
    })
}

fn load_execution_objective_assignment(
    connection: &Connection,
    sequence: i64,
) -> Result<ExecutionObjectiveAssignment, PersistenceError> {
    let (execution, primary) = connection.query_row(
        "SELECT execution_id, primary_objective_id FROM execution_objective_assignments\n         WHERE sequence = ?1",
        params![sequence],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    )?;
    let mut statement = connection.prepare(
        "SELECT objective_id FROM execution_supporting_objectives\n         WHERE sequence = ?1 ORDER BY objective_id",
    )?;
    let supporting = statement
        .query_map(params![sequence], |row| row.get::<_, String>(0))?
        .map(|row| parse_id(row?, "supporting objective", ObjectiveId::parse))
        .collect::<Result<BTreeSet<_>, PersistenceError>>()?;
    Ok(ExecutionObjectiveAssignment {
        execution_id: parse_id(execution, "execution", ExecutionId::parse)?,
        primary: parse_id(primary, "primary objective", ObjectiveId::parse)?,
        supporting,
    })
}

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
                FileObservation {
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
                FileObservation {
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
            .record_language_observation(phenix_core::LanguageObservation {
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
}
