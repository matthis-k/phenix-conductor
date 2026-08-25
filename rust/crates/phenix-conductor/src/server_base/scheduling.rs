impl ConductorServer {
    pub(super) fn execution_group_id_for(
        &self,
        execution_id: &ExecutionId,
    ) -> Result<ExecutionId, ProtocolError> {
        let snapshot = self
            .lock_runtime()
            .map_err(|error| protocol_error(ErrorCode::BackendProtocol, error.to_string()))?
            .snapshot();
        execution_group_id(&snapshot.executions, execution_id).ok_or_else(|| {
            let mut error = protocol_error(
                ErrorCode::UnknownId,
                format!("unknown execution: {execution_id}"),
            );
            error.execution_id = Some(execution_id.clone());
            error
        })
    }

    fn submit(
        &mut self,
        request_id: u64,
        session_id: SessionId,
        text: String,
        output: &SyncSender<ServerMessage>,
        executions: &ExecutionQueue,
        on_root: RootAcceptedHook<'_>,
    ) -> Result<(), ServerError> {
        let execution = match self.lock_runtime()?.submit(&session_id, text) {
            Ok(execution) => execution,
            Err(error) => {
                self.respond(output, request_id, Err(map_conductor_error(error)))?;
                return Ok(());
            }
        };
        let execution_id = execution.id.clone();
        self.persist()?;
        on_root(&execution_id)?;
        self.respond(output, request_id, Ok(Reply::Execution { execution }))?;
        enqueue_pending_execution_group(&self.runtime, &execution_id, executions)
    }

    fn start_callable(
        &mut self,
        request_id: u64,
        session_id: SessionId,
        callable: CallableId,
        input: serde_json::Value,
        output: &SyncSender<ServerMessage>,
        executions: &ExecutionQueue,
        on_root: RootAcceptedHook<'_>,
    ) -> Result<(), ServerError> {
        let execution =
            match self
                .lock_runtime()?
                .start_session_callable(&session_id, &callable, input)
            {
                Ok(execution) => execution,
                Err(error) => {
                    self.respond(output, request_id, Err(map_conductor_error(error)))?;
                    return Ok(());
                }
            };
        let execution_id = execution.id.clone();
        self.persist()?;
        on_root(&execution_id)?;
        self.respond(
            output,
            request_id,
            Ok(Reply::Execution {
                execution: execution.clone(),
            }),
        )?;
        enqueue_pending_execution_group(&self.runtime, &execution_id, executions)
    }
}
