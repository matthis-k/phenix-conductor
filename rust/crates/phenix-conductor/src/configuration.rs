#[cfg(test)]
use phenix_conductor::ContextBudgetPolicy;
use phenix_conductor::{CompiledConfiguration, ConductorError, ContextCompactionConfiguration};
use phenix_core::{
    AgentDefinition, LanguageServiceRequirement, ManagedLanguageProviderDefinition,
    OrchestrationDefinition, RoutingProfile,
};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

/// Process-owned executable configuration for one conductor deployment.
///
/// The conductor owns validation and execution semantics, while applications
/// own the concrete agent, orchestration, routing, and managed language-service
/// definitions supplied in this file. The durable store keeps revision
/// fingerprints and runtime state. Process startup recompiles every supplied
/// historical file and binds revisions by semantic fingerprint.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeConfiguration {
    #[serde(default)]
    pub agents: Vec<AgentDefinition>,
    #[serde(default)]
    pub orchestrations: Vec<OrchestrationDefinition>,
    #[serde(default)]
    pub routing_profiles: Vec<RoutingProfile>,
    #[serde(default)]
    pub managed_language_providers: Vec<ManagedLanguageProviderDefinition>,
    #[serde(default)]
    pub language_services: Vec<LanguageServiceRequirement>,
    #[serde(default)]
    pub context_compaction: Option<ContextCompactionConfiguration>,
}

impl RuntimeConfiguration {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigurationError> {
        let path = path.as_ref();
        let source = fs::read_to_string(path).map_err(|source| ConfigurationError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        serde_json::from_str(&source).map_err(|source| ConfigurationError::Parse {
            path: path.to_path_buf(),
            source,
        })
    }

    pub fn compile(
        self,
        mut configuration: CompiledConfiguration,
    ) -> Result<CompiledConfiguration, ConfigurationError> {
        for agent in self.agents {
            configuration.register_agent(agent)?;
        }
        for orchestration in self.orchestrations {
            configuration.register_orchestration(orchestration)?;
        }
        for profile in self.routing_profiles {
            configuration.register_routing_profile(profile)?;
        }
        for provider in self.managed_language_providers {
            configuration.register_managed_language_provider(provider)?;
        }
        for requirement in self.language_services {
            configuration.set_language_service_requirement(requirement);
        }
        if let Some(context_compaction) = self.context_compaction {
            configuration.configure_context_compaction(context_compaction);
        }
        Ok(configuration)
    }
}

#[derive(Debug)]
pub enum ConfigurationError {
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: PathBuf,
        source: serde_json::Error,
    },
    Runtime(ConductorError),
    Language(phenix_core::LanguageServiceError),
}

impl Display for ConfigurationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(
                    f,
                    "failed to read conductor configuration {}: {source}",
                    path.display()
                )
            }
            Self::Parse { path, source } => {
                write!(
                    f,
                    "invalid conductor configuration {}: {source}",
                    path.display()
                )
            }
            Self::Runtime(source) => write!(f, "invalid conductor configuration: {source}"),
            Self::Language(source) => {
                write!(f, "invalid language-service configuration: {source}")
            }
        }
    }
}

impl Error for ConfigurationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
            Self::Runtime(source) => Some(source),
            Self::Language(source) => Some(source),
        }
    }
}

impl From<ConductorError> for ConfigurationError {
    fn from(value: ConductorError) -> Self {
        Self::Runtime(value)
    }
}

impl From<phenix_core::LanguageServiceError> for ConfigurationError {
    fn from(value: phenix_core::LanguageServiceError) -> Self {
        Self::Language(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_conductor::ConductorRuntime;
    use phenix_core::{
        BackendId, CallableDescriptor, CallableId, CallableKind, CallablePolicy, CapabilitySet,
        ExecutionAuthority, ExecutionTarget, FilesystemAuthority, InferenceOptions,
        LanguageProviderCapabilities, LanguageProviderId, LanguageServiceKind, ModelId,
        ModelTarget, NetworkAuthority, OrchestrationNode, OrchestrationNodeId,
        OrchestrationValueBinding, ProviderId, RepositoryAuthority, RoutingProfileId,
    };
    use serde_json::json;
    use std::collections::BTreeMap;

    fn install_configuration(
        runtime: &mut ConductorRuntime,
        configuration: RuntimeConfiguration,
    ) -> Result<(), ConfigurationError> {
        let base = runtime.current_compiled_configuration()?;
        runtime.reload_configuration(configuration.compile(base)?)?;
        Ok(())
    }

    fn descriptor(id: &str, kind: CallableKind) -> CallableDescriptor {
        CallableDescriptor {
            id: CallableId::parse(id).unwrap(),
            kind,
            description: format!("{id} fixture"),
            input_schema: json!({"type": "object"}),
            output_schema: json!({"type": "object"}),
            capabilities: CapabilitySet::default(),
            policy: CallablePolicy::default(),
        }
    }

    fn node(id: &str, callable: CallableId, objective: Option<&str>) -> OrchestrationNode {
        OrchestrationNode {
            input_bindings: BTreeMap::from([(
                "objective".to_owned(),
                OrchestrationValueBinding::Input {
                    pointer: "/objective".to_owned(),
                },
            )]),
            id: OrchestrationNodeId::parse(id).unwrap(),
            callable,
            depends_on: Vec::new(),
            objective: objective.map(str::to_owned),
        }
    }

    fn target(model: &str) -> ModelTarget {
        ModelTarget {
            backend: BackendId::parse("phenix").unwrap(),
            provider: ProviderId::parse("fixture").unwrap(),
            model: ModelId::parse(model).unwrap(),
            inference: InferenceOptions::default(),
        }
    }

    fn language_kind() -> LanguageServiceKind {
        LanguageServiceKind::parse("rust").unwrap()
    }

    fn language_provider() -> LanguageProviderId {
        LanguageProviderId::parse("rust-analyzer").unwrap()
    }

    #[test]
    fn application_configuration_rebinds_agents_workflows_and_routes() {
        let agent = descriptor("agent.fixture", CallableKind::Agent);
        let orchestration = OrchestrationDefinition {
            output_bindings: Default::default(),
            interface_agent: None,
            descriptor: descriptor("orchestration.fixture", CallableKind::Orchestration),
            nodes: vec![node(
                "fixture",
                agent.id.clone(),
                Some("inspect the objective"),
            )],
        };
        let route = RoutingProfile {
            id: RoutingProfileId::parse("router.fixture").unwrap(),
            default_target: target("fallback"),
            callable_targets: BTreeMap::from([(agent.id.clone(), target("agent"))]),
        };
        let encoded = serde_json::to_string(&RuntimeConfiguration {
            agents: vec![AgentDefinition::new(
                agent.clone(),
                ExecutionAuthority::read_only(),
            )],
            orchestrations: vec![orchestration],
            routing_profiles: vec![route],
            ..RuntimeConfiguration::default()
        })
        .unwrap();
        let configuration: RuntimeConfiguration = serde_json::from_str(&encoded).unwrap();
        let mut runtime = ConductorRuntime::new();
        install_configuration(&mut runtime, configuration).unwrap();

        assert_eq!(runtime.callable_descriptors().unwrap().len(), 2);
        assert_eq!(
            runtime
                .callable_descriptors()
                .unwrap()
                .into_iter()
                .map(|item| item.id)
                .collect::<Vec<_>>(),
            vec![
                agent.id,
                CallableId::parse("orchestration.fixture").unwrap()
            ]
        );

        let session = runtime
            .create_session(
                None,
                None,
                ExecutionTarget::Routed(RoutingProfileId::parse("router.fixture").unwrap()),
            )
            .unwrap();
        let execution = runtime.submit(&session.id, "route me").unwrap();
        assert_eq!(
            runtime.resolve_invocation(&execution.id).unwrap().model,
            target("fallback")
        );
    }

    #[test]
    fn configured_agent_authority_reaches_execution_creation() {
        let descriptor = descriptor("agent.writer", CallableKind::Agent);
        let authority = ExecutionAuthority {
            filesystem: FilesystemAuthority::Write,
            network: NetworkAuthority::None,
            repository: RepositoryAuthority::Read,
            ..ExecutionAuthority::read_only()
        };
        let configuration = RuntimeConfiguration {
            agents: vec![AgentDefinition::new(descriptor.clone(), authority.clone())],
            ..RuntimeConfiguration::default()
        };
        let encoded = serde_json::to_string(&configuration).unwrap();
        let decoded: RuntimeConfiguration = serde_json::from_str(&encoded).unwrap();
        let mut runtime = ConductorRuntime::new();
        install_configuration(&mut runtime, decoded).unwrap();
        let session = runtime
            .create_session(None, None, ExecutionTarget::Fixed(target("worker")))
            .unwrap();
        let execution = runtime
            .start_session_callable(&session.id, &descriptor.id, "write")
            .unwrap();

        assert_eq!(
            runtime.execution_authority(&execution.id).unwrap(),
            authority
        );
    }

    #[test]
    fn configured_workflow_step_keeps_the_user_objective() {
        let agent = descriptor("agent.worker", CallableKind::Agent);
        let workflow_id = CallableId::parse("orchestration.implement").unwrap();
        let configuration = RuntimeConfiguration {
            agents: vec![AgentDefinition::new(
                agent.clone(),
                ExecutionAuthority::read_only(),
            )],
            orchestrations: vec![OrchestrationDefinition {
                output_bindings: Default::default(),
                interface_agent: None,
                descriptor: descriptor(workflow_id.as_str(), CallableKind::Orchestration),
                nodes: vec![node(
                    "implement",
                    agent.id,
                    Some("Implement the bounded change."),
                )],
            }],
            ..RuntimeConfiguration::default()
        };
        let mut runtime = ConductorRuntime::new();
        install_configuration(&mut runtime, configuration).unwrap();

        let session = runtime
            .create_session(None, None, ExecutionTarget::Fixed(target("worker")))
            .unwrap();
        let root = runtime.submit(&session.id, "root").unwrap();
        let orchestration = runtime
            .start_orchestration(
                &root.id,
                &workflow_id,
                serde_json::json!({"objective": "Fix routing selection"}),
            )
            .unwrap();
        let child = runtime
            .snapshot()
            .executions
            .into_iter()
            .find(|execution| execution.parent_execution.as_ref() == Some(&orchestration.id))
            .expect("orchestration child exists");

        assert_eq!(
            runtime.resolve_invocation(&child.id).unwrap().prompt,
            "Implement the bounded change.\n\nTyped orchestration input:\n{\"objective\":\"Fix routing selection\"}"
        );
    }

    #[test]
    fn context_compaction_configuration_roundtrips_and_is_revision_bound() {
        let expected = ContextCompactionConfiguration {
            compactor_target: target("compactor"),
            budget_policy: ContextBudgetPolicy {
                output_reserve_tokens: 1_024,
                safety_margin_tokens: 512,
            },
        };
        let encoded = serde_json::to_string(&RuntimeConfiguration {
            context_compaction: Some(expected.clone()),
            ..RuntimeConfiguration::default()
        })
        .unwrap();
        let decoded: RuntimeConfiguration = serde_json::from_str(&encoded).unwrap();
        let mut runtime = ConductorRuntime::new();
        let before = runtime.current_config_revision().clone();
        install_configuration(&mut runtime, decoded).unwrap();
        assert_ne!(runtime.current_config_revision(), &before);
        let session = runtime
            .create_session(None, None, ExecutionTarget::Fixed(target("worker")))
            .unwrap();
        let execution = runtime
            .submit(&session.id, "configured compaction")
            .unwrap();
        assert_eq!(
            runtime
                .context_compaction_configuration_for_execution(&execution.id)
                .unwrap(),
            Some(expected)
        );
    }

    #[test]
    fn application_configuration_rejects_wrong_callable_kinds() {
        let configuration = RuntimeConfiguration {
            agents: vec![AgentDefinition::new(
                descriptor("tool.not-an-agent", CallableKind::Tool),
                ExecutionAuthority::read_only(),
            )],
            ..RuntimeConfiguration::default()
        };
        assert!(matches!(
            install_configuration(&mut ConductorRuntime::new(), configuration),
            Err(ConfigurationError::Runtime(_))
        ));
    }

    #[test]
    fn language_service_configuration_roundtrips_and_is_revision_bound() {
        let capabilities = LanguageProviderCapabilities {
            requests: true,
            notifications: true,
            shared_diagnostics: true,
            background_documents: true,
            dirty_buffers: false,
        };
        let configuration = RuntimeConfiguration {
            managed_language_providers: vec![ManagedLanguageProviderDefinition {
                service: language_kind(),
                provider: language_provider(),
                command: PathBuf::from("rust-analyzer"),
                args: vec!["--stdio".to_owned()],
                capabilities: capabilities.clone(),
            }],
            language_services: vec![LanguageServiceRequirement {
                service: language_kind(),
                required_capabilities: capabilities,
                preferred_provider: Some(language_provider()),
            }],
            ..RuntimeConfiguration::default()
        };
        let encoded = serde_json::to_string(&configuration).unwrap();
        let decoded: RuntimeConfiguration = serde_json::from_str(&encoded).unwrap();
        let mut runtime = ConductorRuntime::new();
        let before = runtime.current_config_revision().clone();
        install_configuration(&mut runtime, decoded).unwrap();
        assert_ne!(runtime.current_config_revision(), &before);
        let compiled = runtime.current_compiled_configuration().unwrap();
        assert_eq!(
            compiled
                .language_service_configuration()
                .managed_for(&language_kind())
                .len(),
            1
        );
        assert_eq!(
            compiled
                .language_service_configuration()
                .requirement_for(&language_kind())
                .preferred_provider,
            Some(language_provider())
        );
    }
}
