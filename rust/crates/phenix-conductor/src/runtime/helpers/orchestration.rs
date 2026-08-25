fn redact_attempt_group(
    group: &mut AttemptGroup,
    secret_names: &BTreeSet<String>,
    secret_values: &BTreeSet<String>,
) {
    redact_text(&mut group.goal, secret_values);
    for failure in &mut group.failures {
        redact_failure_summary(failure, secret_names, secret_values);
    }
}

fn redact_failure_summary(
    failure: &mut phenix_core::FailureAttemptSummary,
    secret_names: &BTreeSet<String>,
    secret_values: &BTreeSet<String>,
) {
    for text in [
        &mut failure.approach,
        &mut failure.failure_at,
        &mut failure.reason,
    ] {
        if let Ok(mut value) = serde_json::from_str::<Value>(text) {
            redact_value(&mut value, secret_names, secret_values);
            *text = value.to_string();
        } else {
            redact_text(text, secret_values);
        }
    }
    for completed in &mut failure.completed_work {
        redact_text(completed, secret_values);
    }
}
