fn redact_execution_payload(
    payload: &mut JournalExecutionPayload,
    secret_names: &BTreeSet<String>,
    secret_values: &BTreeSet<String>,
) {
    match payload {
        JournalExecutionPayload::Invocation { input, .. } => {
            if let Ok(mut value) = serde_json::from_str::<Value>(input) {
                redact_value(&mut value, secret_names, secret_values);
                *input = value.to_string();
            } else {
                redact_text(input, secret_values);
            }
        }
        JournalExecutionPayload::Orchestration { input, .. } => {
            redact_value(input, secret_names, secret_values);
        }
    }
}

fn redact_value(
    value: &mut Value,
    secret_names: &BTreeSet<String>,
    secret_values: &BTreeSet<String>,
) {
    match value {
        Value::String(text) => redact_text(text, secret_values),
        Value::Array(values) => {
            for value in values {
                redact_value(value, secret_names, secret_values);
            }
        }
        Value::Object(values) => {
            for (name, value) in values {
                if secret_names.contains(name) {
                    *value = Value::String("[REDACTED]".to_owned());
                } else {
                    redact_value(value, secret_names, secret_values);
                }
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

fn redact_text(text: &mut String, secrets: &BTreeSet<String>) {
    let mut secrets = secrets.iter().collect::<Vec<_>>();
    secrets.sort_by_key(|secret| std::cmp::Reverse(secret.len()));
    for secret in secrets {
        if text.contains(secret) {
            *text = text.replace(secret, "[REDACTED]");
        }
    }
}
