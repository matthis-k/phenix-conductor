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
