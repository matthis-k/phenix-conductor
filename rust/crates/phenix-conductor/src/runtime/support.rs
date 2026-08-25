impl ConductorRuntime {
    pub fn register_agent(&mut self, definition: AgentDefinition) -> Result<(), ConductorError> {
        self.revise_configuration(move |configuration| configuration.register_agent(definition))?;
        Ok(())
    }

    pub fn register_worker_profile(
        &mut self,
        profile: WorkerProfileDefinition,
    ) -> Result<(), ConductorError> {
        self.revise_configuration(move |configuration| {
            configuration.register_worker_profile(profile)?;
            Ok(())
        })?;
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

    pub fn promote_text_artifact(
        &mut self,
        execution_id: &ExecutionId,
        title: impl Into<String>,
        content: impl Into<String>,
    ) -> Result<phenix_core::ContextResourceRevision, ConductorError> {
        self.execution_authority(execution_id)?;
        let content = content.into();
        let digest = format!("{:x}", Sha256::digest(content.as_bytes()));
        let revision = ContextRevision::parse(format!("sha256:{digest}"))
            .expect("sha256 artifact revision must be a valid context revision");
        let id = ContextResourceId::parse(format!("artifact:{execution_id}:{digest}"))
            .expect("generated artifact id must be a valid context resource id");
        let source_ref = phenix_core::ExactReference::Context {
            resource_id: id.clone(),
            revision: revision.clone(),
        };

        if let Some(existing) = self.journal.entries.iter().find_map(|entry| match &entry.event {
            DomainEvent::ContextResourceRevisionRegistered { resource }
                if resource.source_ref == source_ref =>
            {
                Some(resource.clone())
            }
            _ => None,
        }) {
            return Ok(existing);
        }

        let resource = phenix_core::ContextResourceRevision {
            descriptor: ContextDescriptor {
                id,
                kind: ContextResourceKind::Artifact,
                title: title.into(),
                description: "Immutable execution artifact".to_owned(),
                scope: ContextScope::Execution {
                    execution_id: execution_id.clone(),
                },
                revision: revision.clone(),
                estimated_cost: source_ref.to_string().len() as u64,
            },
            tier: phenix_core::ContextTier::DiscoverableContent,
            source_ref,
            content_identity: revision,
            content: Some(content),
        };
        self.record_domain_event(DomainEvent::ContextResourceRevisionRegistered {
            resource: resource.clone(),
        })?;
        Ok(resource)
    }

    fn next_language_observation_id(&self) -> LanguageObservationId {
        let mut ordinal = self
            .journal
            .entries
            .iter()
            .filter(|entry| matches!(entry.event, DomainEvent::LanguageObservationRecorded { .. }))
            .count()
            + 1;
        loop {
            let candidate = LanguageObservationId::parse(format!("language-observation-{ordinal}"))
                .expect("generated language observation id");
            let exists = self.journal.entries.iter().any(|entry| {
                matches!(
                    &entry.event,
                    DomainEvent::LanguageObservationRecorded { observation }
                        if observation.id == candidate
                )
            });
            if !exists {
                return candidate;
            }
            ordinal += 1;
        }
    }

    pub fn record_language_observation(
        &mut self,
        observation: LanguageObservationInput,
    ) -> Result<LanguageObservation, ConductorError> {
        let observation = LanguageObservation {
            id: self.next_language_observation_id(),
            execution: observation.execution,
            workspace: observation.workspace,
            service: observation.service,
            provider: observation.provider,
            provider_epoch: observation.provider_epoch,
            operation: observation.operation,
            result: observation.result,
        };
        self.record_domain_event(DomainEvent::LanguageObservationRecorded {
            observation: observation.clone(),
        })?;
        Ok(observation)
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

    pub fn start_worker_profile(
        &mut self,
        parent_id: &ExecutionId,
        profile_id: &WorkerProfileId,
        objective: impl Into<String>,
    ) -> Result<ExecutionSummary, ConductorError> {
        self.start_worker_profile_inner(parent_id, profile_id, objective.into(), None)
    }

    pub fn start_worker_profile_with_restrictions(
        &mut self,
        parent_id: &ExecutionId,
        profile_id: &WorkerProfileId,
        objective: impl Into<String>,
        restrictions: &ExecutionAuthority,
    ) -> Result<ExecutionSummary, ConductorError> {
        self.start_worker_profile_inner(
            parent_id,
            profile_id,
            objective.into(),
            Some(restrictions),
        )
    }

    fn start_worker_profile_inner(
        &mut self,
        parent_id: &ExecutionId,
        profile_id: &WorkerProfileId,
        objective: String,
        restrictions: Option<&ExecutionAuthority>,
    ) -> Result<ExecutionSummary, ConductorError> {
        let (callable, profile_maximum) = {
            let resolved = self
                .configuration_for_execution(parent_id)?
                .resolve_worker_profile(profile_id)?;
            (
                resolved.profile.agent.clone(),
                resolved.profile.authority_maximum.clone(),
            )
        };
        let effective_restrictions = restrictions.map_or(profile_maximum.clone(), |requested| {
            profile_maximum.attenuate(requested)
        });
        let child = self.start_agent_with_node(
            parent_id,
            &callable,
            objective,
            None,
            Some(&effective_restrictions),
        )?;
        self.record_domain_event(DomainEvent::WorkerProfileBound {
            execution_id: child.id.clone(),
            profile_id: profile_id.clone(),
        })?;
        Ok(child)
    }
}

#[cfg(test)]
mod artifact_tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixed_target() -> ExecutionTarget {
        ExecutionTarget::Fixed(ModelTarget {
            backend: phenix_core::BackendId::parse("mock").unwrap(),
            provider: phenix_core::ProviderId::parse("mock").unwrap(),
            model: phenix_core::ModelId::parse("mock").unwrap(),
            inference: phenix_core::InferenceOptions::default(),
        })
    }

    #[test]
    fn promoted_artifact_is_immutable_and_survives_relational_restore() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("phenix-durable-artifacts-{nonce}"));
        fs::create_dir_all(&root).unwrap();

        let mut runtime = ConductorRuntime::new();
        let session = runtime.create_session(None, None, fixed_target()).unwrap();
        let execution = runtime.submit(&session.id, "produce artifact").unwrap();
        let artifact = runtime
            .promote_text_artifact(&execution.id, "build log", "exact build output")
            .unwrap();

        let duplicate = runtime
            .promote_text_artifact(
                &execution.id,
                "ignored replacement title",
                "exact build output",
            )
            .unwrap();
        assert_eq!(duplicate, artifact);

        let store = SqliteStore::new(root.join("state.sqlite"));
        store.save(runtime.journal()).unwrap();
        let restored = ConductorRuntime::restore(store.load().unwrap()).unwrap();
        let resolved = restored
            .resolve_exact_reference(&artifact.source_ref)
            .unwrap();
        assert_eq!(resolved, ResolvedExactReference::Context(artifact.clone()));
        assert_eq!(artifact.content.as_deref(), Some("exact build output"));

        fs::remove_dir_all(root).unwrap();
    }
}
