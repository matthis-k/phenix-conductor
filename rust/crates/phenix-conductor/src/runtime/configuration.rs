impl ConductorRuntime {
    fn current_configuration(&self) -> Result<&CompiledConfiguration, ConductorError> {
        self.configuration_revision(&self.config_revision)
    }

    pub fn current_compiled_configuration(&self) -> Result<CompiledConfiguration, ConductorError> {
        Ok(self.current_configuration()?.clone())
    }

    pub(crate) fn configuration_revision(
        &self,
        revision: &ConfigRevisionId,
    ) -> Result<&CompiledConfiguration, ConductorError> {
        self.config_revisions
            .get(revision)
            .ok_or_else(|| ConductorError::UnknownConfigRevision(revision.clone()))?
            .configuration
            .as_ref()
            .ok_or_else(|| ConductorError::UnboundConfigRevision(revision.clone()))
    }

    pub(crate) fn configuration_for_session(
        &self,
        session_id: &SessionId,
    ) -> Result<&CompiledConfiguration, ConductorError> {
        let revision = &self
            .sessions
            .get(session_id)
            .ok_or_else(|| ConductorError::UnknownSession(session_id.clone()))?
            .summary
            .config_revision;
        self.configuration_revision(revision)
    }

    pub(crate) fn configuration_for_execution(
        &self,
        execution_id: &ExecutionId,
    ) -> Result<&CompiledConfiguration, ConductorError> {
        let revision = &self
            .executions
            .get(execution_id)
            .ok_or_else(|| ConductorError::UnknownExecution(execution_id.clone()))?
            .config_revision;
        self.configuration_revision(revision)
    }

    #[must_use]
    pub fn current_config_revision(&self) -> &ConfigRevisionId {
        &self.config_revision
    }

    pub fn execution_config_revision(
        &self,
        execution_id: &ExecutionId,
    ) -> Result<ConfigRevisionId, ConductorError> {
        self.executions
            .get(execution_id)
            .map(|execution| execution.config_revision.clone())
            .ok_or_else(|| ConductorError::UnknownExecution(execution_id.clone()))
    }

    pub fn bind_configuration_revision(
        &mut self,
        revision: &ConfigRevisionId,
        configuration: CompiledConfiguration,
    ) -> Result<(), ConductorError> {
        let slot = self
            .config_revisions
            .get_mut(revision)
            .ok_or_else(|| ConductorError::UnknownConfigRevision(revision.clone()))?;
        if slot.configuration.is_some() {
            return Err(ConductorError::ConfigRevisionAlreadyBound(revision.clone()));
        }
        let actual = configuration.fingerprint();
        if actual != slot.fingerprint {
            return Err(ConductorError::ConfigRevisionFingerprintMismatch {
                revision: revision.clone(),
                expected: slot.fingerprint.clone(),
                actual,
            });
        }
        slot.configuration = Some(configuration);
        Ok(())
    }

    pub fn bind_available_configurations(
        &mut self,
        configurations: &[CompiledConfiguration],
    ) -> Result<Vec<ConfigRevisionId>, ConductorError> {
        let available = configurations
            .iter()
            .map(|configuration| (configuration.fingerprint(), configuration))
            .collect::<BTreeMap<_, _>>();
        let bindings = self
            .config_revisions
            .iter()
            .filter(|(_, slot)| slot.configuration.is_none())
            .filter_map(|(revision, slot)| {
                available
                    .get(&slot.fingerprint)
                    .map(|configuration| (revision.clone(), (*configuration).clone()))
            })
            .collect::<Vec<_>>();
        let mut bound = Vec::with_capacity(bindings.len());
        for (revision, configuration) in bindings {
            self.bind_configuration_revision(&revision, configuration)?;
            bound.push(revision);
        }
        Ok(bound)
    }

    pub fn activate_configuration(
        &mut self,
        configuration: CompiledConfiguration,
    ) -> Result<ConfigRevisionId, ConductorError> {
        let fingerprint = configuration.fingerprint();
        let current = self
            .config_revisions
            .get(&self.config_revision)
            .expect("current configuration revision exists");
        if current.fingerprint == fingerprint {
            let revision = self.config_revision.clone();
            if current.configuration.is_none() {
                self.bind_configuration_revision(&revision, configuration)?;
            }
            return Ok(revision);
        }
        self.reload_configuration(configuration)
    }

    #[must_use]
    pub fn required_config_revisions(&self) -> BTreeSet<ConfigRevisionId> {
        let mut revisions = BTreeSet::from([self.config_revision.clone()]);
        revisions.extend(
            self.sessions
                .values()
                .map(|session| session.summary.config_revision.clone()),
        );
        revisions.extend(
            self.executions
                .values()
                .map(|execution| execution.config_revision.clone()),
        );
        revisions
    }

    pub fn ensure_required_configurations_bound(&self) -> Result<(), ConductorError> {
        for revision in self.required_config_revisions() {
            self.configuration_revision(&revision)?;
        }
        Ok(())
    }

    pub fn reload_configuration(
        &mut self,
        configuration: CompiledConfiguration,
    ) -> Result<ConfigRevisionId, ConductorError> {
        let revision = self.new_config_revision_id();
        let fingerprint = configuration.fingerprint();
        self.record_domain_event(DomainEvent::ConfigurationRevisionActivated {
            revision: revision.clone(),
            fingerprint,
        })?;
        let slot = self
            .config_revisions
            .get_mut(&revision)
            .expect("configuration activation creates a revision slot");
        slot.configuration = Some(configuration);
        Ok(revision)
    }

    fn revise_configuration<F>(&mut self, update: F) -> Result<ConfigRevisionId, ConductorError>
    where
        F: FnOnce(&mut CompiledConfiguration) -> Result<(), ConductorError>,
    {
        let mut configuration = self.current_configuration()?.clone();
        update(&mut configuration)?;
        self.reload_configuration(configuration)
    }

    pub fn skill_descriptors(&self) -> Result<Vec<SkillDescriptor>, ConductorError> {
        Ok(self.current_configuration()?.skill_descriptors())
    }

    pub fn callable_descriptors(&self) -> Result<Vec<CallableDescriptor>, ConductorError> {
        Ok(self.current_configuration()?.callable_descriptors())
    }

    pub fn routing_profiles(&self) -> Result<Vec<RoutingProfileDescriptor>, ConductorError> {
        Ok(self.current_configuration()?.routing_profiles())
    }

    pub(crate) fn callable_descriptors_for_execution(
        &self,
        execution_id: &ExecutionId,
    ) -> Result<Vec<CallableDescriptor>, ConductorError> {
        Ok(self
            .configuration_for_execution(execution_id)?
            .callable_descriptors())
    }

    fn configured_authority_for_execution(
        &self,
        execution: &ExecutionSummary,
    ) -> Result<ExecutionAuthority, ConductorError> {
        let revision = if let Some(parent_id) = execution.parent_execution.as_ref() {
            self.executions
                .get(parent_id)
                .ok_or_else(|| ConductorError::UnknownExecution(parent_id.clone()))?
                .config_revision
                .clone()
        } else {
            self.sessions
                .get(&execution.session_id)
                .ok_or_else(|| ConductorError::UnknownSession(execution.session_id.clone()))?
                .summary
                .config_revision
                .clone()
        };
        let callables = &self.configuration_revision(&revision)?.callables;
        match execution.kind {
            ExecutionKind::Root => {
                let mut authority = authority_envelope(
                    callables
                        .agent_definitions()
                        .map(|definition| &definition.authority),
                );
                authority.callables.extend(
                    callables
                        .descriptors()
                        .into_iter()
                        .filter(|descriptor| {
                            matches!(
                                descriptor.kind,
                                CallableKind::Agent | CallableKind::Orchestration
                            )
                        })
                        .map(|descriptor| descriptor.id),
                );
                Ok(authority)
            }
            ExecutionKind::Agent => {
                let Some(callable) = execution.callable.as_ref() else {
                    return Ok(ExecutionAuthority::read_only());
                };
                Ok(callables.agent_definition(callable)?.authority.clone())
            }
            ExecutionKind::Orchestration => {
                let Some(callable) = execution.callable.as_ref() else {
                    return Ok(ExecutionAuthority::read_only());
                };
                let definition = callables.orchestration(callable)?;
                let mut authorities = definition
                    .nodes
                    .iter()
                    .map(|node| {
                        callables
                            .agent_definition(&node.callable)
                            .map(|definition| &definition.authority)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                if let Some(interface_agent) = definition.interface_agent.as_ref() {
                    authorities.push(&callables.agent_definition(interface_agent)?.authority);
                }
                let mut authority = authority_envelope(authorities);
                authority
                    .callables
                    .extend(definition.nodes.iter().map(|node| node.callable.clone()));
                if let Some(interface_agent) = definition.interface_agent.as_ref() {
                    authority.callables.insert(interface_agent.clone());
                }
                Ok(authority)
            }
        }
    }

    fn create_session_at_revision(
        &mut self,
        parent_session: Option<SessionId>,
        name: Option<String>,
        target: ExecutionTarget,
        revision: ConfigRevisionId,
    ) -> Result<SessionSummary, ConductorError> {
        self.configuration_revision(&revision)?;
        let workspace_id = if let Some(parent) = parent_session.as_ref() {
            self.sessions
                .get(parent)
                .ok_or_else(|| ConductorError::UnknownSession(parent.clone()))?
                .summary
                .workspace_id
                .clone()
        } else {
            self.workspace_id.clone()
        };
        let summary = SessionSummary {
            id: self.new_session_id(),
            parent_session,
            name,
            workspace_id,
            config_revision: revision,
            default_target: target,
            state: SessionState::Active,
        };
        self.record_domain_event(DomainEvent::SessionCreated {
            session: summary.clone(),
        })?;
        Ok(summary)
    }

    fn new_config_revision_id(&self) -> ConfigRevisionId {
        ConfigRevisionId::parse(format!("config-{}", self.next_config_revision + 1))
            .expect("generated config revision id")
    }
}
