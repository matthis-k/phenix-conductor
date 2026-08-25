fn fail_runtime_execution(
    runtime: &mut ConductorRuntime,
    execution_id: &ExecutionId,
    error: ProtocolError,
) -> Result<(), ConductorError> {
    let Some(state) = runtime.execution_state(execution_id) else {
        return Err(ConductorError::UnknownExecution(execution_id.clone()));
    };
    if is_terminal_state(&state) {
        return Ok(());
    }
    runtime.push_event(
        execution_id,
        ExecutionEventKind::Error {
            code: format!("{:?}", error.code).to_lowercase(),
            message: error.message,
        },
    )?;
    runtime.set_state(execution_id, ExecutionState::Failed)
}

fn map_execution_provider_error(error: ExecutionProviderError) -> ProtocolError {
    match error {
        ExecutionProviderError::Unsupported(message) => {
            protocol_error(ErrorCode::UnsupportedCapability, message)
        }
        ExecutionProviderError::Failed(message) | ExecutionProviderError::Protocol(message) => {
            protocol_error(ErrorCode::ExecutionProviderFailure, message)
        }
    }
}
