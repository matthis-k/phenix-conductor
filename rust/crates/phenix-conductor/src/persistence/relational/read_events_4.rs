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
        "lifecycle_hook_metadata" => {
            let encoded = required(row.4, "lifecycle hook metadata")?;
            let (hook_id, key, value): (String, String, Value) =
                serde_json::from_str(&encoded).map_err(|error| {
                    invalid(format!("invalid lifecycle hook metadata: {error}"))
                })?;
            ExecutionEventKind::LifecycleHookMetadata { hook_id, key, value }
        }
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

fn parse_execution_kind(value: &str) -> Result<ExecutionKind, PersistenceError> {
    match value {
        "root" => Ok(ExecutionKind::Root),
        "agent" => Ok(ExecutionKind::Agent),
        "orchestration" => Ok(ExecutionKind::Orchestration),
        other => Err(invalid(format!("unknown execution kind: {other}"))),
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

fn parse_session_state(value: &str) -> Result<SessionState, PersistenceError> {
    match value {
        "active" => Ok(SessionState::Active),
        "closed" => Ok(SessionState::Closed),
        other => Err(invalid(format!("unknown session state: {other}"))),
    }
}

fn parse_filesystem(value: &str) -> Result<FilesystemAuthority, PersistenceError> {
    match value {
        "read_only" => Ok(FilesystemAuthority::ReadOnly),
        "write" => Ok(FilesystemAuthority::Write),
        other => Err(invalid(format!("unknown filesystem authority: {other}"))),
    }
}

fn parse_network(value: &str) -> Result<NetworkAuthority, PersistenceError> {
    match value {
        "none" => Ok(NetworkAuthority::None),
        "outbound" => Ok(NetworkAuthority::Outbound),
        other => Err(invalid(format!("unknown network authority: {other}"))),
    }
}

fn parse_repository(value: &str) -> Result<RepositoryAuthority, PersistenceError> {
    match value {
        "read" => Ok(RepositoryAuthority::Read),
        "write" => Ok(RepositoryAuthority::Write),
        other => Err(invalid(format!("unknown repository authority: {other}"))),
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
