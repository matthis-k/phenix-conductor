fn conductor_protocol_error(error: ConductorError) -> BackendError {
    BackendError::Protocol(error.to_string())
}

fn validate_json_schema(schema: &Value, value: &Value) -> Result<(), String> {
    let validator = jsonschema::validator_for(schema)
        .map_err(|error| format!("invalid configured JSON schema: {error}"))?;
    if let Err(error) = validator.validate(value) {
        return Err(error.to_string());
    }
    Ok(())
}

fn secret_material(authority: &ExecutionAuthority) -> (BTreeSet<String>, BTreeSet<String>) {
    let names = authority.secrets.clone();
    let values = names
        .iter()
        .filter_map(|name| std::env::var(name).ok())
        .filter(|value| !value.is_empty())
        .collect();
    (names, values)
}

fn is_terminal(state: &ExecutionState) -> bool {
    matches!(
        state,
        ExecutionState::Completed
            | ExecutionState::Failed
            | ExecutionState::Cancelled
            | ExecutionState::Interrupted
    )
}

fn authority_envelope<'a>(
    authorities: impl IntoIterator<Item = &'a ExecutionAuthority>,
) -> ExecutionAuthority {
    let mut envelope = ExecutionAuthority::read_only();
    for authority in authorities {
        envelope.filesystem = envelope.filesystem.max(authority.filesystem);
        envelope.network = envelope.network.max(authority.network);
        envelope.repository = envelope.repository.max(authority.repository);
        envelope.ipc.extend(authority.ipc.iter().cloned());
        envelope.secrets.extend(authority.secrets.iter().cloned());
        envelope
            .callables
            .extend(authority.callables.iter().cloned());
    }
    envelope
}
