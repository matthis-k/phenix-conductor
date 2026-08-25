use super::{invalid, parse_id, runtime_u64, PersistenceError};
use crate::DomainEvent;
use phenix_core::{
    ContextResourceId, ContextRevision, DecisionApplicability, DecisionCreator,
    DecisionHistoryMatch, DecisionHistoryQuery, DecisionHistoryScope, DecisionId, DecisionRecord,
    DecisionRelation, DecisionState, ExactReference, ExecutionId, FileObservationId,
    LanguageObservationId, ObjectiveId, PlanId, WorkspaceId,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use std::collections::BTreeSet;

pub(super) fn insert_event(
    transaction: &Transaction<'_>,
    sequence: i64,
    event: &DomainEvent,
) -> Result<(), PersistenceError> {
    match event {
        DomainEvent::DecisionDraftCreated { decision }
        | DomainEvent::DecisionDraftRevised { decision, .. } => {
            insert_snapshot(transaction, sequence, decision)?;
        }
        DomainEvent::DecisionRecorded { decision_id } => {
            transaction.execute(
                "INSERT INTO decision_recordings(sequence, decision_id) VALUES (?1, ?2)",
                params![sequence, decision_id.to_string()],
            )?;
            index_decision(transaction, decision_id)?;
        }
        DomainEvent::DecisionApplicabilityAssessed {
            decision_id,
            applicability,
        } => {
            transaction.execute(
                "INSERT INTO decision_applicability_assessments(sequence, decision_id, applicability) VALUES (?1, ?2, ?3)",
                params![sequence, decision_id.to_string(), applicability_token(applicability)],
            )?;
        }
        _ => return Err(invalid("non-decision event passed to decision persistence")),
    }
    Ok(())
}

pub(super) fn load_event(
    connection: &Connection,
    sequence: i64,
    event_type: &str,
) -> Result<DomainEvent, PersistenceError> {
    match event_type {
        "decision_draft_created" => Ok(DomainEvent::DecisionDraftCreated {
            decision: load_snapshot(connection, sequence)?,
        }),
        "decision_draft_revised" => {
            let decision = load_snapshot(connection, sequence)?;
            Ok(DomainEvent::DecisionDraftRevised {
                expected_revision: decision
                    .revision
                    .checked_sub(1)
                    .ok_or_else(|| invalid("decision revision cannot be zero"))?,
                decision,
            })
        }
        "decision_recorded" => Ok(DomainEvent::DecisionRecorded {
            decision_id: load_decision_id(connection, "decision_recordings", sequence)?,
        }),
        "decision_applicability_assessed" => {
            let (decision, applicability) = connection.query_row(
                "SELECT decision_id, applicability FROM decision_applicability_assessments WHERE sequence = ?1",
                params![sequence],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )?;
            Ok(DomainEvent::DecisionApplicabilityAssessed {
                decision_id: parse_id(decision, "decision", DecisionId::parse)?,
                applicability: parse_applicability(&applicability)?,
            })
        }
        other => Err(invalid(format!("unknown decision event type: {other}"))),
    }
}

fn insert_snapshot(
    transaction: &Transaction<'_>,
    sequence: i64,
    decision: &DecisionRecord,
) -> Result<(), PersistenceError> {
    let (creator_kind, creator_execution) = match &decision.creator {
        DecisionCreator::User => ("user", None),
        DecisionCreator::Execution { execution_id } => {
            ("execution", Some(execution_id.to_string()))
        }
    };
    let (relation_kind, relation_decision) = match &decision.relation {
        None => (None, None),
        Some(DecisionRelation::Supersedes { decision_id }) => {
            (Some("supersedes"), Some(decision_id.to_string()))
        }
        Some(DecisionRelation::Reverts { decision_id }) => {
            (Some("reverts"), Some(decision_id.to_string()))
        }
    };
    transaction.execute(
        "INSERT INTO decision_snapshots(sequence, decision_id, workspace_id, revision, question, chosen_option, rationale, creator_kind, creator_execution_id, relation_kind, relation_decision_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![sequence, decision.id.to_string(), decision.workspace.to_string(), i64::try_from(decision.revision).map_err(|_| invalid("decision revision is too large"))?, decision.question, decision.chosen_option, decision.rationale, creator_kind, creator_execution, relation_kind, relation_decision],
    )?;
    for (index, alternative) in decision.alternatives.iter().enumerate() {
        transaction.execute("INSERT INTO decision_alternatives(sequence, alternative_order, alternative) VALUES (?1, ?2, ?3)", params![sequence, i64::try_from(index).map_err(|_| invalid("too many decision alternatives"))?, alternative])?;
    }
    if let Some(reason) = &decision.alternatives_not_considered_reason {
        transaction.execute(
            "INSERT INTO decision_no_alternative_reasons(sequence, reason) VALUES (?1, ?2)",
            params![sequence, reason],
        )?;
    }
    for (index, reference) in decision.evidence.iter().enumerate() {
        let (kind, id, event_sequence, revision) = encode_reference(reference)?;
        transaction.execute("INSERT INTO decision_evidence_refs(sequence, evidence_order, source_kind, source_id, source_event_sequence, source_revision) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", params![sequence, i64::try_from(index).map_err(|_| invalid("too many decision evidence refs"))?, kind, id, event_sequence, revision])?;
    }
    for objective in &decision.objectives {
        transaction.execute(
            "INSERT INTO decision_objectives(sequence, objective_id) VALUES (?1, ?2)",
            params![sequence, objective.to_string()],
        )?;
    }
    for dependency in &decision.dependencies {
        transaction.execute(
            "INSERT INTO decision_dependencies(sequence, dependency_decision_id) VALUES (?1, ?2)",
            params![sequence, dependency.to_string()],
        )?;
    }
    Ok(())
}

fn load_snapshot(
    connection: &Connection,
    sequence: i64,
) -> Result<DecisionRecord, PersistenceError> {
    let row = connection.query_row(
        "SELECT decision_id, workspace_id, revision, question, chosen_option, rationale, creator_kind, creator_execution_id, relation_kind, relation_decision_id FROM decision_snapshots WHERE sequence = ?1",
        params![sequence],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?, row.get::<_, String>(3)?, row.get::<_, String>(4)?, row.get::<_, String>(5)?, row.get::<_, String>(6)?, row.get::<_, Option<String>>(7)?, row.get::<_, Option<String>>(8)?, row.get::<_, Option<String>>(9)?)),
    )?;
    let decision_id = parse_id(row.0, "decision", DecisionId::parse)?;
    let creator = match (row.6.as_str(), row.7) {
        ("user", None) => DecisionCreator::User,
        ("execution", Some(id)) => DecisionCreator::Execution {
            execution_id: parse_id(id, "decision creator execution", ExecutionId::parse)?,
        },
        _ => return Err(invalid("invalid decision creator columns")),
    };
    let relation = match (row.8.as_deref(), row.9) {
        (None, None) => None,
        (Some("supersedes"), Some(id)) => Some(DecisionRelation::Supersedes {
            decision_id: parse_id(id, "related decision", DecisionId::parse)?,
        }),
        (Some("reverts"), Some(id)) => Some(DecisionRelation::Reverts {
            decision_id: parse_id(id, "related decision", DecisionId::parse)?,
        }),
        _ => return Err(invalid("invalid decision relation columns")),
    };
    let recorded = connection
        .query_row(
            "SELECT 1 FROM decision_recordings WHERE decision_id = ?1 AND sequence < ?2",
            params![decision_id.to_string(), sequence],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    Ok(DecisionRecord {
        id: decision_id,
        workspace: parse_id(row.1, "workspace", WorkspaceId::parse)?,
        revision: runtime_u64(row.2, "decision revision")?,
        state: if recorded {
            DecisionState::Recorded
        } else {
            DecisionState::Draft
        },
        question: row.3,
        chosen_option: row.4,
        rationale: row.5,
        alternatives: load_strings(connection, "decision_alternatives", "alternative", sequence)?,
        alternatives_not_considered_reason: connection
            .query_row(
                "SELECT reason FROM decision_no_alternative_reasons WHERE sequence = ?1",
                params![sequence],
                |row| row.get::<_, String>(0),
            )
            .optional()?,
        evidence: load_evidence(connection, sequence)?,
        creator,
        objectives: load_ids(
            connection,
            "decision_objectives",
            "objective_id",
            sequence,
            ObjectiveId::parse,
        )?,
        dependencies: load_ids(
            connection,
            "decision_dependencies",
            "dependency_decision_id",
            sequence,
            DecisionId::parse,
        )?,
        relation,
        applicability: DecisionApplicability::Applicable,
    })
}

fn load_decision_id(
    connection: &Connection,
    table: &str,
    sequence: i64,
) -> Result<DecisionId, PersistenceError> {
    let sql = format!("SELECT decision_id FROM {table} WHERE sequence = ?1");
    let value = connection.query_row(&sql, params![sequence], |row| row.get::<_, String>(0))?;
    parse_id(value, "decision", DecisionId::parse)
}

fn load_strings(
    connection: &Connection,
    table: &str,
    column: &str,
    sequence: i64,
) -> Result<Vec<String>, PersistenceError> {
    let sql =
        format!("SELECT {column} FROM {table} WHERE sequence = ?1 ORDER BY alternative_order");
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params![sequence], |row| row.get::<_, String>(0))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(PersistenceError::from)
}

fn load_ids<T, F>(
    connection: &Connection,
    table: &str,
    column: &str,
    sequence: i64,
    parse: F,
) -> Result<BTreeSet<T>, PersistenceError>
where
    T: Ord,
    F: Fn(String) -> Result<T, phenix_core::InvalidId> + Copy,
{
    let sql = format!("SELECT {column} FROM {table} WHERE sequence = ?1 ORDER BY {column}");
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params![sequence], |row| row.get::<_, String>(0))?;
    rows.map(|row| {
        row.map_err(PersistenceError::from)
            .and_then(|id| parse_id(id, column, parse))
    })
    .collect()
}

fn load_evidence(
    connection: &Connection,
    sequence: i64,
) -> Result<Vec<ExactReference>, PersistenceError> {
    let mut statement = connection.prepare("SELECT source_kind, source_id, source_event_sequence, source_revision FROM decision_evidence_refs WHERE sequence = ?1 ORDER BY evidence_order")?;
    let rows = statement.query_map(params![sequence], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<i64>>(2)?,
            row.get::<_, Option<String>>(3)?,
        ))
    })?;
    rows.map(|row| {
        let (kind, id, event, revision) = row?;
        decode_reference(&kind, id, event, revision)
    })
    .collect()
}

type EncodedExactReference = (&'static str, Option<String>, Option<i64>, Option<String>);

fn encode_reference(reference: &ExactReference) -> Result<EncodedExactReference, PersistenceError> {
    Ok(match reference {
        ExactReference::Objective(id) => ("objective", Some(id.to_string()), None, None),
        ExactReference::Plan(id) => ("plan", Some(id.to_string()), None, None),
        ExactReference::Decision(id) => ("decision", Some(id.to_string()), None, None),
        ExactReference::Execution(id) => ("execution", Some(id.to_string()), None, None),
        ExactReference::Event(sequence) => (
            "event",
            None,
            Some(i64::try_from(*sequence).map_err(|_| invalid("event sequence is too large"))?),
            None,
        ),
        ExactReference::FileObservation(id) => {
            ("file_observation", Some(id.to_string()), None, None)
        }
        ExactReference::LanguageObservation(id) => {
            ("language_observation", Some(id.to_string()), None, None)
        }
        ExactReference::Context {
            resource_id,
            revision,
        } => (
            "context",
            Some(resource_id.to_string()),
            None,
            Some(revision.to_string()),
        ),
    })
}

fn decode_reference(
    kind: &str,
    id: Option<String>,
    event: Option<i64>,
    revision: Option<String>,
) -> Result<ExactReference, PersistenceError> {
    let required_id = |label: &str| {
        id.clone()
            .ok_or_else(|| invalid(format!("missing {label} id")))
    };
    match kind {
        "objective" => Ok(ExactReference::Objective(parse_id(
            required_id("objective")?,
            "objective",
            ObjectiveId::parse,
        )?)),
        "plan" => Ok(ExactReference::Plan(parse_id(
            required_id("plan")?,
            "plan",
            PlanId::parse,
        )?)),
        "decision" => Ok(ExactReference::Decision(parse_id(
            required_id("decision")?,
            "decision",
            DecisionId::parse,
        )?)),
        "execution" => Ok(ExactReference::Execution(parse_id(
            required_id("execution")?,
            "execution",
            ExecutionId::parse,
        )?)),
        "event" => Ok(ExactReference::Event(runtime_u64(
            event.ok_or_else(|| invalid("missing event sequence"))?,
            "event sequence",
        )?)),
        "file_observation" => Ok(ExactReference::FileObservation(parse_id(
            required_id("file observation")?,
            "file observation",
            FileObservationId::parse,
        )?)),
        "language_observation" => Ok(ExactReference::LanguageObservation(parse_id(
            required_id("language observation")?,
            "language observation",
            LanguageObservationId::parse,
        )?)),
        "context" => Ok(ExactReference::Context {
            resource_id: parse_id(
                required_id("context resource")?,
                "context resource",
                ContextResourceId::parse,
            )?,
            revision: parse_id(
                revision.ok_or_else(|| invalid("missing context revision"))?,
                "context revision",
                ContextRevision::parse,
            )?,
        }),
        other => Err(invalid(format!(
            "unknown decision evidence reference kind: {other}"
        ))),
    }
}

fn applicability_token(value: &DecisionApplicability) -> &'static str {
    match value {
        DecisionApplicability::Applicable => "applicable",
        DecisionApplicability::Questionable => "questionable",
        DecisionApplicability::Invalidated => "invalidated",
    }
}
fn parse_applicability(value: &str) -> Result<DecisionApplicability, PersistenceError> {
    match value {
        "applicable" => Ok(DecisionApplicability::Applicable),
        "questionable" => Ok(DecisionApplicability::Questionable),
        "invalidated" => Ok(DecisionApplicability::Invalidated),
        other => Err(invalid(format!("unknown decision applicability: {other}"))),
    }
}

fn index_decision(
    transaction: &Transaction<'_>,
    decision_id: &DecisionId,
) -> Result<(), PersistenceError> {
    let row = transaction.query_row(
        "SELECT sequence, question, chosen_option, rationale FROM decision_snapshots WHERE decision_id = ?1 ORDER BY revision DESC LIMIT 1",
        params![decision_id.to_string()],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?)),
    )?;
    let alternatives =
        load_strings(transaction, "decision_alternatives", "alternative", row.0)?.join("\n");
    let reason = transaction
        .query_row(
            "SELECT reason FROM decision_no_alternative_reasons WHERE sequence = ?1",
            params![row.0],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let alternative_provenance = if alternatives.is_empty() {
        reason.unwrap_or_default()
    } else {
        alternatives
    };
    transaction.execute("INSERT INTO decision_fts(decision_id, question, chosen_option, rationale, alternatives) VALUES (?1, ?2, ?3, ?4, ?5)", params![decision_id.to_string(), row.1, row.2, row.3, alternative_provenance])?;
    Ok(())
}

impl super::SqliteStore {
    pub fn rebuild_decision_search_index(&self) -> Result<(), PersistenceError> {
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        transaction.execute("DELETE FROM decision_fts", [])?;
        let mut statement =
            transaction.prepare("SELECT decision_id FROM decision_recordings ORDER BY sequence")?;
        let ids = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        drop(statement);
        for id in ids {
            index_decision(&transaction, &parse_id(id, "decision", DecisionId::parse)?)?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn search_decision_history(
        &self,
        query: &DecisionHistoryQuery,
    ) -> Result<Vec<DecisionHistoryMatch>, PersistenceError> {
        let connection = self.open()?;
        let limit = i64::try_from(query.limit.max(1))
            .map_err(|_| invalid("history query limit is too large"))?;
        let sql = match &query.scope {
            DecisionHistoryScope::Workspace => "SELECT f.decision_id, f.question, f.chosen_option, f.rationale FROM decision_fts f WHERE decision_fts MATCH ?1 ORDER BY bm25(decision_fts) LIMIT ?2".to_owned(),
            DecisionHistoryScope::ObjectiveLineage(_) => "WITH RECURSIVE lineage(objective_id) AS (SELECT ?3 UNION ALL SELECT o.parent_objective_id FROM objective_creations o JOIN lineage l ON o.objective_id = l.objective_id WHERE o.parent_objective_id IS NOT NULL), latest(decision_id, sequence) AS (SELECT s.decision_id, MAX(s.sequence) FROM decision_snapshots s JOIN decision_recordings r ON r.decision_id = s.decision_id GROUP BY s.decision_id) SELECT f.decision_id, f.question, f.chosen_option, f.rationale FROM decision_fts f JOIN latest l ON l.decision_id = f.decision_id JOIN decision_objectives d ON d.sequence = l.sequence JOIN lineage x ON x.objective_id = d.objective_id WHERE decision_fts MATCH ?1 ORDER BY bm25(decision_fts) LIMIT ?2".to_owned(),
        };
        let objective = match &query.scope {
            DecisionHistoryScope::ObjectiveLineage(id) => Some(id.to_string()),
            DecisionHistoryScope::Workspace => None,
        };
        let mut statement = connection.prepare(&sql)?;
        let mut rows = if let Some(objective) = objective {
            statement.query(params![query.text, limit, objective])?
        } else {
            statement.query(params![query.text, limit])?
        };
        let mut matches = Vec::new();
        while let Some(row) = rows.next()? {
            let id = parse_id(row.get::<_, String>(0)?, "decision", DecisionId::parse)?;
            let applicability = connection.query_row("SELECT applicability FROM decision_applicability_assessments WHERE decision_id = ?1 ORDER BY sequence DESC LIMIT 1", params![id.to_string()], |row| row.get::<_, String>(0)).optional()?.map(|value| parse_applicability(&value)).transpose()?.unwrap_or(DecisionApplicability::Applicable);
            matches.push(DecisionHistoryMatch {
                decision_id: id,
                question: row.get(1)?,
                chosen_option: row.get(2)?,
                rationale: row.get(3)?,
                applicability,
            });
        }
        Ok(matches)
    }
}
