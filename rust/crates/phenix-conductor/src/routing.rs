use crate::{CompiledConfiguration, ConductorRuntime};
use phenix_core::{
    CallableId, LanguageServiceConfiguration, LanguageServiceRequirement,
    ManagedLanguageProviderDefinition, ModelTarget, RoutingProfile, RoutingProfileDescriptor,
    RoutingProfileId, WorkspaceId,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RoutingRegistryError {
    Duplicate(RoutingProfileId),
    Unknown(RoutingProfileId),
}

impl Display for RoutingRegistryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Duplicate(id) => write!(f, "routing profile already registered: {id}"),
            Self::Unknown(id) => write!(f, "unknown routing profile: {id}"),
        }
    }
}

impl Error for RoutingRegistryError {}

#[derive(Clone, Debug, Default)]
pub struct RoutingRegistry {
    profiles: BTreeMap<RoutingProfileId, RoutingProfile>,
    language_services: LanguageServiceConfiguration,
}

impl RoutingRegistry {
    pub(crate) fn semantic_manifest(&self) -> Value {
        json!({
            "profiles": self.profiles,
            "language_services": self.language_services.semantic_manifest(),
        })
    }

    pub fn register(&mut self, profile: RoutingProfile) -> Result<(), RoutingRegistryError> {
        if self.profiles.contains_key(&profile.id) {
            return Err(RoutingRegistryError::Duplicate(profile.id));
        }
        self.profiles.insert(profile.id.clone(), profile);
        Ok(())
    }

    pub fn register_managed_language_provider(
        &mut self,
        definition: ManagedLanguageProviderDefinition,
    ) -> Result<(), phenix_core::LanguageServiceError> {
        self.language_services.register_managed(definition)
    }

    pub fn set_language_service_requirement(&mut self, requirement: LanguageServiceRequirement) {
        self.language_services.set_requirement(requirement);
    }

    #[must_use]
    pub fn language_service_configuration(&self) -> &LanguageServiceConfiguration {
        &self.language_services
    }

    #[must_use]
    pub fn contains(&self, profile: &RoutingProfileId) -> bool {
        self.profiles.contains_key(profile)
    }

    /// Returns the configured routing profiles with the distinct providers that
    /// must be authenticated before the profile can be used.
    #[must_use]
    pub fn descriptors(&self) -> Vec<RoutingProfileDescriptor> {
        self.profiles
            .values()
            .map(|profile| {
                let mut providers = BTreeSet::from([profile.default_target.provider.clone()]);
                providers.extend(
                    profile
                        .callable_targets
                        .values()
                        .map(|target| target.provider.clone()),
                );
                RoutingProfileDescriptor {
                    id: profile.id.clone(),
                    providers: providers.into_iter().collect(),
                }
            })
            .collect()
    }

    pub fn resolve(
        &self,
        profile: &RoutingProfileId,
        callable: Option<&CallableId>,
    ) -> Result<ModelTarget, RoutingRegistryError> {
        let profile = self
            .profiles
            .get(profile)
            .ok_or_else(|| RoutingRegistryError::Unknown(profile.clone()))?;
        Ok(callable
            .and_then(|id| profile.callable_targets.get(id))
            .unwrap_or(&profile.default_target)
            .clone())
    }
}

impl CompiledConfiguration {
    pub fn register_managed_language_provider(
        &mut self,
        definition: ManagedLanguageProviderDefinition,
    ) -> Result<(), phenix_core::LanguageServiceError> {
        self.routing.register_managed_language_provider(definition)
    }

    pub fn set_language_service_requirement(&mut self, requirement: LanguageServiceRequirement) {
        self.routing.set_language_service_requirement(requirement);
    }

    #[must_use]
    pub fn language_service_configuration(&self) -> &LanguageServiceConfiguration {
        self.routing.language_service_configuration()
    }
}

impl ConductorRuntime {
    #[must_use]
    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_core::{
        BackendId, InferenceOptions, LanguageProviderCapabilities, LanguageProviderId,
        LanguageServiceKind, ModelId, ProviderId,
    };
    use std::path::PathBuf;

    fn model(provider: &str, name: &str) -> ModelTarget {
        ModelTarget {
            backend: BackendId::parse("mock").unwrap(),
            provider: ProviderId::parse(provider).unwrap(),
            model: ModelId::parse(name).unwrap(),
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
    fn callable_override_wins_over_profile_default() {
        let agent = CallableId::parse("agent.scout").unwrap();
        let mut routing = RoutingRegistry::default();
        routing
            .register(RoutingProfile {
                id: RoutingProfileId::parse("default").unwrap(),
                default_target: model("mock", "root"),
                callable_targets: BTreeMap::from([(agent.clone(), model("mock", "scout"))]),
            })
            .unwrap();
        assert_eq!(
            routing
                .resolve(&RoutingProfileId::parse("default").unwrap(), Some(&agent))
                .unwrap(),
            model("mock", "scout")
        );
    }

    #[test]
    fn catalog_is_sorted_and_lists_every_distinct_provider() {
        let agent = CallableId::parse("agent.scout").unwrap();
        let mut routing = RoutingRegistry::default();
        routing
            .register(RoutingProfile {
                id: RoutingProfileId::parse("router.zeta").unwrap(),
                default_target: model("openai-codex", "root"),
                callable_targets: BTreeMap::from([(agent.clone(), model("opencode-go", "scout"))]),
            })
            .unwrap();
        routing
            .register(RoutingProfile {
                id: RoutingProfileId::parse("router.alpha").unwrap(),
                default_target: model("openai-api", "root"),
                callable_targets: BTreeMap::from([(agent, model("openai-api", "scout"))]),
            })
            .unwrap();

        assert_eq!(
            routing.descriptors(),
            vec![
                RoutingProfileDescriptor {
                    id: RoutingProfileId::parse("router.alpha").unwrap(),
                    providers: vec![ProviderId::parse("openai-api").unwrap()],
                },
                RoutingProfileDescriptor {
                    id: RoutingProfileId::parse("router.zeta").unwrap(),
                    providers: vec![
                        ProviderId::parse("openai-codex").unwrap(),
                        ProviderId::parse("opencode-go").unwrap(),
                    ],
                },
            ]
        );
    }

    #[test]
    fn language_configuration_changes_the_compiled_configuration_fingerprint() {
        let mut configuration = CompiledConfiguration::default();
        let before = configuration.fingerprint();
        configuration
            .register_managed_language_provider(ManagedLanguageProviderDefinition {
                service: language_kind(),
                provider: language_provider(),
                command: PathBuf::from("rust-analyzer"),
                args: vec!["--stdio".to_owned()],
                capabilities: LanguageProviderCapabilities {
                    requests: true,
                    notifications: true,
                    shared_diagnostics: true,
                    background_documents: true,
                    dirty_buffers: false,
                },
            })
            .unwrap();
        configuration.set_language_service_requirement(LanguageServiceRequirement {
            service: language_kind(),
            required_capabilities: LanguageProviderCapabilities {
                requests: true,
                ..LanguageProviderCapabilities::default()
            },
            preferred_provider: Some(language_provider()),
        });
        assert_ne!(before, configuration.fingerprint());
    }
}
