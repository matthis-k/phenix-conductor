use crate::{CallableOperation, ConductorError, ConductorRuntime, DomainEvent, ExecutionPayload};
use phenix_core::{
    CallableId, ConfigRevisionId, ContextInjection, ContextInjectionLifetime,
    ContextInjectionRequester, ContextResourceId, ContextResourceKind, ContextResourceRevision,
    ContextRevision, ExecutionId, ExecutionState, SessionId, SkillInvocationPolicy,
};
use std::fmt::{self, Debug, Display, Formatter};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionProviderKind {
    Model,
    Native,
    Acp,
    RemotePhenix,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionProviderRequest {
    pub execution_id: ExecutionId,
    pub session_id: SessionId,
    pub parent_execution: Option<ExecutionId>,
    pub callable: CallableId,
    pub config_revision: ConfigRevisionId,
    pub objective: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExecutionProviderEvent {
    ReasoningDelta(String),
    ContentDelta(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExecutionProviderError {
    Unsupported(String),
    Failed(String),
    Protocol(String),
}

impl Display for ExecutionProviderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(message) => {
                write!(f, "unsupported execution provider capability: {message}")
            }
            Self::Failed(message) => write!(f, "execution provider failed: {message}"),
            Self::Protocol(message) => write!(f, "execution provider protocol error: {message}"),
        }
    }
}

impl std::error::Error for ExecutionProviderError {}

pub trait ExecutionProviderHost {
    fn emit(&mut self, event: ExecutionProviderEvent) -> Result<(), ExecutionProviderError>;
}

pub trait ExecutionProvider: Send + Sync {
    fn kind(&self) -> ExecutionProviderKind;

    fn execute(
        &self,
        request: &ExecutionProviderRequest,
        host: &mut dyn ExecutionProviderHost,
    ) -> Result<(), ExecutionProviderError>;

    fn cancel(&self, _execution_id: &ExecutionId) -> Result<(), ExecutionProviderError> {
        Err(ExecutionProviderError::Unsupported(
            "provider does not implement cancellation".to_owned(),
        ))
    }
}

#[derive(Clone)]
pub enum ExecutionProviderBinding {
    Model,
    Provider(Arc<dyn ExecutionProvider>),
}

impl ExecutionProviderBinding {
    #[must_use]
    pub fn kind(&self) -> ExecutionProviderKind {
        match self {
            Self::Model => ExecutionProviderKind::Model,
            Self::Provider(provider) => provider.kind(),
        }
    }

    #[must_use]
    pub fn provider(&self) -> Option<&Arc<dyn ExecutionProvider>> {
        match self {
            Self::Model => None,
            Self::Provider(provider) => Some(provider),
        }
    }
}

impl Debug for ExecutionProviderBinding {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExecutionProviderBinding")
            .field("kind", &self.kind())
            .finish_non_exhaustive()
    }
}

impl ConductorRuntime {
    /// Resolve the immutable provider dispatch inputs while the caller holds
    /// the runtime lock. The returned provider/request pair is then safe to
    /// execute after releasing that lock, which keeps frontend cancellation
    /// and event delivery responsive during a blocking provider call.
    pub(crate) fn prepare_provider_execution(
        &self,
        execution_id: &ExecutionId,
    ) -> Result<(Arc<dyn ExecutionProvider>, ExecutionProviderRequest), ConductorError> {
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
        let configuration = self.configuration_for_execution(execution_id)?;
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
        };
        Ok((provider, request))
    }

    pub fn load_context_for_execution(
        &mut self,
        execution_id: &ExecutionId,
        resource_id: &ContextResourceId,
        requested_revision: &ContextRevision,
        requested_by: ContextInjectionRequester,
        lifetime: ContextInjectionLifetime,
        reason: impl Into<String>,
    ) -> Result<(ContextResourceRevision, ContextInjection), ConductorError> {
        if lifetime == ContextInjectionLifetime::Objective
            && self.execution_objectives(execution_id)?.is_none()
        {
            return Err(
                crate::ObjectiveError::MissingExecutionObjective(execution_id.clone()).into(),
            );
        }
        let configuration = self.configuration_for_execution(execution_id)?;
        let resource = configuration
            .context_catalog()
            .resolve_revision(resource_id, requested_revision)
            .map_err(|error| ConductorError::InvalidExecutionData {
                execution_id: execution_id.clone(),
                message: error.to_string(),
            })?
            .clone();
        if requested_by != ContextInjectionRequester::User
            && resource.descriptor.kind == ContextResourceKind::Skill
        {
            let skill = configuration
                .skill_descriptors()
                .into_iter()
                .find(|skill| {
                    resource.descriptor.id.as_str() == format!("skill:{}", skill.id.as_str())
                })
                .ok_or_else(|| ConductorError::InvalidExecutionData {
                    execution_id: execution_id.clone(),
                    message: format!(
                        "skill context resource {} has no configured skill descriptor",
                        resource.descriptor.id
                    ),
                })?;
            if skill.invocation == SkillInvocationPolicy::ManualOnly {
                return Err(crate::ContextError::ManualOnlySkill(skill.id).into());
            }
        }
        let injection = ContextInjection {
            execution_id: execution_id.clone(),
            source_ref: resource.source_ref.clone(),
            source_revision: resource.descriptor.revision.clone(),
            requested_by,
            reason: reason.into(),
            lifetime,
            content_identity: resource.content_identity.clone(),
        };
        self.record_domain_event(DomainEvent::ContextInjectionRecorded {
            injection: injection.clone(),
        })?;
        Ok((resource, injection))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CompiledConfiguration, ContextRegistry, SkillRegistry};
    use phenix_core::{
        BackendId, ExecutionTarget, InferenceOptions, ModelId, ModelTarget, ProviderId,
    };
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture_root() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("phenix-context-injection-journal-{nonce}"))
    }

    fn write(path: impl AsRef<Path>, content: &str) {
        let path = path.as_ref();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn fixed_target() -> ExecutionTarget {
        ExecutionTarget::Fixed(ModelTarget {
            backend: BackendId::parse("mock").unwrap(),
            provider: ProviderId::parse("mock").unwrap(),
            model: ModelId::parse("mock").unwrap(),
            inference: InferenceOptions::default(),
        })
    }

    #[test]
    fn context_load_records_durable_injection_history() {
        let root = fixture_root();
        fs::create_dir_all(root.join(".git")).unwrap();
        write(root.join("CONTRIBUTING.md"), "exact project context");

        let mut configuration = CompiledConfiguration::default();
        configuration.install_context_registry(ContextRegistry::discover(&root).unwrap());
        configuration.install_skill_registry(SkillRegistry::discover(&root).unwrap());

        let mut runtime = ConductorRuntime::new();
        runtime.reload_configuration(configuration).unwrap();
        let session = runtime.create_session(None, None, fixed_target()).unwrap();
        let execution = runtime.submit(&session.id, "load project context").unwrap();
        let id = ContextResourceId::parse("project-document:CONTRIBUTING.md").unwrap();
        let descriptor = runtime
            .context_descriptors_for_execution(&execution.id)
            .unwrap()
            .into_iter()
            .find(|descriptor| descriptor.id == id)
            .unwrap();

        runtime
            .load_context_for_execution(
                &execution.id,
                &id,
                &descriptor.revision,
                ContextInjectionRequester::Agent,
                ContextInjectionLifetime::SingleRequest,
                "agent requested exact project context",
            )
            .unwrap();

        let durable = serde_json::to_string(&runtime.journal).unwrap();
        assert!(
            durable.contains("agent requested exact project context"),
            "context load must record its canonical injection in durable journal history"
        );

        fs::remove_dir_all(root).unwrap();
    }
}
