impl ConductorRuntime {
    pub fn register_invocation_guard<G>(&mut self, guard: G)
    where
        G: InvocationGuard + 'static,
    {
        self.policy.register(guard);
    }

    pub fn register_provider_agent<P>(
        &mut self,
        definition: AgentDefinition,
        provider: P,
    ) -> Result<(), ConductorError>
    where
        P: ExecutionProvider + 'static,
    {
        self.revise_configuration(move |configuration| {
            configuration.register_provider_agent(definition, provider)
        })?;
        Ok(())
    }

    pub fn has_model_invocable_skills(&self) -> Result<bool, ConductorError> {
        Ok(self.current_configuration()?.has_model_invocable_skills())
    }

    pub(crate) fn has_model_invocable_skills_for_execution(
        &self,
        execution_id: &ExecutionId,
    ) -> Result<bool, ConductorError> {
        Ok(self
            .configuration_for_execution(execution_id)?
            .has_model_invocable_skills())
    }

    pub fn execution_provider_kind(
        &self,
        execution_id: &ExecutionId,
    ) -> Result<ExecutionProviderKind, ConductorError> {
        let execution = self
            .executions
            .get(execution_id)
            .ok_or_else(|| ConductorError::UnknownExecution(execution_id.clone()))?;
        let configuration = self.configuration_revision(&execution.config_revision)?;
        match execution.summary.callable.as_ref() {
            None if execution.summary.kind == ExecutionKind::Root => {
                Ok(ExecutionProviderKind::Model)
            }
            Some(callable) => Ok(configuration.callables.execution_provider(callable)?.kind()),
            None => Err(ConductorError::NonProviderExecution(execution_id.clone())),
        }
    }

    pub fn resolve_invocation(
        &mut self,
        execution_id: &ExecutionId,
    ) -> Result<ResolvedInvocation, ConductorError> {
        let (summary, input) = {
            let execution = self
                .executions
                .get(execution_id)
                .ok_or_else(|| ConductorError::UnknownExecution(execution_id.clone()))?;
            if execution.summary.kind == ExecutionKind::Orchestration {
                return Err(ConductorError::NonModelExecution(execution_id.clone()));
            }
            let ExecutionPayload::Invocation { input } = &execution.payload else {
                return Err(ConductorError::NonModelExecution(execution_id.clone()));
            };
            (execution.summary.clone(), input.clone())
        };
        if self.execution_provider_kind(execution_id)? != ExecutionProviderKind::Model {
            return Err(ConductorError::NonModelExecution(execution_id.clone()));
        }

        let configuration = self.configuration_for_execution(execution_id)?.clone();
        let execution_revision = self.execution_config_revision(execution_id)?;
        let route = if let Some(route) = self.resolved_routes.get(execution_id) {
            route.clone()
        } else {
            let requested_target = summary.target.clone();
            let model = match &requested_target {
                ExecutionTarget::Fixed(model) => model.clone(),
                ExecutionTarget::Routed(profile) => configuration
                    .routing
                    .resolve(profile, summary.callable.as_ref())?,
            };
            let route = ResolvedRoute {
                requested_target,
                model,
                config_revision: execution_revision,
            };
            self.record_domain_event(DomainEvent::InvocationResolved {
                execution_id: execution_id.clone(),
                route: route.clone(),
            })?;
            route
        };

        let (prompt, explicit_skills) = self.render_model_prompt(execution_id, &input)?;
        if !explicit_skills.is_empty() {
            self.skill_activations
                .entry(execution_id.clone())
                .or_default()
                .extend(explicit_skills);
        }

        Ok(ResolvedInvocation {
            execution_id: execution_id.clone(),
            session_id: summary.session_id,
            config_revision: route.config_revision.clone(),
            callable: summary.callable,
            requested_target: route.requested_target,
            model: route.model,
            prompt,
            tools: ToolProvision {
                callables: self.permitted_tool_descriptors(execution_id)?,
            },
        })
    }

    pub fn prepare_invocation(
        &self,
        resolved: ResolvedInvocation,
        capabilities: &BackendCapabilities,
    ) -> Result<PreparedInvocation, ConductorError> {
        let execution = self
            .executions
            .get(&resolved.execution_id)
            .ok_or_else(|| ConductorError::UnknownExecution(resolved.execution_id.clone()))?;
        if execution.summary.state != ExecutionState::Pending {
            return Err(ConductorError::InvalidLifecycle(resolved.execution_id));
        }
        let tools = resolved.tools.clone().prepare(capabilities)?;
        let prepared = PreparedInvocation { resolved, tools };
        self.check_model_policy(&prepared)?;
        Ok(prepared)
    }

    pub fn drive_provider_execution(
        &mut self,
        execution_id: &ExecutionId,
    ) -> Result<(), ConductorError> {
        let (summary, input) = {
            let execution = self
                .executions
                .get(execution_id)
                .ok_or_else(|| ConductorError::UnknownExecution(execution_id.clone()))?;
            if execution.summary.state != ExecutionState::Pending {
                return Err(ConductorError::InvalidLifecycle(execution_id.clone()));
            }
            let ExecutionPayload::Invocation { input } = &execution.payload else {
                return Err(ConductorError::NonProviderExecution(execution_id.clone()));
            };
            (execution.summary.clone(), input.clone())
        };
        let callable = summary
            .callable
            .clone()
            .ok_or_else(|| ConductorError::NonProviderExecution(execution_id.clone()))?;
        let configuration = self.configuration_for_execution(execution_id)?.clone();
        let descriptor = configuration.callables.descriptor(&callable)?.clone();
        let binding = configuration
            .callables
            .execution_provider(&callable)?
            .clone();
        let Some(provider) = binding.provider().cloned() else {
            return Err(ConductorError::NonProviderExecution(execution_id.clone()));
        };
        self.check_callable_policy(
            execution_id,
            &descriptor,
            CallableOperation::DispatchProvider,
        )?;
        let config_revision = self.execution_config_revision(execution_id)?;
        let request = ExecutionProviderRequest {
            execution_id: execution_id.clone(),
            session_id: summary.session_id,
            parent_execution: summary.parent_execution,
            callable,
            config_revision,
            objective: input,
            context: self.project_execution_context(execution_id)?,
        };

        self.set_state(execution_id, ExecutionState::Running)?;
        let result = {
            let mut host = ProviderRuntimeHost {
                runtime: self,
                execution_id: execution_id.clone(),
            };
            provider.execute(&request, &mut host)
        };
        if let Err(error) = result {
            self.set_state(execution_id, ExecutionState::Failed)?;
            return Err(ConductorError::ExecutionProvider(error));
        }
        if self
            .executions
            .get(execution_id)
            .is_some_and(|execution| execution.summary.state == ExecutionState::Running)
        {
            self.set_state(execution_id, ExecutionState::Completed)?;
        }
        Ok(())
    }

    fn check_model_policy(&self, prepared: &PreparedInvocation) -> Result<(), ConductorError> {
        let context = InvocationPolicyContext {
            session_id: &prepared.resolved.session_id,
            execution_id: &prepared.resolved.execution_id,
            config_revision: &prepared.resolved.config_revision,
            subject: InvocationSubject::Model {
                invocation: prepared,
            },
        };
        self.policy
            .check(&context)
            .map_err(|denial| ConductorError::PolicyDenied {
                execution_id: prepared.resolved.execution_id.clone(),
                denial,
            })
    }
}
