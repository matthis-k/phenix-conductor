impl ConductorRuntime {
    pub fn register_tool<F, O>(
        &mut self,
        descriptor: CallableDescriptor,
        handler: F,
    ) -> Result<(), ConductorError>
    where
        F: Fn(&str) -> Result<O, String> + Send + Sync + 'static,
        O: Into<ToolOutcome> + 'static,
    {
        self.revise_configuration(move |configuration| {
            configuration.register_tool(descriptor, handler)
        })?;
        Ok(())
    }

    pub fn tool_descriptors(&self) -> Result<Vec<CallableDescriptor>, ConductorError> {
        Ok(self.current_configuration()?.tool_descriptors())
    }

    fn permitted_tool_descriptors(
        &self,
        execution_id: &ExecutionId,
    ) -> Result<Vec<CallableDescriptor>, ConductorError> {
        let authority = self.execution_authority(execution_id)?;
        Ok(self
            .configuration_for_execution(execution_id)?
            .callables
            .tool_descriptors()
            .into_iter()
            .filter(|descriptor| {
                authority
                    .filesystem
                    .permits_capabilities(&descriptor.capabilities)
            })
            .collect())
    }

    fn check_callable_policy(
        &self,
        execution_id: &ExecutionId,
        descriptor: &CallableDescriptor,
        operation: CallableOperation,
    ) -> Result<(), ConductorError> {
        let execution = self
            .executions
            .get(execution_id)
            .ok_or_else(|| ConductorError::UnknownExecution(execution_id.clone()))?;
        let context = InvocationPolicyContext {
            session_id: &execution.summary.session_id,
            execution_id,
            config_revision: &execution.config_revision,
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

    fn invoke_tool(
        &mut self,
        execution_id: &ExecutionId,
        allowed_tools: &BTreeSet<CallableId>,
        invocation: ToolInvocation,
    ) -> Result<ToolResult, BackendError> {
        let callables = self
            .configuration_for_execution(execution_id)
            .map_err(conductor_protocol_error)?
            .callables
            .clone();
        if !allowed_tools.contains(&invocation.callable)
            || !callables.contains(&invocation.callable)
        {
            return Err(BackendError::Protocol(format!(
                "backend invoked unprovisioned tool {}",
                invocation.callable
            )));
        }
        self.dispatch_lifecycle_hooks(execution_id, LifecycleEvent::CallableStarted)
            .map_err(conductor_protocol_error)?;
        let tool_call_id = self.new_tool_call_id();
        self.push_event(
            execution_id,
            ExecutionEventKind::ToolCallStarted {
                tool_call_id: tool_call_id.clone(),
                callable: invocation.callable.clone(),
            },
        )
        .map_err(conductor_protocol_error)?;
        self.push_event(
            execution_id,
            ExecutionEventKind::ToolCallArguments {
                tool_call_id: tool_call_id.clone(),
                arguments: invocation.arguments_json.clone(),
            },
        )
        .map_err(conductor_protocol_error)?;

        let descriptor = callables
            .descriptor(&invocation.callable)
            .map_err(|error| BackendError::Protocol(error.to_string()))?
            .clone();
        let result = match self.check_callable_policy(
            execution_id,
            &descriptor,
            CallableOperation::InvokeTool,
        ) {
            Ok(()) => match serde_json::from_str::<Value>(&invocation.arguments_json) {
                Ok(_) => {
                    let authority = self
                        .execution_authority(execution_id)
                        .map_err(conductor_protocol_error)?;
                    let workspace_id = self.workspace_id.clone();
                    let language_configuration = self
                        .configuration_for_execution(execution_id)
                        .map_err(conductor_protocol_error)?
                        .language_service_configuration()
                        .clone();
                    let sandbox_state = self
                        .execution_sandbox_state(execution_id)
                        .map_err(|error| BackendError::Protocol(error.to_string()))?;
                    let context = callables::ToolExecutionContext {
                        execution_id: execution_id.clone(),
                        workspace_id,
                        language_configuration,
                        authority,
                        sandbox_state,
                    };
                    let outcome = callables
                        .invoke_tool(&context, &invocation.callable, &invocation.arguments_json)
                        .map_err(|error| BackendError::Protocol(error.to_string()))?;
                    if outcome.success {
                        for observation in outcome.file_observations.iter().cloned() {
                            self.record_file_observation(execution_id, observation)
                                .map_err(conductor_protocol_error)?;
                        }
                    }
                    for mut patch in outcome.diagnostic_write_patches.iter().cloned() {
                        let (_, secret_values) = secret_material(&context.authority);
                        redact_text(&mut patch.patch, &secret_values);
                        self.record_domain_event(DomainEvent::DiagnosticWritePatchCaptured {
                            patch,
                        })
                        .map_err(conductor_protocol_error)?;
                    }
                    for observation in outcome.language_observations.iter().cloned() {
                        self.record_language_observation(observation)
                            .map_err(conductor_protocol_error)?;
                    }
                    outcome.into_backend_result()
                }
                Err(error) => ToolResult {
                    output: format!("invalid JSON tool arguments: {error}"),
                    success: false,
                },
            },
            Err(ConductorError::PolicyDenied { denial, .. }) => ToolResult {
                output: denial.message,
                success: false,
            },
            Err(error) => return Err(conductor_protocol_error(error)),
        };
        self.dispatch_lifecycle_hooks(execution_id, LifecycleEvent::CallableCompleted)
            .map_err(conductor_protocol_error)?;
        self.push_event(
            execution_id,
            ExecutionEventKind::ToolCallFinished {
                tool_call_id,
                output: result.output.clone(),
                success: result.success,
            },
        )
        .map_err(conductor_protocol_error)?;
        Ok(result)
    }

    fn new_tool_call_id(&self) -> ToolCallId {
        ToolCallId::parse(format!("tool-call-{}", self.next_tool_call + 1)).expect("generated id")
    }
}
