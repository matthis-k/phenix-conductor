use crate::{
    CallableOperation, ConductorError, ConductorRuntime, DomainEvent, ExecutionContextProjection,
    ExecutionPayload, JournalEntry,
};
use phenix_core::{
    CallableId, ConfigRevisionId, ContextInjection, ContextInjectionLifetime,
    ContextInjectionRequester, ContextResourceId, ContextResourceKind, ContextResourceRevision,
    ContextRevision, ContextScope, ContextTier, ExactReference, ExecutionId, ExecutionState,
    ExecutionSummary, FileObservation, LanguageObservation, ObjectiveRecord, PlanRecord, SessionId,
    SkillInvocationPolicy,
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
    pub context: ExecutionContextProjection,
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

#[derive(Clone, Debug, PartialEq)]
pub enum ResolvedExactReference {
    Objective(ObjectiveRecord),
    Plan(PlanRecord),
    Execution(ExecutionSummary),
    Event(JournalEntry),
    FileObservation(FileObservation),
    LanguageObservation(LanguageObservation),
    Context(ContextResourceRevision),
}

impl ResolvedExactReference {
    #[must_use]
    pub fn objective(&self) -> Option<&ObjectiveRecord> {
        match self {
            Self::Objective(objective) => Some(objective),
            _ => None,
        }
    }

    #[must_use]
    pub fn plan(&self) -> Option<&PlanRecord> {
        match self {
            Self::Plan(plan) => Some(plan),
            _ => None,
        }
    }

    #[must_use]
    pub fn execution(&self) -> Option<&ExecutionSummary> {
        match self {
            Self::Execution(execution) => Some(execution),
            _ => None,
        }
    }

    #[must_use]
    pub fn event(&self) -> Option<&JournalEntry> {
        match self {
            Self::Event(event) => Some(event),
            _ => None,
        }
    }

    #[must_use]
    pub fn file_observation(&self) -> Option<&FileObservation> {
        match self {
            Self::FileObservation(observation) => Some(observation),
            _ => None,
        }
    }

    #[must_use]
    pub fn language_observation(&self) -> Option<&LanguageObservation> {
        match self {
            Self::LanguageObservation(observation) => Some(observation),
            _ => None,
        }
    }

    #[must_use]
    pub fn context_resource(&self) -> Option<&ContextResourceRevision> {
        match self {
            Self::Context(resource) => Some(resource),
            _ => None,
        }
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
            context: self.project_execution_context(execution_id)?,
        };
        Ok((provider, request))
    }

    fn ensure_context_scope_for_execution(
        &self,
        execution_id: &ExecutionId,
        scope: &ContextScope,
    ) -> Result<(), ConductorError> {
        let execution = self
            .executions
            .get(execution_id)
            .ok_or_else(|| ConductorError::UnknownExecution(execution_id.clone()))?;
        let session_id = execution.summary.session_id.clone();
        let config_revision = execution.config_revision.clone();
        let workspace_id = self
            .sessions
            .get(&session_id)
            .ok_or_else(|| ConductorError::UnknownSession(session_id.clone()))?
            .summary
            .workspace_id
            .clone();

        let allowed = match scope {
            ContextScope::Workspace {
                workspace_id: scoped_workspace,
            } => scoped_workspace == &workspace_id,
            ContextScope::Execution {
                execution_id: scoped_execution,
            } => scoped_execution == execution_id,
            ContextScope::Objective { objective_id } => self
                .execution_objectives(execution_id)?
                .is_some_and(|assignment| {
                    assignment.primary == *objective_id
                        || assignment.supporting.contains(objective_id)
                }),
            ContextScope::Path { .. } => true,
            ContextScope::Configuration { revision } => revision == &config_revision,
        };
        if allowed {
            Ok(())
        } else {
            Err(ConductorError::InvalidExecutionData {
                execution_id: execution_id.clone(),
                message: format!("context resource scope does not apply to execution: {scope:?}"),
            })
        }
    }

    fn registered_context_resource(
        &self,
        resource_id: &ContextResourceId,
        revision: &ContextRevision,
    ) -> Option<&ContextResourceRevision> {
        self.journal
            .entries
            .iter()
            .rev()
            .find_map(|entry| match &entry.event {
                DomainEvent::ContextResourceRevisionRegistered { resource }
                    if resource.descriptor.id == *resource_id
                        && resource.descriptor.revision == *revision =>
                {
                    Some(resource)
                }
                _ => None,
            })
    }

    fn register_context_resource_revision(
        &mut self,
        resource: ContextResourceRevision,
    ) -> Result<(), ConductorError> {
        if let Some(existing) =
            self.registered_context_resource(&resource.descriptor.id, &resource.descriptor.revision)
        {
            let same_identity = existing.descriptor == resource.descriptor
                && existing.tier == resource.tier
                && existing.source_ref == resource.source_ref
                && existing.content_identity == resource.content_identity;
            let compatible_content = existing.content == resource.content
                || existing.content.is_none()
                || resource.content.is_none();
            if !same_identity || !compatible_content {
                return Err(crate::JournalError::InvalidEvent(format!(
                    "context resource revision changed after durable registration: {}@{}",
                    resource.descriptor.id, resource.descriptor.revision
                ))
                .into());
            }
            return Ok(());
        }
        self.record_domain_event(DomainEvent::ContextResourceRevisionRegistered { resource })
    }

    pub fn resolve_exact_reference(
        &self,
        reference: &ExactReference,
    ) -> Result<ResolvedExactReference, ConductorError> {
        match reference {
            ExactReference::Objective(objective_id) => Ok(ResolvedExactReference::Objective(
                self.objective(objective_id)?,
            )),
            ExactReference::Plan(plan_id) => Ok(ResolvedExactReference::Plan(self.plan(plan_id)?)),
            ExactReference::Execution(execution_id) => self
                .executions
                .get(execution_id)
                .map(|record| ResolvedExactReference::Execution(record.summary.clone()))
                .ok_or_else(|| ConductorError::UnknownExecution(execution_id.clone())),
            ExactReference::Event(sequence) => self
                .journal
                .entries
                .iter()
                .find(|entry| entry.sequence == *sequence)
                .cloned()
                .map(ResolvedExactReference::Event)
                .ok_or_else(|| {
                    crate::JournalError::InvalidEvent(format!(
                        "unknown exact event reference: {sequence}"
                    ))
                    .into()
                }),
            ExactReference::Context {
                resource_id,
                revision,
            } => self
                .registered_context_resource(resource_id, revision)
                .cloned()
                .or_else(|| {
                    self.config_revisions
                        .values()
                        .filter_map(|slot| slot.configuration.as_ref())
                        .find_map(|configuration| {
                            configuration
                                .context_catalog()
                                .resolve_revision(resource_id, revision)
                                .ok()
                                .cloned()
                        })
                })
                .map(ResolvedExactReference::Context)
                .ok_or_else(|| {
                    crate::JournalError::InvalidEvent(format!(
                        "unknown exact context reference: {reference}"
                    ))
                    .into()
                }),
            ExactReference::FileObservation(observation_id) => self
                .journal
                .entries
                .iter()
                .find_map(|entry| match &entry.event {
                    DomainEvent::WorkspaceFileObserved { observation, .. }
                        if &observation.id == observation_id =>
                    {
                        Some(observation.clone())
                    }
                    _ => None,
                })
                .map(ResolvedExactReference::FileObservation)
                .ok_or_else(|| {
                    crate::JournalError::InvalidEvent(format!(
                        "unknown exact file observation reference: {observation_id}"
                    ))
                    .into()
                }),
            ExactReference::LanguageObservation(observation_id) => self
                .journal
                .entries
                .iter()
                .find_map(|entry| match &entry.event {
                    DomainEvent::LanguageObservationRecorded { observation }
                        if &observation.id == observation_id =>
                    {
                        Some(observation.clone())
                    }
                    _ => None,
                })
                .map(ResolvedExactReference::LanguageObservation)
                .ok_or_else(|| {
                    crate::JournalError::InvalidEvent(format!(
                        "unknown exact language observation reference: {observation_id}"
                    ))
                    .into()
                }),
        }
    }

    fn durable_context_resource_for_execution(
        &self,
        execution_id: &ExecutionId,
        resource_id: &ContextResourceId,
        requested_revision: &ContextRevision,
    ) -> Result<Option<ContextResourceRevision>, ConductorError> {
        let Some(descriptor) = self
            .context_descriptors_for_execution(execution_id)?
            .into_iter()
            .find(|descriptor| {
                descriptor.id == *resource_id && descriptor.revision == *requested_revision
            })
        else {
            return Ok(None);
        };

        let (source_ref, content) = match descriptor.kind {
            ContextResourceKind::Objective => {
                let assignment = self.execution_objectives(execution_id)?.ok_or_else(|| {
                    crate::ObjectiveError::MissingExecutionObjective(execution_id.clone())
                })?;
                let objective = self.objective(&assignment.primary)?;
                let expected_id = ContextResourceId::parse(format!("objective:{}", objective.id))
                    .expect("objective id must produce a context resource id");
                if expected_id != descriptor.id {
                    return Ok(None);
                }
                (
                    ExactReference::Objective(objective.id.clone()),
                    serde_json::to_string(&objective)
                        .expect("objective context resource must serialize"),
                )
            }
            ContextResourceKind::Plan => {
                let Some(assignment) = self.execution_plan(execution_id)? else {
                    return Ok(None);
                };
                let plan = self.plan(&assignment.plan_id)?;
                let expected_id = ContextResourceId::parse(format!("plan:{}", plan.id))
                    .expect("plan id must produce a context resource id");
                if expected_id != descriptor.id {
                    return Ok(None);
                }
                (
                    ExactReference::Plan(plan.id.clone()),
                    serde_json::to_string(&plan).expect("plan context resource must serialize"),
                )
            }
            ContextResourceKind::Skill | ContextResourceKind::ProjectDocument => return Ok(None),
        };

        Ok(Some(ContextResourceRevision {
            descriptor,
            tier: ContextTier::DiscoverableContent,
            source_ref,
            content_identity: requested_revision.clone(),
            content: Some(content),
        }))
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
        let resource = if let Ok(resource) = configuration
            .context_catalog()
            .resolve_revision(resource_id, requested_revision)
        {
            resource.clone()
        } else {
            self.durable_context_resource_for_execution(
                execution_id,
                resource_id,
                requested_revision,
            )?
            .ok_or_else(|| ConductorError::InvalidExecutionData {
                execution_id: execution_id.clone(),
                message: format!(
                    "unknown context resource revision: {resource_id}@{requested_revision}"
                ),
            })?
        };
        self.ensure_context_scope_for_execution(execution_id, &resource.descriptor.scope)?;
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
        let mut durable_resource = resource.clone();
        let (_, secret_values) = crate::secret_material(&self.execution_authority(execution_id)?);
        if durable_resource.content.as_ref().is_some_and(|content| {
            secret_values
                .iter()
                .any(|secret| !secret.is_empty() && content.contains(secret))
        }) {
            durable_resource.content = None;
        }
        self.register_context_resource_revision(durable_resource)?;

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
        BackendId, ExecutionTarget, InferenceOptions, ModelId, ModelTarget, ObjectiveId, PlanId,
        PlanStep, PlanStepId, PlanStepRevisability, PlanStepState, ProviderId, WorkspaceId,
    };
    use std::collections::BTreeSet;
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
    fn context_scope_uses_execution_owned_identities() {
        let mut runtime = ConductorRuntime::new();
        let session = runtime.create_session(None, None, fixed_target()).unwrap();
        let execution = runtime
            .submit(&session.id, "validate context scope")
            .unwrap();
        let objective = runtime
            .execution_objectives(&execution.id)
            .unwrap()
            .expect("root execution must have an objective")
            .primary;
        let config_revision = runtime.execution_config_revision(&execution.id).unwrap();

        runtime
            .ensure_context_scope_for_execution(
                &execution.id,
                &ContextScope::Workspace {
                    workspace_id: session.workspace_id.clone(),
                },
            )
            .unwrap();
        runtime
            .ensure_context_scope_for_execution(
                &execution.id,
                &ContextScope::Execution {
                    execution_id: execution.id.clone(),
                },
            )
            .unwrap();
        runtime
            .ensure_context_scope_for_execution(
                &execution.id,
                &ContextScope::Objective {
                    objective_id: objective,
                },
            )
            .unwrap();
        runtime
            .ensure_context_scope_for_execution(
                &execution.id,
                &ContextScope::Configuration {
                    revision: config_revision,
                },
            )
            .unwrap();
        runtime
            .ensure_context_scope_for_execution(
                &execution.id,
                &ContextScope::Path {
                    path: PathBuf::from("CONTRIBUTING.md"),
                },
            )
            .unwrap();

        for scope in [
            ContextScope::Workspace {
                workspace_id: WorkspaceId::parse("workspace:other").unwrap(),
            },
            ContextScope::Execution {
                execution_id: ExecutionId::parse("execution-other").unwrap(),
            },
            ContextScope::Objective {
                objective_id: ObjectiveId::parse("objective-other").unwrap(),
            },
            ContextScope::Configuration {
                revision: ConfigRevisionId::parse("config-other").unwrap(),
            },
        ] {
            assert!(matches!(
                runtime.ensure_context_scope_for_execution(&execution.id, &scope),
                Err(ConductorError::InvalidExecutionData { .. })
            ));
        }
    }

    #[test]
    fn exact_plan_reference_resolves_authoritative_plan() {
        let mut runtime = ConductorRuntime::new();
        let session = runtime.create_session(None, None, fixed_target()).unwrap();
        let execution = runtime
            .submit(&session.id, "resolve plan reference")
            .unwrap();
        let objective = runtime
            .execution_objectives(&execution.id)
            .unwrap()
            .expect("root execution must have an objective")
            .primary;
        let plan = runtime
            .create_plan(
                BTreeSet::from([objective.clone()]),
                vec![PlanStep {
                    id: PlanStepId::parse("step-1").unwrap(),
                    description: "Resolve the durable plan by exact identity".to_owned(),
                    state: PlanStepState::Proposed,
                    revisability: PlanStepRevisability::Revisable,
                    depends_on: BTreeSet::new(),
                    objective_refs: BTreeSet::from([objective]),
                }],
            )
            .unwrap();

        let resolved = runtime
            .resolve_exact_reference(&ExactReference::Plan(plan.id.clone()))
            .unwrap();
        assert_eq!(resolved.plan(), Some(&plan));
        assert!(runtime
            .resolve_exact_reference(&ExactReference::Plan(
                PlanId::parse("plan-missing").unwrap()
            ))
            .is_err());
    }

    #[test]
    fn durable_objective_and_plan_descriptors_use_common_load_path() {
        let mut runtime = ConductorRuntime::new();
        let session = runtime.create_session(None, None, fixed_target()).unwrap();
        let execution = runtime
            .submit(&session.id, "load durable semantic context")
            .unwrap();
        let objective = runtime
            .execution_objectives(&execution.id)
            .unwrap()
            .expect("execution must have a primary objective")
            .primary;
        let step_id = PlanStepId::parse("context-load").unwrap();
        let plan = runtime
            .create_plan(
                BTreeSet::from([objective.clone()]),
                vec![PlanStep {
                    id: step_id.clone(),
                    description: "load plan through common context operation".to_owned(),
                    state: PlanStepState::Proposed,
                    revisability: PlanStepRevisability::Revisable,
                    depends_on: BTreeSet::new(),
                    objective_refs: BTreeSet::from([objective.clone()]),
                }],
            )
            .unwrap();
        runtime
            .assign_execution_to_plan_step(&execution.id, &plan.id, &step_id)
            .unwrap();

        let descriptors = runtime
            .context_descriptors_for_execution(&execution.id)
            .unwrap();
        let objective_id = ContextResourceId::parse(format!("objective:{objective}")).unwrap();
        let plan_id = ContextResourceId::parse(format!("plan:{}", plan.id)).unwrap();
        let objective_descriptor = descriptors
            .iter()
            .find(|descriptor| descriptor.id == objective_id)
            .unwrap();
        let plan_descriptor = descriptors
            .iter()
            .find(|descriptor| descriptor.id == plan_id)
            .unwrap();

        let (objective_resource, objective_injection) = runtime
            .load_context_for_execution(
                &execution.id,
                &objective_id,
                &objective_descriptor.revision,
                ContextInjectionRequester::Agent,
                ContextInjectionLifetime::SingleRequest,
                "load exact objective context",
            )
            .unwrap();
        assert_eq!(
            objective_resource.source_ref,
            ExactReference::Objective(objective.clone())
        );
        assert_eq!(
            objective_injection.source_ref,
            objective_resource.source_ref
        );

        let (plan_resource, plan_injection) = runtime
            .load_context_for_execution(
                &execution.id,
                &plan_id,
                &plan_descriptor.revision,
                ContextInjectionRequester::Agent,
                ContextInjectionLifetime::SingleRequest,
                "load exact plan context",
            )
            .unwrap();
        assert_eq!(plan_resource.source_ref, ExactReference::Plan(plan.id));
        assert_eq!(plan_injection.source_ref, plan_resource.source_ref);
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
