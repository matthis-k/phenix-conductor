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
        ExecutionEventKind::LifecycleHookMetadata { hook_id, key, value } => {
            columns.text = Some(
                serde_json::to_string(&(hook_id, key, value)).map_err(|error| {
                    invalid(format!("cannot encode lifecycle hook metadata: {error}"))
                })?,
            );
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
