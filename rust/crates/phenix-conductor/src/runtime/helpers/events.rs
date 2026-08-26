fn redact_domain_event(
    event: &mut DomainEvent,
    executions: &BTreeMap<ExecutionId, ExecutionRecord>,
    attempt_groups: &BTreeMap<AttemptGroupId, AttemptGroup>,
) {
    let material_for = |execution_id: &ExecutionId| {
        executions
            .get(execution_id)
            .map(|execution| secret_material(&execution.authority))
            .unwrap_or_default()
    };
    match event {
        DomainEvent::ExecutionCreated { payload, .. } => {
            let (names, values) = secret_material(payload.authority());
            redact_execution_payload(payload, &names, &values);
        }
        DomainEvent::FrontendEvent { event } => {
            let (names, values) = material_for(&event.execution_id);
            redact_event(event, &names, &values);
        }
        DomainEvent::AttemptGroupCreated { group } => {
            let (names, values) = group
                .attempts
                .first()
                .map(&material_for)
                .unwrap_or_default();
            redact_attempt_group(group, &names, &values);
        }
        DomainEvent::AttemptFailureRecorded { group_id, failure } => {
            let (names, values) = attempt_groups
                .get(group_id)
                .and_then(|group| group.attempts.first())
                .map(&material_for)
                .unwrap_or_default();
            redact_failure_summary(failure, &names, &values);
        }
        DomainEvent::OrchestrationNodeInputBound {
            execution_id,
            input,
            ..
        }
        | DomainEvent::ExecutionOutputRecorded {
            execution_id,
            output: input,
        } => {
            let (names, values) = material_for(execution_id);
            redact_value(input, &names, &values);
        }
        DomainEvent::DiagnosticWritePatchCaptured { patch } => {
            let (_, values) = material_for(&patch.execution_id);
            redact_text(&mut patch.patch, &values);
        }
        DomainEvent::LanguageObservationRecorded { observation } => {
            let (names, values) = material_for(&observation.execution);
            redact_value(&mut observation.result.value, &names, &values);
            if let LanguageOperation::WorkspaceSymbols { query } = &mut observation.operation {
                redact_text(query, &values);
            }
        }
        _ => {}
    }
}

fn redact_event(
    event: &mut ExecutionEvent,
    secret_names: &BTreeSet<String>,
    secret_values: &BTreeSet<String>,
) {
    match &mut event.kind {
        ExecutionEventKind::LifecycleHookMetadata { value, .. } => {
            redact_value(value, secret_names, secret_values);
        }
        ExecutionEventKind::UserInput { text }
        | ExecutionEventKind::AssistantContentDelta { text }
        | ExecutionEventKind::ReasoningDelta { text }
        | ExecutionEventKind::ToolCallArguments {
            arguments: text, ..
        }
        | ExecutionEventKind::ToolCallFinished { output: text, .. }
        | ExecutionEventKind::Error { message: text, .. } => {
            if let Ok(mut value) = serde_json::from_str::<Value>(text) {
                redact_value(&mut value, secret_names, secret_values);
                *text = value.to_string();
            } else {
                redact_text(text, secret_values);
            }
        }
        _ => {}
    }
}
