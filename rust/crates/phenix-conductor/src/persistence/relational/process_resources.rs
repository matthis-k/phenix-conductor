fn owner_columns(owner: &crate::DurableResourceOwner) -> (&'static str, String) {
    match owner {
        crate::DurableResourceOwner::Execution(id) => ("execution", id.to_string()),
        crate::DurableResourceOwner::Workspace(id) => ("workspace", id.to_string()),
    }
}

fn load_owner(kind: &str, id: String) -> Result<crate::DurableResourceOwner, PersistenceError> {
    match kind {
        "execution" => Ok(crate::DurableResourceOwner::Execution(parse_id(
            id,
            "process resource execution owner",
            ExecutionId::parse,
        )?)),
        "workspace" => Ok(crate::DurableResourceOwner::Workspace(parse_id(
            id,
            "process resource workspace owner",
            WorkspaceId::parse,
        )?)),
        other => Err(invalid(format!(
            "unknown process resource owner kind: {other}"
        ))),
    }
}

fn insert_authority(
    transaction: &Transaction<'_>,
    authority: &ExecutionAuthority,
) -> Result<i64, PersistenceError> {
    let value = serde_json::to_value(authority)
        .map_err(|error| invalid(format!("cannot encode process resource authority: {error}")))?;
    insert_value(transaction, &value)
}

fn load_authority(
    connection: &Connection,
    value_id: i64,
) -> Result<ExecutionAuthority, PersistenceError> {
    serde_json::from_value(load_value(connection, value_id)?).map_err(|error| {
        invalid(format!(
            "invalid process resource authority structured value: {error}"
        ))
    })
}

fn insert_process_resource_event(
    transaction: &Transaction<'_>,
    sequence: i64,
    event: &DomainEvent,
) -> Result<(), PersistenceError> {
    match event {
        DomainEvent::TerminalCreated { terminal } => {
            let (owner_kind, owner_id) = owner_columns(&terminal.owner);
            let authority = insert_authority(transaction, &terminal.authority)?;
            transaction.execute(
                "INSERT INTO terminal_resources(
                     terminal_id, owner_kind, owner_id, created_by_execution_id,
                     authority_value_id, created_sequence
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    terminal.id.as_str(),
                    owner_kind,
                    owner_id,
                    terminal.created_by.to_string(),
                    authority,
                    sequence
                ],
            )?;
        }
        DomainEvent::JobCreated { job } => {
            let (owner_kind, owner_id) = owner_columns(&job.owner);
            let authority = insert_authority(transaction, &job.authority)?;
            transaction.execute(
                "INSERT INTO job_resources(
                     job_id, owner_kind, owner_id, created_by_execution_id,
                     authority_value_id, created_sequence
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    job.id.as_str(),
                    owner_kind,
                    owner_id,
                    job.created_by.to_string(),
                    authority,
                    sequence
                ],
            )?;
        }
        DomainEvent::TerminalStateChanged { terminal_id, state } => {
            insert_process_state(transaction, sequence, "terminal", terminal_id.as_str(), state)?;
        }
        DomainEvent::JobStateChanged { job_id, state } => {
            insert_process_state(transaction, sequence, "job", job_id.as_str(), state)?;
        }
        DomainEvent::JobPromoted {
            job_id,
            workspace_id,
        } => {
            transaction.execute(
                "INSERT INTO process_resource_events(
                     sequence, resource_kind, resource_id, event_kind, owner_id
                 ) VALUES (?1, 'job', ?2, 'promoted', ?3)",
                params![sequence, job_id.as_str(), workspace_id.to_string()],
            )?;
        }
        DomainEvent::TerminalOutputRecorded {
            terminal_id,
            output,
        } => insert_process_output(
            transaction,
            sequence,
            "terminal",
            terminal_id.as_str(),
            output,
        )?,
        DomainEvent::JobOutputRecorded { job_id, output } => {
            insert_process_output(transaction, sequence, "job", job_id.as_str(), output)?;
        }
        _ => unreachable!("non process-resource event"),
    }
    Ok(())
}

fn insert_process_state(
    transaction: &Transaction<'_>,
    sequence: i64,
    kind: &str,
    id: &str,
    state: &crate::DurableProcessState,
) -> Result<(), PersistenceError> {
    let (event_kind, exit_code) = match state {
        crate::DurableProcessState::Running => ("running", None),
        crate::DurableProcessState::Exited { code } => ("exited", code.map(i64::from)),
        crate::DurableProcessState::Revoked => ("revoked", None),
    };
    transaction.execute(
        "INSERT INTO process_resource_events(
             sequence, resource_kind, resource_id, event_kind, exit_code
         ) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![sequence, kind, id, event_kind, exit_code],
    )?;
    Ok(())
}

fn insert_process_output(
    transaction: &Transaction<'_>,
    sequence: i64,
    kind: &str,
    id: &str,
    output: &ExactReference,
) -> Result<(), PersistenceError> {
    let value = serde_json::to_value(output)
        .map_err(|error| invalid(format!("cannot encode process output reference: {error}")))?;
    let value_id = insert_value(transaction, &value)?;
    transaction.execute(
        "INSERT INTO process_resource_events(
             sequence, resource_kind, resource_id, event_kind, output_ref_value_id
         ) VALUES (?1, ?2, ?3, 'output', ?4)",
        params![sequence, kind, id, value_id],
    )?;
    Ok(())
}

fn load_process_resource_event(
    connection: &Connection,
    sequence: i64,
    event_type: &str,
) -> Result<DomainEvent, PersistenceError> {
    match event_type {
        "terminal_created" => {
            let row = connection.query_row(
                "SELECT terminal_id, owner_kind, owner_id, created_by_execution_id,
                        authority_value_id
                 FROM terminal_resources WHERE created_sequence = ?1",
                params![sequence],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, i64>(4)?,
                    ))
                },
            )?;
            Ok(DomainEvent::TerminalCreated {
                terminal: crate::TerminalRecord {
                    id: crate::TerminalId::parse(row.0)
                        .map_err(|_| invalid("invalid terminal id"))?,
                    owner: load_owner(&row.1, row.2)?,
                    created_by: parse_id(row.3, "terminal creator", ExecutionId::parse)?,
                    authority: load_authority(connection, row.4)?,
                    state: crate::DurableProcessState::Running,
                    output_refs: Vec::new(),
                },
            })
        }
        "job_created" => {
            let row = connection.query_row(
                "SELECT job_id, owner_kind, owner_id, created_by_execution_id,
                        authority_value_id
                 FROM job_resources WHERE created_sequence = ?1",
                params![sequence],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, i64>(4)?,
                    ))
                },
            )?;
            Ok(DomainEvent::JobCreated {
                job: crate::JobRecord {
                    id: crate::JobId::parse(row.0).map_err(|_| invalid("invalid job id"))?,
                    owner: load_owner(&row.1, row.2)?,
                    created_by: parse_id(row.3, "job creator", ExecutionId::parse)?,
                    authority: load_authority(connection, row.4)?,
                    state: crate::DurableProcessState::Running,
                    output_refs: Vec::new(),
                },
            })
        }
        _ => {
            let row = connection.query_row(
                "SELECT resource_kind, resource_id, event_kind, owner_id, exit_code,
                        output_ref_value_id
                 FROM process_resource_events WHERE sequence = ?1",
                params![sequence],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Option<i64>>(4)?,
                        row.get::<_, Option<i64>>(5)?,
                    ))
                },
            )?;
            match (row.0.as_str(), row.2.as_str()) {
                ("terminal", "exited") => Ok(DomainEvent::TerminalStateChanged {
                    terminal_id: crate::TerminalId::parse(row.1)
                        .map_err(|_| invalid("invalid terminal id"))?,
                    state: crate::DurableProcessState::Exited {
                        code: row
                            .4
                            .map(|code| {
                                i32::try_from(code)
                                    .map_err(|_| invalid("terminal exit code out of range"))
                            })
                            .transpose()?,
                    },
                }),
                ("terminal", "revoked") => Ok(DomainEvent::TerminalStateChanged {
                    terminal_id: crate::TerminalId::parse(row.1)
                        .map_err(|_| invalid("invalid terminal id"))?,
                    state: crate::DurableProcessState::Revoked,
                }),
                ("job", "exited") => Ok(DomainEvent::JobStateChanged {
                    job_id: crate::JobId::parse(row.1).map_err(|_| invalid("invalid job id"))?,
                    state: crate::DurableProcessState::Exited {
                        code: row
                            .4
                            .map(|code| {
                                i32::try_from(code)
                                    .map_err(|_| invalid("job exit code out of range"))
                            })
                            .transpose()?,
                    },
                }),
                ("job", "revoked") => Ok(DomainEvent::JobStateChanged {
                    job_id: crate::JobId::parse(row.1).map_err(|_| invalid("invalid job id"))?,
                    state: crate::DurableProcessState::Revoked,
                }),
                ("job", "promoted") => Ok(DomainEvent::JobPromoted {
                    job_id: crate::JobId::parse(row.1).map_err(|_| invalid("invalid job id"))?,
                    workspace_id: parse_id(
                        row.3.ok_or_else(|| invalid("missing promoted workspace"))?,
                        "promoted workspace",
                        WorkspaceId::parse,
                    )?,
                }),
                ("terminal", "output") => Ok(DomainEvent::TerminalOutputRecorded {
                    terminal_id: crate::TerminalId::parse(row.1)
                        .map_err(|_| invalid("invalid terminal id"))?,
                    output: load_process_output(connection, row.5, "terminal")?,
                }),
                ("job", "output") => Ok(DomainEvent::JobOutputRecorded {
                    job_id: crate::JobId::parse(row.1).map_err(|_| invalid("invalid job id"))?,
                    output: load_process_output(connection, row.5, "job")?,
                }),
                _ => Err(invalid(format!(
                    "invalid process resource event: {} {}",
                    row.0, row.2
                ))),
            }
        }
    }
}

fn load_process_output(
    connection: &Connection,
    value_id: Option<i64>,
    kind: &str,
) -> Result<ExactReference, PersistenceError> {
    let value_id = value_id.ok_or_else(|| invalid(format!("missing {kind} output ref")))?;
    serde_json::from_value(load_value(connection, value_id)?).map_err(|error| {
        invalid(format!(
            "invalid {kind} output ref structured value: {error}"
        ))
    })
}
