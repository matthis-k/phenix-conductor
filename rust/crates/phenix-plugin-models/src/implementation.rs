use phenix_core::{
    Authority, CapabilityId, DurableSchema, PluginExecution, PluginHost, PluginId, PluginInstance,
    PluginManifest, ResourceNamespace, ServiceContribution, ServiceId, TransactionOp,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MODEL_ROUTING_SERVICE: &str = "phenix.models.routing@1";
pub const MODEL_INFERENCE_SERVICE: &str = "phenix.models.inference@1";
const MODEL_ROUTING_PLUGIN: &str = "phenix.models";
const MODEL_NAMESPACE: &str = "phenix.models.state";
const PERSISTENCE_SCHEMA: &str = "kernel.persistence.schema";
const PERSISTENCE_READ: &str = "kernel.persistence.read";
const PERSISTENCE_WRITE: &str = "kernel.persistence.write";
const PROFILE_INDEX: &str = "index/profiles";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelTarget {
    pub provider_plugin: String,
    pub model: String,
    #[serde(default)]
    pub options: BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RoutingProfile {
    pub id: String,
    pub default_target: ModelTarget,
    #[serde(default)]
    pub callable_targets: BTreeMap<String, ModelTarget>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RoutingProfileDescriptor {
    pub id: String,
    pub providers: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelInferenceRequest {
    pub model: String,
    pub input: Vec<u8>,
    #[serde(default)]
    pub options: BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelInferenceResponse {
    pub output: Vec<u8>,
    pub provider_metadata: BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum ModelCommand {
    RegisterProfile {
        profile: RoutingProfile,
    },
    GetProfile {
        id: String,
    },
    ListProfiles,
    SetProviderAuthenticated {
        provider_plugin: String,
        authenticated: bool,
    },
    Resolve {
        profile_id: String,
        callable_id: Option<String>,
    },
    Invoke {
        profile_id: String,
        callable_id: Option<String>,
        input: Vec<u8>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ModelResponse {
    Profile {
        profile: Option<RoutingProfile>,
    },
    Profiles {
        profiles: Vec<RoutingProfileDescriptor>,
    },
    Authentication {
        provider_plugin: String,
        authenticated: bool,
    },
    Target {
        target: ModelTarget,
    },
    Inference {
        target: ModelTarget,
        response: ModelInferenceResponse,
    },
}

#[must_use]
pub fn model_routing_manifest(maximum_authority: Authority) -> PluginManifest {
    let persistence = Authority::new([
        capability(PERSISTENCE_SCHEMA),
        capability(PERSISTENCE_READ),
        capability(PERSISTENCE_WRITE),
    ]);
    let maximum_authority = Authority::new(
        maximum_authority
            .capabilities()
            .cloned()
            .chain(persistence.capabilities().cloned()),
    );
    PluginManifest {
        id: PluginId::parse(MODEL_ROUTING_PLUGIN).expect("static plugin id is valid"),
        version: 1,
        execution: PluginExecution::Embedded,
        dependencies: Vec::new(),
        services: vec![ServiceContribution {
            service: model_routing_service(),
            priority: 100,
            required_authority: Authority::default(),
        }],
        resource_namespaces: vec![model_namespace()],
        maximum_authority,
    }
}

#[must_use]
pub fn model_routing_factory() -> Box<dyn PluginInstance> {
    Box::new(ModelRoutingPlugin::default())
}

#[must_use]
pub fn model_routing_service() -> ServiceId {
    ServiceId::parse(MODEL_ROUTING_SERVICE).expect("static service id is valid")
}

#[must_use]
pub fn model_inference_service() -> ServiceId {
    ServiceId::parse(MODEL_INFERENCE_SERVICE).expect("static service id is valid")
}

fn model_namespace() -> ResourceNamespace {
    ResourceNamespace::parse(MODEL_NAMESPACE).expect("static namespace is valid")
}

fn capability(value: &str) -> CapabilityId {
    CapabilityId::parse(value).expect("static capability is valid")
}

#[derive(Default)]
struct ModelRoutingPlugin {
    authenticated: BTreeSet<PluginId>,
}

impl PluginInstance for ModelRoutingPlugin {
    fn start(&mut self, host: &PluginHost<'_>) -> Result<(), String> {
        host.register_durable_schema(&DurableSchema::new(model_namespace(), 1))
            .map_err(|error| error.to_string())
    }

    fn invoke(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        if service != &model_routing_service() {
            return Err(format!("unsupported model routing service: {service}"));
        }
        let command: ModelCommand =
            serde_json::from_slice(input).map_err(|error| error.to_string())?;
        let response = match command {
            ModelCommand::RegisterProfile { profile } => {
                validate_profile(&profile)?;
                insert_profile(host, &profile)?;
                ModelResponse::Profile {
                    profile: Some(profile),
                }
            }
            ModelCommand::GetProfile { id } => {
                validate_identity("routing profile id", &id)?;
                ModelResponse::Profile {
                    profile: read_profile(host, &id)?,
                }
            }
            ModelCommand::ListProfiles => ModelResponse::Profiles {
                profiles: load_profiles(host)?
                    .into_iter()
                    .map(|profile| descriptor(&profile))
                    .collect(),
            },
            ModelCommand::SetProviderAuthenticated {
                provider_plugin,
                authenticated,
            } => {
                let plugin =
                    PluginId::parse(provider_plugin.clone()).map_err(|error| error.to_string())?;
                if authenticated {
                    self.authenticated.insert(plugin);
                } else {
                    self.authenticated.remove(&plugin);
                }
                ModelResponse::Authentication {
                    provider_plugin,
                    authenticated,
                }
            }
            ModelCommand::Resolve {
                profile_id,
                callable_id,
            } => ModelResponse::Target {
                target: resolve_target(host, &profile_id, callable_id.as_deref())?,
            },
            ModelCommand::Invoke {
                profile_id,
                callable_id,
                input,
            } => {
                let target = resolve_target(host, &profile_id, callable_id.as_deref())?;
                let provider = PluginId::parse(target.provider_plugin.clone())
                    .map_err(|error| error.to_string())?;
                if !self.authenticated.contains(&provider) {
                    return Err(format!("provider authentication required: {provider}"));
                }
                let request = ModelInferenceRequest {
                    model: target.model.clone(),
                    input,
                    options: target.options.clone(),
                };
                let output = host
                    .invoke_service(
                        &model_inference_service(),
                        &serde_json::to_vec(&request).map_err(|error| error.to_string())?,
                        host.authority(),
                        Some(&provider),
                    )
                    .map_err(|error| error.to_string())?;
                let response: ModelInferenceResponse =
                    serde_json::from_slice(&output).map_err(|error| error.to_string())?;
                ModelResponse::Inference { target, response }
            }
        };
        serde_json::to_vec(&response).map_err(|error| error.to_string())
    }
}

fn validate_profile(profile: &RoutingProfile) -> Result<(), String> {
    validate_identity("routing profile id", &profile.id)?;
    validate_target(&profile.default_target)?;
    for (callable, target) in &profile.callable_targets {
        validate_identity("callable id", callable)?;
        validate_target(target)?;
    }
    Ok(())
}

fn validate_target(target: &ModelTarget) -> Result<(), String> {
    PluginId::parse(target.provider_plugin.clone()).map_err(|error| error.to_string())?;
    validate_identity("model id", &target.model)
}

fn validate_identity(label: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{label} must not be empty"))
    } else {
        Ok(())
    }
}

fn descriptor(profile: &RoutingProfile) -> RoutingProfileDescriptor {
    let mut providers = BTreeSet::from([profile.default_target.provider_plugin.clone()]);
    providers.extend(
        profile
            .callable_targets
            .values()
            .map(|target| target.provider_plugin.clone()),
    );
    RoutingProfileDescriptor {
        id: profile.id.clone(),
        providers: providers.into_iter().collect(),
    }
}

fn resolve_target(
    host: &PluginHost<'_>,
    profile_id: &str,
    callable_id: Option<&str>,
) -> Result<ModelTarget, String> {
    validate_identity("routing profile id", profile_id)?;
    let profile = read_profile(host, profile_id)?
        .ok_or_else(|| format!("unknown routing profile: {profile_id}"))?;
    Ok(callable_id
        .and_then(|callable| profile.callable_targets.get(callable))
        .unwrap_or(&profile.default_target)
        .clone())
}

fn insert_profile(host: &PluginHost<'_>, profile: &RoutingProfile) -> Result<(), String> {
    let key = profile_key(&profile.id);
    let old_index = read_raw(host, PROFILE_INDEX)?;
    let mut ids: Vec<String> = old_index
        .as_deref()
        .map(|value| serde_json::from_slice(value).map_err(|error| error.to_string()))
        .transpose()?
        .unwrap_or_default();
    if ids.iter().any(|id| id == &profile.id) || read_raw(host, &key)?.is_some() {
        return Err(format!(
            "routing profile already registered: {}",
            profile.id
        ));
    }
    ids.push(profile.id.clone());
    ids.sort();
    host.transact_durable(
        &model_namespace(),
        &[
            TransactionOp::AssertValue {
                key: key.clone(),
                expected: None,
            },
            TransactionOp::AssertValue {
                key: PROFILE_INDEX.into(),
                expected: old_index,
            },
            TransactionOp::Put {
                key,
                value: serde_json::to_vec(profile).map_err(|error| error.to_string())?,
            },
            TransactionOp::Put {
                key: PROFILE_INDEX.into(),
                value: serde_json::to_vec(&ids).map_err(|error| error.to_string())?,
            },
        ],
    )
    .map_err(|error| error.to_string())
}

fn read_profile(host: &PluginHost<'_>, id: &str) -> Result<Option<RoutingProfile>, String> {
    read_raw(host, &profile_key(id))?
        .map(|value| serde_json::from_slice(&value).map_err(|error| error.to_string()))
        .transpose()
}

fn load_profiles(host: &PluginHost<'_>) -> Result<Vec<RoutingProfile>, String> {
    let ids: Vec<String> = read_raw(host, PROFILE_INDEX)?
        .as_deref()
        .map(|value| serde_json::from_slice(value).map_err(|error| error.to_string()))
        .transpose()?
        .unwrap_or_default();
    ids.into_iter()
        .map(|id| read_profile(host, &id)?.ok_or_else(|| format!("missing routing profile: {id}")))
        .collect()
}

fn read_raw(host: &PluginHost<'_>, key: &str) -> Result<Option<Vec<u8>>, String> {
    host.read_durable(&model_namespace(), key)
        .map_err(|error| error.to_string())
}

fn profile_key(id: &str) -> String {
    format!("profile/{id}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_core::{Kernel, KernelConfig, LocalPersistence};
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_db(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "phenix-{name}-{}-{nonce}.sqlite",
            std::process::id()
        ))
    }

    fn target(provider: &str, model: &str) -> ModelTarget {
        ModelTarget {
            provider_plugin: provider.into(),
            model: model.into(),
            options: BTreeMap::new(),
        }
    }

    fn routing_authority() -> Authority {
        model_routing_manifest(Authority::default()).maximum_authority
    }

    fn kernel_with(path: &PathBuf) -> Kernel {
        let manifest = model_routing_manifest(Authority::default());
        let plugin = manifest.id.clone();
        let persistence = LocalPersistence::open(path).unwrap();
        let mut kernel =
            Kernel::with_persistence(KernelConfig::new([manifest]).unwrap(), persistence);
        kernel
            .register_embedded_factory(plugin, model_routing_factory)
            .unwrap();
        kernel.activate_all().unwrap();
        kernel
    }

    fn invoke(kernel: &mut Kernel, command: ModelCommand) -> Result<ModelResponse, String> {
        let output = kernel
            .invoke(
                &model_routing_service(),
                &serde_json::to_vec(&command).unwrap(),
                &routing_authority(),
                None,
            )
            .map_err(|error| error.to_string())?;
        serde_json::from_slice(&output).map_err(|error| error.to_string())
    }

    #[test]
    fn routing_profiles_are_immutable_durable_and_callable_specific() {
        let path = temp_db("model-routing");
        let profile = RoutingProfile {
            id: "default".into(),
            default_target: target("provider.default", "root"),
            callable_targets: BTreeMap::from([(
                "agent.scout".into(),
                target("provider.scout", "scout"),
            )]),
        };
        {
            let mut kernel = kernel_with(&path);
            invoke(
                &mut kernel,
                ModelCommand::RegisterProfile {
                    profile: profile.clone(),
                },
            )
            .unwrap();
            assert!(invoke(
                &mut kernel,
                ModelCommand::RegisterProfile {
                    profile: profile.clone()
                }
            )
            .unwrap_err()
            .contains("already registered"));
        }
        let mut restored = kernel_with(&path);
        let response = invoke(
            &mut restored,
            ModelCommand::Resolve {
                profile_id: "default".into(),
                callable_id: Some("agent.scout".into()),
            },
        )
        .unwrap();
        assert_eq!(
            response,
            ModelResponse::Target {
                target: target("provider.scout", "scout")
            }
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn provider_authentication_is_process_local_not_durable() {
        let path = temp_db("model-auth");
        {
            let mut kernel = kernel_with(&path);
            invoke(
                &mut kernel,
                ModelCommand::SetProviderAuthenticated {
                    provider_plugin: "provider.default".into(),
                    authenticated: true,
                },
            )
            .unwrap();
        }
        let mut restored = kernel_with(&path);
        let response = invoke(
            &mut restored,
            ModelCommand::SetProviderAuthenticated {
                provider_plugin: "provider.default".into(),
                authenticated: false,
            },
        )
        .unwrap();
        assert_eq!(
            response,
            ModelResponse::Authentication {
                provider_plugin: "provider.default".into(),
                authenticated: false
            }
        );
        let _ = fs::remove_file(path);
    }
}
