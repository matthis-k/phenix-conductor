mod worker_profiles {
    include!("../worker_profiles.rs");
}

pub use worker_profiles::{
    ResolvedWorkerProfile, WorkerProfileDefinition, WorkerProfileError, WorkerProfileId,
};
use worker_profiles::WorkerProfileRegistry;

#[derive(Clone, Debug, Default)]
pub struct CompiledConfiguration {
    callables: CallableRegistry,
    routing: RoutingRegistry,
    context: ContextRegistry,
    skills: SkillRegistry,
    worker_profiles: WorkerProfileRegistry,
}

impl CompiledConfiguration {
    fn fingerprint(&self) -> ConfigRevisionFingerprint {
        let manifest = json!({
            "callables": self.callables.semantic_manifest(),
            "routing": self.routing.semantic_manifest(),
            "context": self.context.semantic_manifest(),
            "skills": self.skills.semantic_manifest(),
            "worker_profiles": self.worker_profiles.semantic_manifest(),
        });
        let encoded = serde_json::to_vec(&manifest)
            .expect("compiled configuration manifest is JSON serializable");
        let digest = Sha256::digest(encoded);
        let encoded = digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        ConfigRevisionFingerprint(encoded)
    }

    fn context_catalog(&self) -> phenix_core::ContextCatalog {
        let mut catalog = self
            .context
            .project_context_catalog()
            .expect("project context catalog construction must preserve immutable revisions");
        let skill_catalog = self
            .skills
            .skill_context_catalog()
            .expect("skill context catalog construction must preserve immutable revisions");
        for descriptor in skill_catalog.descriptors() {
            let revision = skill_catalog
                .resolve_revision(&descriptor.id, &descriptor.revision)
                .expect("catalog descriptor must resolve to its exact revision")
                .clone();
            catalog
                .register_revision(revision)
                .expect("project and skill context identities must not conflict");
        }
        catalog
    }

    pub fn register_tool<F, O>(
        &mut self,
        descriptor: CallableDescriptor,
        handler: F,
    ) -> Result<(), ConductorError>
    where
        F: Fn(&str) -> Result<O, String> + Send + Sync + 'static,
        O: Into<ToolOutcome> + 'static,
    {
        self.callables.register_tool(descriptor, handler)?;
        Ok(())
    }

    pub(crate) fn register_contextual_tool<F, O>(
        &mut self,
        descriptor: CallableDescriptor,
        handler: F,
    ) -> Result<(), ConductorError>
    where
        F: Fn(&callables::ToolExecutionContext, &str) -> Result<O, String> + Send + Sync + 'static,
        O: Into<ToolOutcome> + 'static,
    {
        self.callables
            .register_contextual_tool(descriptor, handler)?;
        Ok(())
    }

    pub fn register_agent(&mut self, definition: AgentDefinition) -> Result<(), ConductorError> {
        self.callables.register_agent(definition)?;
        Ok(())
    }

    pub fn register_provider_agent<P>(
        &mut self,
        definition: AgentDefinition,
        provider: P,
    ) -> Result<(), ConductorError>
    where
        P: ExecutionProvider + 'static,
    {
        self.callables
            .register_provider_agent(definition, provider)?;
        Ok(())
    }

    pub fn register_orchestration(
        &mut self,
        definition: OrchestrationDefinition,
    ) -> Result<(), ConductorError> {
        self.callables.register_orchestration(definition)?;
        Ok(())
    }

    pub fn register_routing_profile(
        &mut self,
        profile: RoutingProfile,
    ) -> Result<(), ConductorError> {
        self.routing.register(profile)?;
        Ok(())
    }

    pub fn register_worker_profile(
        &mut self,
        profile: WorkerProfileDefinition,
    ) -> Result<(), WorkerProfileError> {
        if self.callables.agent_definition(&profile.agent).is_err() {
            return Err(WorkerProfileError::InvalidAgent {
                profile: profile.id,
                agent: profile.agent,
            });
        }
        self.worker_profiles.register(profile)
    }

    pub fn worker_profile(
        &self,
        id: &WorkerProfileId,
    ) -> Result<&WorkerProfileDefinition, WorkerProfileError> {
        self.worker_profiles.get(id)
    }

    pub fn resolve_worker_profile(
        &self,
        id: &WorkerProfileId,
    ) -> Result<ResolvedWorkerProfile<'_>, WorkerProfileError> {
        let profile = self.worker_profiles.get(id)?;
        let agent = self
            .callables
            .agent_definition(&profile.agent)
            .map_err(|_| WorkerProfileError::InvalidAgent {
                profile: profile.id.clone(),
                agent: profile.agent.clone(),
            })?;
        Ok(ResolvedWorkerProfile { profile, agent })
    }

    pub fn install_context_registry(&mut self, context: ContextRegistry) {
        self.context = context;
    }

    pub fn install_skill_registry(&mut self, skills: SkillRegistry) {
        self.skills = skills;
    }

    #[must_use]
    pub fn callable_descriptors(&self) -> Vec<CallableDescriptor> {
        self.callables.descriptors()
    }

    #[must_use]
    pub fn tool_descriptors(&self) -> Vec<CallableDescriptor> {
        self.callables.tool_descriptors()
    }

    #[must_use]
    pub fn routing_profiles(&self) -> Vec<RoutingProfileDescriptor> {
        self.routing.descriptors()
    }

    #[must_use]
    pub fn skill_descriptors(&self) -> Vec<SkillDescriptor> {
        self.skills.skill_descriptors()
    }

    #[must_use]
    pub fn has_model_invocable_skills(&self) -> bool {
        self.skills.has_model_invocable_skills()
    }

    #[must_use]
    pub fn has_skills(&self) -> bool {
        self.skills.has_skills()
    }
}
