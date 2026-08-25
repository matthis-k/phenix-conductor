use super::{invalid, parse_id, runtime_u64, PersistenceError};
use crate::DomainEvent;
use phenix_core::{
    ExecutionId, ExecutionPlanAssignment, ObjectiveId, PlanId, PlanRecord, PlanState, PlanStep,
    PlanStepId, PlanStepRevisability, PlanStepState, PlanStepTransition, PlanTransition,
    PlanTransitionCause, WorkspaceId,
};
use rusqlite::{params, Connection, Transaction};
use std::collections::BTreeSet;

pub(super) fn insert_event(
    transaction: &Transaction<'_>,
    sequence: i64,
    event: &DomainEvent,
) -> Result<(), PersistenceError> {
    match event {
        DomainEvent::PlanCreated { plan } => {
            transaction.execute(
                "INSERT INTO plan_creations(
                     sequence, plan_id, workspace_id, revision, supersedes_plan_id
                 ) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    sequence,
                    plan.id.to_string(),
                    plan.workspace.to_string(),
                    sql_u64(plan.revision, "plan revision")?,
                    plan.supersedes.as_ref().map(ToString::to_string),
                ],
            )?;
            insert_objectives(
                transaction,
                "plan_creation_objectives",
                sequence,
                &plan.objective_refs,
            )?;
            insert_steps(transaction, "creation", sequence, &plan.steps)?;
        }
        DomainEvent::PlanDraftRevised {
            plan,
            expected_revision,
        } => {
            transaction.execute(
                "INSERT INTO plan_draft_revisions(
                     sequence, plan_id, expected_revision, revision
                 ) VALUES (?1, ?2, ?3, ?4)",
                params![
                    sequence,
                    plan.id.to_string(),
                    sql_u64(*expected_revision, "expected plan revision")?,
                    sql_u64(plan.revision, "plan revision")?,
                ],
            )?;
            insert_objectives(
                transaction,
                "plan_revision_objectives",
                sequence,
                &plan.objective_refs,
            )?;
            insert_steps(transaction, "revision", sequence, &plan.steps)?;
        }
        DomainEvent::PlanStateChanged { transition } => {
            let (kind, execution, detail) = cause_columns(&transition.cause);
            transaction.execute(
                "INSERT INTO plan_state_changes(
                     sequence, plan_id, from_state, to_state, cause_kind,
                     cause_execution_id, cause_detail
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    sequence,
                    transition.plan_id.to_string(),
                    plan_state_token(&transition.from),
                    plan_state_token(&transition.to),
                    kind,
                    execution,
                    detail,
                ],
            )?;
        }
        DomainEvent::PlanStepStateChanged { transition } => {
            let (kind, execution, detail) = cause_columns(&transition.cause);
            transaction.execute(
                "INSERT INTO plan_step_state_changes(
                     sequence, plan_id, step_id, from_state, to_state, cause_kind,
                     cause_execution_id, cause_detail
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    sequence,
                    transition.plan_id.to_string(),
                    transition.step_id.to_string(),
                    step_state_token(&transition.from),
                    step_state_token(&transition.to),
                    kind,
                    execution,
                    detail,
                ],
            )?;
        }
        DomainEvent::ExecutionPlanAssigned { assignment } => {
            transaction.execute(
                "INSERT INTO execution_plan_assignments(
                     sequence, execution_id, plan_id, step_id
                 ) VALUES (?1, ?2, ?3, ?4)",
                params![
                    sequence,
                    assignment.execution_id.to_string(),
                    assignment.plan_id.to_string(),
                    assignment.step_id.to_string(),
                ],
            )?;
        }
        _ => return Err(invalid("non-plan event passed to plan persistence")),
    }
    Ok(())
}

pub(super) fn load_event(
    connection: &Connection,
    sequence: i64,
    event_type: &str,
) -> Result<DomainEvent, PersistenceError> {
    match event_type {
        "plan_created" => Ok(DomainEvent::PlanCreated {
            plan: load_creation(connection, sequence)?,
        }),
        "plan_draft_revised" => {
            let (plan, expected_revision) = load_revision(connection, sequence)?;
            Ok(DomainEvent::PlanDraftRevised {
                plan,
                expected_revision,
            })
        }
        "plan_state_changed" => Ok(DomainEvent::PlanStateChanged {
            transition: load_plan_transition(connection, sequence)?,
        }),
        "plan_step_state_changed" => Ok(DomainEvent::PlanStepStateChanged {
            transition: load_step_transition(connection, sequence)?,
        }),
        "execution_plan_assigned" => {
            let (execution, plan, step) = connection.query_row(
                "SELECT execution_id, plan_id, step_id
                 FROM execution_plan_assignments WHERE sequence = ?1",
                params![sequence],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )?;
            Ok(DomainEvent::ExecutionPlanAssigned {
                assignment: ExecutionPlanAssignment {
                    execution_id: parse_id(execution, "execution", ExecutionId::parse)?,
                    plan_id: parse_id(plan, "plan", PlanId::parse)?,
                    step_id: parse_id(step, "plan step", PlanStepId::parse)?,
                },
            })
        }
        other => Err(invalid(format!("unknown plan event type: {other}"))),
    }
}

fn insert_objectives(
    transaction: &Transaction<'_>,
    table: &str,
    sequence: i64,
    objectives: &BTreeSet<ObjectiveId>,
) -> Result<(), PersistenceError> {
    let sql = format!("INSERT INTO {table}(sequence, objective_id) VALUES (?1, ?2)");
    for objective in objectives {
        transaction.execute(&sql, params![sequence, objective.to_string()])?;
    }
    Ok(())
}

fn insert_steps(
    transaction: &Transaction<'_>,
    kind: &str,
    sequence: i64,
    steps: &[PlanStep],
) -> Result<(), PersistenceError> {
    let (steps_table, deps_table, objectives_table) = match kind {
        "creation" => (
            "plan_creation_steps",
            "plan_creation_step_dependencies",
            "plan_creation_step_objectives",
        ),
        "revision" => (
            "plan_revision_steps",
            "plan_revision_step_dependencies",
            "plan_revision_step_objectives",
        ),
        _ => unreachable!("static plan shape kind"),
    };
    let step_sql = format!(
        "INSERT INTO {steps_table}(
             sequence, step_order, step_id, description, state, revisability
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
    );
    let dep_sql = format!(
        "INSERT INTO {deps_table}(sequence, step_id, dependency_step_id)
         VALUES (?1, ?2, ?3)"
    );
    let objective_sql = format!(
        "INSERT INTO {objectives_table}(sequence, step_id, objective_id)
         VALUES (?1, ?2, ?3)"
    );
    for (index, step) in steps.iter().enumerate() {
        transaction.execute(
            &step_sql,
            params![
                sequence,
                i64::try_from(index).map_err(|_| invalid("plan contains too many steps"))?,
                step.id.to_string(),
                step.description,
                step_state_token(&step.state),
                revisability_token(&step.revisability),
            ],
        )?;
        for dependency in &step.depends_on {
            transaction.execute(
                &dep_sql,
                params![sequence, step.id.to_string(), dependency.to_string()],
            )?;
        }
        for objective in &step.objective_refs {
            transaction.execute(
                &objective_sql,
                params![sequence, step.id.to_string(), objective.to_string()],
            )?;
        }
    }
    Ok(())
}

fn load_creation(connection: &Connection, sequence: i64) -> Result<PlanRecord, PersistenceError> {
    let (plan, workspace, revision, supersedes) = connection.query_row(
        "SELECT plan_id, workspace_id, revision, supersedes_plan_id
         FROM plan_creations WHERE sequence = ?1",
        params![sequence],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        },
    )?;
    Ok(PlanRecord {
        id: parse_id(plan, "plan", PlanId::parse)?,
        workspace: parse_id(workspace, "workspace", WorkspaceId::parse)?,
        state: PlanState::Draft,
        revision: runtime_u64(revision, "plan revision")?,
        objective_refs: load_objectives(connection, "plan_creation_objectives", sequence)?,
        supersedes: supersedes
            .map(|id| parse_id(id, "superseded plan", PlanId::parse))
            .transpose()?,
        steps: load_steps(connection, "creation", sequence)?,
    })
}

fn load_revision(
    connection: &Connection,
    sequence: i64,
) -> Result<(PlanRecord, u64), PersistenceError> {
    let (plan, expected, revision) = connection.query_row(
        "SELECT plan_id, expected_revision, revision
         FROM plan_draft_revisions WHERE sequence = ?1",
        params![sequence],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        },
    )?;
    let (workspace, supersedes) = connection.query_row(
        "SELECT workspace_id, supersedes_plan_id FROM plan_creations WHERE plan_id = ?1",
        params![plan],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
    )?;
    Ok((
        PlanRecord {
            id: parse_id(plan, "plan", PlanId::parse)?,
            workspace: parse_id(workspace, "workspace", WorkspaceId::parse)?,
            state: PlanState::Draft,
            revision: runtime_u64(revision, "plan revision")?,
            objective_refs: load_objectives(connection, "plan_revision_objectives", sequence)?,
            supersedes: supersedes
                .map(|id| parse_id(id, "superseded plan", PlanId::parse))
                .transpose()?,
            steps: load_steps(connection, "revision", sequence)?,
        },
        runtime_u64(expected, "expected plan revision")?,
    ))
}

fn load_objectives(
    connection: &Connection,
    table: &str,
    sequence: i64,
) -> Result<BTreeSet<ObjectiveId>, PersistenceError> {
    let sql = format!("SELECT objective_id FROM {table} WHERE sequence = ?1 ORDER BY objective_id");
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params![sequence], |row| row.get::<_, String>(0))?;
    rows.map(|row| {
        row.map_err(PersistenceError::from)
            .and_then(|id| parse_id(id, "objective", ObjectiveId::parse))
    })
    .collect()
}

fn load_steps(
    connection: &Connection,
    kind: &str,
    sequence: i64,
) -> Result<Vec<PlanStep>, PersistenceError> {
    let (steps_table, deps_table, objectives_table) = match kind {
        "creation" => (
            "plan_creation_steps",
            "plan_creation_step_dependencies",
            "plan_creation_step_objectives",
        ),
        "revision" => (
            "plan_revision_steps",
            "plan_revision_step_dependencies",
            "plan_revision_step_objectives",
        ),
        _ => unreachable!("static plan shape kind"),
    };
    let sql = format!(
        "SELECT step_id, description, state, revisability
         FROM {steps_table} WHERE sequence = ?1 ORDER BY step_order"
    );
    let rows = {
        let mut statement = connection.prepare(&sql)?;
        let rows = statement
            .query_map(params![sequence], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        rows
    };
    let mut steps = Vec::with_capacity(rows.len());
    for (step, description, state, revisability) in rows {
        let step_id = parse_id(step, "plan step", PlanStepId::parse)?;
        steps.push(PlanStep {
            id: step_id.clone(),
            description,
            state: parse_step_state(&state)?,
            revisability: parse_revisability(&revisability)?,
            depends_on: load_step_ids(
                connection,
                deps_table,
                "dependency_step_id",
                sequence,
                &step_id,
            )?,
            objective_refs: load_step_objectives(connection, objectives_table, sequence, &step_id)?,
        });
    }
    Ok(steps)
}

fn load_step_ids(
    connection: &Connection,
    table: &str,
    column: &str,
    sequence: i64,
    step_id: &PlanStepId,
) -> Result<BTreeSet<PlanStepId>, PersistenceError> {
    let sql = format!(
        "SELECT {column} FROM {table} WHERE sequence = ?1 AND step_id = ?2 ORDER BY {column}"
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params![sequence, step_id.to_string()], |row| {
        row.get::<_, String>(0)
    })?;
    rows.map(|row| {
        row.map_err(PersistenceError::from)
            .and_then(|id| parse_id(id, "plan step", PlanStepId::parse))
    })
    .collect()
}

fn load_step_objectives(
    connection: &Connection,
    table: &str,
    sequence: i64,
    step_id: &PlanStepId,
) -> Result<BTreeSet<ObjectiveId>, PersistenceError> {
    let sql = format!(
        "SELECT objective_id FROM {table} WHERE sequence = ?1 AND step_id = ?2 ORDER BY objective_id"
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params![sequence, step_id.to_string()], |row| {
        row.get::<_, String>(0)
    })?;
    rows.map(|row| {
        row.map_err(PersistenceError::from)
            .and_then(|id| parse_id(id, "objective", ObjectiveId::parse))
    })
    .collect()
}

fn load_plan_transition(
    connection: &Connection,
    sequence: i64,
) -> Result<PlanTransition, PersistenceError> {
    let row = connection.query_row(
        "SELECT plan_id, from_state, to_state, cause_kind,
                cause_execution_id, cause_detail
         FROM plan_state_changes WHERE sequence = ?1",
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
    Ok(PlanTransition {
        plan_id: parse_id(row.0, "plan", PlanId::parse)?,
        from: parse_plan_state(&row.1)?,
        to: parse_plan_state(&row.2)?,
        cause: parse_cause(&row.3, row.4, row.5)?,
    })
}

fn load_step_transition(
    connection: &Connection,
    sequence: i64,
) -> Result<PlanStepTransition, PersistenceError> {
    let row = connection.query_row(
        "SELECT plan_id, step_id, from_state, to_state, cause_kind,
                cause_execution_id, cause_detail
         FROM plan_step_state_changes WHERE sequence = ?1",
        params![sequence],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
            ))
        },
    )?;
    Ok(PlanStepTransition {
        plan_id: parse_id(row.0, "plan", PlanId::parse)?,
        step_id: parse_id(row.1, "plan step", PlanStepId::parse)?,
        from: parse_step_state(&row.2)?,
        to: parse_step_state(&row.3)?,
        cause: parse_cause(&row.4, row.5, row.6)?,
    })
}

fn cause_columns(cause: &PlanTransitionCause) -> (&'static str, Option<String>, Option<String>) {
    match cause {
        PlanTransitionCause::AgentAction { execution_id } => {
            ("agent_action", Some(execution_id.to_string()), None)
        }
        PlanTransitionCause::ExecutionOutcome { execution_id } => {
            ("execution_outcome", Some(execution_id.to_string()), None)
        }
        PlanTransitionCause::EvidenceAssessment { evidence_ref } => {
            ("evidence_assessment", None, Some(evidence_ref.clone()))
        }
        PlanTransitionCause::UserAction => ("user_action", None, None),
        PlanTransitionCause::Policy { description } => ("policy", None, Some(description.clone())),
    }
}

fn parse_cause(
    kind: &str,
    execution: Option<String>,
    detail: Option<String>,
) -> Result<PlanTransitionCause, PersistenceError> {
    match kind {
        "agent_action" => Ok(PlanTransitionCause::AgentAction {
            execution_id: parse_id(
                execution.ok_or_else(|| invalid("agent plan cause is missing execution"))?,
                "execution",
                ExecutionId::parse,
            )?,
        }),
        "execution_outcome" => Ok(PlanTransitionCause::ExecutionOutcome {
            execution_id: parse_id(
                execution.ok_or_else(|| invalid("execution plan cause is missing execution"))?,
                "execution",
                ExecutionId::parse,
            )?,
        }),
        "evidence_assessment" => Ok(PlanTransitionCause::EvidenceAssessment {
            evidence_ref: detail.ok_or_else(|| invalid("plan evidence cause is missing detail"))?,
        }),
        "user_action" => Ok(PlanTransitionCause::UserAction),
        "policy" => Ok(PlanTransitionCause::Policy {
            description: detail.ok_or_else(|| invalid("plan policy cause is missing detail"))?,
        }),
        other => Err(invalid(format!("unknown plan cause: {other}"))),
    }
}

fn plan_state_token(state: &PlanState) -> &'static str {
    match state {
        PlanState::Draft => "draft",
        PlanState::Active => "active",
        PlanState::Completed => "completed",
        PlanState::Failed => "failed",
        PlanState::Invalidated => "invalidated",
        PlanState::Abandoned => "abandoned",
        PlanState::Superseded => "superseded",
    }
}

fn parse_plan_state(value: &str) -> Result<PlanState, PersistenceError> {
    match value {
        "draft" => Ok(PlanState::Draft),
        "active" => Ok(PlanState::Active),
        "completed" => Ok(PlanState::Completed),
        "failed" => Ok(PlanState::Failed),
        "invalidated" => Ok(PlanState::Invalidated),
        "abandoned" => Ok(PlanState::Abandoned),
        "superseded" => Ok(PlanState::Superseded),
        other => Err(invalid(format!("unknown plan state: {other}"))),
    }
}

fn step_state_token(state: &PlanStepState) -> &'static str {
    match state {
        PlanStepState::Proposed => "proposed",
        PlanStepState::Committed => "committed",
        PlanStepState::Active => "active",
        PlanStepState::Completed => "completed",
        PlanStepState::Failed => "failed",
        PlanStepState::Invalidated => "invalidated",
        PlanStepState::Abandoned => "abandoned",
    }
}

fn parse_step_state(value: &str) -> Result<PlanStepState, PersistenceError> {
    match value {
        "proposed" => Ok(PlanStepState::Proposed),
        "committed" => Ok(PlanStepState::Committed),
        "active" => Ok(PlanStepState::Active),
        "completed" => Ok(PlanStepState::Completed),
        "failed" => Ok(PlanStepState::Failed),
        "invalidated" => Ok(PlanStepState::Invalidated),
        "abandoned" => Ok(PlanStepState::Abandoned),
        other => Err(invalid(format!("unknown plan step state: {other}"))),
    }
}

fn revisability_token(value: &PlanStepRevisability) -> &'static str {
    match value {
        PlanStepRevisability::Revisable => "revisable",
        PlanStepRevisability::Fixed => "fixed",
    }
}

fn parse_revisability(value: &str) -> Result<PlanStepRevisability, PersistenceError> {
    match value {
        "revisable" => Ok(PlanStepRevisability::Revisable),
        "fixed" => Ok(PlanStepRevisability::Fixed),
        other => Err(invalid(format!("unknown plan step revisability: {other}"))),
    }
}

fn sql_u64(value: u64, label: &str) -> Result<i64, PersistenceError> {
    i64::try_from(value).map_err(|_| invalid(format!("{label} does not fit SQLite INTEGER")))
}
