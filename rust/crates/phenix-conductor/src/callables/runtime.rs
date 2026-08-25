use super::*;

impl ConductorRuntime {
    /// Start an agent or orchestration as a first-class top-level execution in a
    /// session. This is the conductor-owned entrypoint used by frontends; it
    /// does not synthesize a model-backed wrapper execution.
    pub fn start_session_callable(
        &mut self,
        session_id: &SessionId,
        callable: &CallableId,
        input: impl Into<Value>,
    ) -> Result<ExecutionSummary, ConductorError> {
        self.start_session_callable_with_restrictions(session_id, callable, input, None)
    }

    pub fn start_session_callable_with_restrictions(
        &mut self,
        session_id: &SessionId,
        callable: &CallableId,
        input: impl Into<Value>,
        restrictions: Option<&ExecutionAuthority>,
    ) -> Result<ExecutionSummary, ConductorError> {
        let input = input.into();
        if input.as_str().is_some_and(|input| input.trim().is_empty()) {
            return Err(ConductorError::EmptyInput);
        }
        let callables = self
            .configuration_for_session(session_id)?
            .callables
            .clone();
        let descriptor = callables.descriptor(callable)?.clone();
        let execution_id = self.new_execution_id();

        match descriptor.kind {
            CallableKind::Agent => {
                callables.execution_provider(callable)?;
                self.check_session_callable_policy(
                    session_id,
                    &execution_id,
                    &descriptor,
                    CallableOperation::StartAgent,
                )?;
                self.create_session_callable_execution(
                    session_id,
                    execution_id,
                    ExecutionKind::Agent,
                    callable.clone(),
                    ExecutionPayload::Invocation {
                        input: input
                            .as_str()
                            .map(str::to_owned)
                            .unwrap_or_else(|| input.to_string()),
                    },
                    restrictions,
                )
            }
            CallableKind::Orchestration => {
                let definition = callables.orchestration(callable)?.clone();
                crate::validate_json_schema(&definition.descriptor.input_schema, &input).map_err(
                    |message| ConductorError::InvalidExecutionData {
                        execution_id: execution_id.clone(),
                        message: format!("orchestration input: {message}"),
                    },
                )?;
                self.check_session_callable_policy(
                    session_id,
                    &execution_id,
                    &definition.descriptor,
                    CallableOperation::StartOrchestration,
                )?;
                for node in &definition.nodes {
                    let node_descriptor = callables.descriptor(&node.callable)?.clone();
                    callables.execution_provider(&node.callable)?;
                    self.check_session_callable_policy(
                        session_id,
                        &execution_id,
                        &node_descriptor,
                        CallableOperation::StartAgentNode,
                    )?;
                }
                if let Some(interface_agent) = definition.interface_agent.as_ref() {
                    let descriptor = callables.descriptor(interface_agent)?.clone();
                    callables.execution_provider(interface_agent)?;
                    self.check_session_callable_policy(
                        session_id,
                        &execution_id,
                        &descriptor,
                        CallableOperation::StartAgentNode,
                    )?;
                }
                let summary = self.create_session_callable_execution(
                    session_id,
                    execution_id,
                    ExecutionKind::Orchestration,
                    callable.clone(),
                    ExecutionPayload::Orchestration { input },
                    restrictions,
                )?;
                self.set_state(&summary.id, ExecutionState::Running)?;
                self.advance_orchestration(&summary.id)?;
                Ok(self
                    .executions
                    .get(&summary.id)
                    .expect("orchestration exists after top-level creation")
                    .summary
                    .clone())
            }
            CallableKind::Tool => {
                Err(CallableRegistryError::NotExecutable(callable.clone()).into())
            }
        }
    }

    fn check_session_callable_policy(
        &self,
        session_id: &SessionId,
        execution_id: &ExecutionId,
        descriptor: &CallableDescriptor,
        operation: CallableOperation,
    ) -> Result<(), ConductorError> {
        let session = self
            .sessions
            .get(session_id)
            .ok_or_else(|| ConductorError::UnknownSession(session_id.clone()))?;
        let context = InvocationPolicyContext {
            session_id,
            execution_id,
            config_revision: &session.summary.config_revision,
            subject: InvocationSubject::Callable {
                descriptor,
                operation,
            },
        };
        self.policy
            .check(&context)
            .map_err(|denial| ConductorError::PolicyDenied {
                execution_id: execution_id.clone(),
                denial,
            })
    }

    fn create_session_callable_execution(
        &mut self,
        session_id: &SessionId,
        execution_id: ExecutionId,
        kind: ExecutionKind,
        callable: CallableId,
        payload: ExecutionPayload,
        restrictions: Option<&ExecutionAuthority>,
    ) -> Result<ExecutionSummary, ConductorError> {
        let user_input = match &payload {
            ExecutionPayload::Invocation { input } => input.clone(),
            ExecutionPayload::Orchestration { input } => input.to_string(),
        };
        let target = self
            .sessions
            .get(session_id)
            .ok_or_else(|| ConductorError::UnknownSession(session_id.clone()))?
            .summary
            .default_target
            .clone();
        let summary = ExecutionSummary {
            id: execution_id,
            session_id: session_id.clone(),
            parent_execution: None,
            kind,
            callable: Some(callable),
            target,
            state: ExecutionState::Pending,
        };
        self.record_execution_created(
            summary.clone(),
            JournalExecutionPayload::from(&payload),
            restrictions,
        )?;
        self.accept_root_submission(&summary)?;
        self.push_event(
            &summary.id,
            ExecutionEventKind::UserInput { text: user_input },
        )?;
        self.push_event(
            &summary.id,
            ExecutionEventKind::ExecutionStateChanged {
                state: ExecutionState::Pending,
            },
        )?;
        Ok(summary)
    }
}
