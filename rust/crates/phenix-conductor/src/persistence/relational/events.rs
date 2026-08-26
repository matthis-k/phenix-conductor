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

fn required_column(value: Option<String>, field: &str) -> Result<String, PersistenceError> {
    value.ok_or_else(|| invalid(format!("database row is missing {field}")))
}

fn runtime_u64(value: i64, field: &str) -> Result<u64, PersistenceError> {
    u64::try_from(value).map_err(|_| invalid(format!("database contains an invalid {field}")))
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
        ExecutionEventKind::LifecycleHookMetadata { .. } => "lifecycle_hook_metadata",
        ExecutionEventKind::Error { .. } => "error",
    }
}
