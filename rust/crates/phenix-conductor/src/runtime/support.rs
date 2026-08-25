    pub fn register_agent(&mut self, definition: AgentDefinition) -> Result<(), ConductorError> {
        self.revise_configuration(move |configuration| configuration.register_agent(definition))?;
        Ok(())
    }

    pub fn register_routing_profile(
        &mut self,
        profile: RoutingProfile,
    ) -> Result<(), ConductorError> {
        self.revise_configuration(move |configuration| {
            configuration.register_routing_profile(profile)
        })?;
        Ok(())
    }

    pub fn install_context_registry(
        &mut self,
        context: ContextRegistry,
    ) -> Result<(), ConductorError> {
        self.revise_configuration(move |configuration| {
            configuration.install_context_registry(context);
            Ok(())
        })?;
        Ok(())
    }

    pub fn install_skill_registry(&mut self, skills: SkillRegistry) -> Result<(), ConductorError> {
        self.revise_configuration(move |configuration| {
            configuration.install_skill_registry(skills);
            Ok(())
        })?;
        Ok(())
    }

    pub fn has_skills(&self) -> Result<bool, ConductorError> {
        Ok(self.current_configuration()?.has_skills())
    }

    pub fn load_skill(
        &mut self,
        execution_id: &ExecutionId,
        id: &SkillId,
    ) -> Result<String, ConductorError> {
        let payload = self
            .configuration_for_execution(execution_id)?
            .skills
            .model_skill_payload(id)?;
        self.skill_activations
            .entry(execution_id.clone())
            .or_default()
            .insert(id.clone());
        Ok(payload)
    }

    pub fn read_skill_resource(
        &self,
        execution_id: &ExecutionId,
        id: &SkillId,
        path: &str,
    ) -> Result<String, ConductorError> {
        if !self
            .skill_activations
            .get(execution_id)
            .is_some_and(|skills| skills.contains(id))
        {
            return Err(ContextError::InactiveSkill(id.clone()).into());
        }
        Ok(self
            .configuration_for_execution(execution_id)?
            .skills
            .skill_resource_payload(id, path)?)
    }

    pub fn record_language_observation(
        &mut self,
        observation: LanguageObservation,
    ) -> Result<(), ConductorError> {
        self.record_domain_event(DomainEvent::LanguageObservationRecorded { observation })
    }

    pub fn start_agent(
        &mut self,
        parent_id: &ExecutionId,
        callable: &CallableId,
        objective: impl Into<String>,
    ) -> Result<ExecutionSummary, ConductorError> {
        self.start_agent_with_node(parent_id, callable, objective, None, None)
    }

    pub fn start_agent_with_restrictions(
        &mut self,
        parent_id: &ExecutionId,
        callable: &CallableId,
        objective: impl Into<String>,
        restrictions: &ExecutionAuthority,
    ) -> Result<ExecutionSummary, ConductorError> {
        self.start_agent_with_node(parent_id, callable, objective, None, Some(restrictions))
    }
