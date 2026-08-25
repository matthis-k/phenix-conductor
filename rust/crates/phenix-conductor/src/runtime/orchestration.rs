    pub fn register_orchestration(
        &mut self,
        definition: OrchestrationDefinition,
    ) -> Result<(), ConductorError> {
        self.revise_configuration(move |configuration| {
            configuration.register_orchestration(definition)
        })?;
        Ok(())
    }

    #[must_use]
    pub fn attempt_groups(&self) -> Vec<AttemptGroup> {
        self.attempt_groups.values().cloned().collect()
    }

    #[must_use]
    pub fn attempt_group_for_execution(&self, execution_id: &ExecutionId) -> Option<AttemptGroup> {
        self.attempt_groups
            .values()
            .find(|group| group.contains_execution(execution_id))
            .cloned()
    }

    #[must_use]
    pub fn orchestration_failure_decisions(&self) -> Vec<OrchestrationFailureDecisionRecord> {
        self.orchestration_decisions.values().cloned().collect()
    }

    #[must_use]
    pub fn orchestration_failure_decision(
        &self,
        failed_child: &ExecutionId,
    ) -> Option<OrchestrationFailureDecisionRecord> {
        self.orchestration_decisions.get(failed_child).cloned()
    }

    fn start_agent_with_node(
        &mut self,
        parent_id: &ExecutionId,
        callable: &CallableId,
        objective: impl Into<String>,
        orchestration_node: Option<OrchestrationNodeId>,
        restrictions: Option<&ExecutionAuthority>,
    ) -> Result<ExecutionSummary, ConductorError> {
        let callables = self
            .configuration_for_execution(parent_id)?
            .callables
            .clone();
        let descriptor = callables.descriptor(callable)?.clone();
        if descriptor.kind != CallableKind::Agent {
            return Err(CallableRegistryError::WrongKind {
                callable: callable.clone(),
                expected: CallableKind::Agent,
                actual: descriptor.kind,
            }
            .into());
        }
        callables.execution_provider(callable)?;
        let operation = if orchestration_node.is_some() {
            CallableOperation::StartAgentNode
        } else {
            CallableOperation::StartAgent
        };
        self.check_callable_policy(parent_id, &descriptor, operation)?;
        let child = self.create_child(
            parent_id,
            ExecutionKind::Agent,
            callable.clone(),
            ExecutionPayload::Invocation {
                input: objective.into(),
            },
            restrictions,
        )?;
        if let Some(node_id) = orchestration_node {
            self.record_domain_event(DomainEvent::OrchestrationNodeStarted {
                execution_id: parent_id.clone(),
                node_id,
                child_execution_id: child.id.clone(),
            })?;
        }
        Ok(child)
    }

    pub fn start_orchestration(
        &mut self,
        parent_id: &ExecutionId,
        callable: &CallableId,
        input: impl Into<Value>,
    ) -> Result<ExecutionSummary, ConductorError> {
        self.start_orchestration_inner(parent_id, callable, input.into(), None)
    }

    pub fn start_orchestration_with_restrictions(
        &mut self,
        parent_id: &ExecutionId,
        callable: &CallableId,
        input: impl Into<Value>,
        restrictions: &ExecutionAuthority,
    ) -> Result<ExecutionSummary, ConductorError> {
        self.start_orchestration_inner(parent_id, callable, input.into(), Some(restrictions))
    }

    fn start_orchestration_inner(
        &mut self,
        parent_id: &ExecutionId,
        callable: &CallableId,
        input: Value,
        restrictions: Option<&ExecutionAuthority>,
    ) -> Result<ExecutionSummary, ConductorError> {
        let callables = self
            .configuration_for_execution(parent_id)?
            .callables
            .clone();
        let definition = callables.orchestration(callable)?.clone();
        validate_json_schema(&definition.descriptor.input_schema, &input).map_err(|message| {
            ConductorError::InvalidExecutionData {
                execution_id: parent_id.clone(),
                message: format!("orchestration input: {message}"),
            }
        })?;
        self.check_callable_policy(
            parent_id,
            &definition.descriptor,
            CallableOperation::StartOrchestration,
        )?;
        for step in &definition.nodes {
            let descriptor = callables.descriptor(&step.callable)?.clone();
            callables.execution_provider(&step.callable)?;
            self.check_callable_policy(parent_id, &descriptor, CallableOperation::StartAgentNode)?;
        }
        if let Some(interface_agent) = definition.interface_agent.as_ref() {
            let descriptor = callables.descriptor(interface_agent)?.clone();
            callables.execution_provider(interface_agent)?;
            self.check_callable_policy(parent_id, &descriptor, CallableOperation::StartAgentNode)?;
        }
        let summary = self.create_child(
            parent_id,
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
            .expect("orchestration exists after creation")
            .summary
            .clone())
    }

    fn ensure_orchestration_child_output(
        &mut self,
        execution_id: &ExecutionId,
    ) -> Result<(), ConductorError> {
        if self.execution_outputs.contains_key(execution_id) {
            return Ok(());
        }
        let execution = self
            .executions
            .get(execution_id)
            .ok_or_else(|| ConductorError::UnknownExecution(execution_id.clone()))?;
        let Some(parent_id) = execution.summary.parent_execution.as_ref() else {
            return Ok(());
        };
        if self
            .executions
            .get(parent_id)
            .is_none_or(|parent| parent.summary.kind != ExecutionKind::Orchestration)
        {
            return Ok(());
        }
        let content = self
            .events
            .iter()
            .filter(|event| event.execution_id == *execution_id)
            .filter_map(|event| match &event.kind {
                ExecutionEventKind::AssistantContentDelta { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<String>();
        let output = if content.trim().is_empty() {
            Value::Object(serde_json::Map::new())
        } else {
            serde_json::from_str(&content).map_err(|error| {
                ConductorError::InvalidExecutionData {
                    execution_id: execution_id.clone(),
                    message: format!("output is not valid JSON: {error}"),
                }
            })?
        };
        self.record_execution_output(execution_id, output)
    }

    fn new_attempt_group_id(&self) -> AttemptGroupId {
        AttemptGroupId::parse(format!("attempt-group-{}", self.next_attempt_group + 1))
            .expect("generated id")
    }
