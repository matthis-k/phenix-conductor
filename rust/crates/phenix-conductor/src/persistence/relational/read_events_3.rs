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
    let (observation_id, execution, path, state, hash, kind) = connection.query_row(
        "SELECT observation_id, execution_id, path, version_state, content_hash, file_kind
         FROM workspace_observation_events WHERE observed_sequence = ?1",
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
    Ok(DomainEvent::WorkspaceFileObserved {
        execution_id: parse_id(execution, "execution", ExecutionId::parse)?,
        observation: FileObservation {
            id: parse_id(observation_id, "file observation", FileObservationId::parse)?,
            path: PathBuf::from(path),
            version: parse_file_version(&state, hash, kind)?,
        },
    })
}
