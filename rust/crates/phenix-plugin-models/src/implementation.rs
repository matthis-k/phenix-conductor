use phenix_core::{
    Authority, Bytes, CallableId, CapabilityId, ComponentInterface, DurableSchema, ModelId,
    PhenixValue, PluginContext, PluginExecution, PluginHost, PluginId, PluginInstance,
    PluginManifest, Project, ResourceNamespace, RoutingProfileId, ServiceContribution, ServiceId,
    TransactionOp,
};
pub use phenix_core::{ModelInferenceRequest, ModelInferenceResponse};
use phenix_sdk_macros::PhenixValue;
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

type ModelContext<'host, 'runtime, 'state> =
    PluginContext<'host, 'runtime, (), (), &'state mut BTreeSet<PluginId>>;

fn context<'host, 'runtime, 'state>(
    host: &'host PluginHost<'runtime>,
    authenticated: &'state mut BTreeSet<PluginId>,
) -> ModelContext<'host, 'runtime, 'state> {
    PluginContext::new(host, (), (), authenticated)
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, PhenixValue)]
pub struct ModelTarget {
    pub provider_plugin: PluginId,
    pub model: ModelId,
    #[serde(default)]
    pub options: BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, PhenixValue)]
pub struct RoutingProfile {
    pub id: RoutingProfileId,
    pub default_target: ModelTarget,
    #[serde(default)]
    pub callable_targets: BTreeMap<CallableId, ModelTarget>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, PhenixValue)]
pub struct RoutingProfileDescriptor {
    pub id: RoutingProfileId,
    pub providers: Vec<PluginId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, PhenixValue)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum ModelCommand {
    RegisterProfile {
        profile: RoutingProfile,
    },
    GetProfile {
        id: RoutingProfileId,
    },
    ListProfiles,
    SetProviderAuthenticated {
        provider_plugin: PluginId,
        authenticated: bool,
    },
    Resolve {
        profile_id: RoutingProfileId,
        callable_id: Option<CallableId>,
    },
    Invoke {
        profile_id: RoutingProfileId,
        callable_id: Option<CallableId>,
        input: Bytes,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, PhenixValue)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ModelResponse {
    Profile {
        profile: Option<RoutingProfile>,
    },
    Profiles {
        profiles: Vec<RoutingProfileDescriptor>,
    },
    Authentication {
        provider_plugin: PluginId,
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
            role: phenix_core::ServiceRole::Terminal,
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
        context(host, &mut self.authenticated)
            .kernel
            .register_durable_schema(&DurableSchema::new(model_namespace(), 1))
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
        let mut context = context(host, &mut self.authenticated);
        let interface = crate::ModelRoutingInterface::interface_id();
        let command = context
            .kernel
            .decode_projected::<ModelCommand>(&interface, input)
            .map_err(|error| error.to_string())?;
        let response = handle(&mut context, command)?;
        context
            .kernel
            .encode_value(&response)
            .map_err(|error| error.to_string())
    }
}

fn handle(
    context: &mut ModelContext<'_, '_, '_>,
    command: ModelCommand,
) -> Result<ModelResponse, String> {
    match command {
        ModelCommand::RegisterProfile { profile } => {
            insert_profile(context, &profile)?;
            Ok(ModelResponse::Profile {
                profile: Some(profile),
            })
        }
        ModelCommand::GetProfile { id } => Ok(ModelResponse::Profile {
            profile: read_profile(context, &id)?,
        }),
        ModelCommand::ListProfiles => Ok(ModelResponse::Profiles {
            profiles: load_profiles(context)?
                .into_iter()
                .map(|profile| descriptor(&profile))
                .collect(),
        }),
        ModelCommand::SetProviderAuthenticated {
            provider_plugin,
            authenticated,
        } => {
            if authenticated {
                context.plugin.state.insert(provider_plugin.clone());
            } else {
                context.plugin.state.remove(&provider_plugin);
            }
            Ok(ModelResponse::Authentication {
                provider_plugin,
                authenticated,
            })
        }
        ModelCommand::Resolve {
            profile_id,
            callable_id,
        } => Ok(ModelResponse::Target {
            target: resolve_target(context, &profile_id, callable_id.as_ref())?,
        }),
        ModelCommand::Invoke {
            profile_id,
            callable_id,
            input,
        } => {
            let target = resolve_target(context, &profile_id, callable_id.as_ref())?;
            if !context.plugin.state.contains(&target.provider_plugin) {
                return Err(format!(
                    "provider authentication required: {}",
                    target.provider_plugin
                ));
            }
            let request = ModelInferenceRequest {
                model: target.model.clone(),
                input,
                options: target.options.clone(),
            };
            let output = context
                .kernel
                .invoke_service_abi(
                    &model_inference_service(),
                    &serde_json::to_vec(&PhenixValue::from(&request))
                        .map_err(|error| error.to_string())?,
                    context.call.authority,
                    Some(&target.provider_plugin),
                )
                .map_err(|error| error.to_string())?;
            let output: PhenixValue =
                serde_json::from_slice(&output).map_err(|error| error.to_string())?;
            let response = ModelInferenceResponse::try_from(Project(&output))
                .map_err(|error| error.to_string())?;
            Ok(ModelResponse::Inference { target, response })
        }
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
    context: &ModelContext<'_, '_, '_>,
    profile_id: &RoutingProfileId,
    callable_id: Option<&CallableId>,
) -> Result<ModelTarget, String> {
    let profile = read_profile(context, profile_id)?
        .ok_or_else(|| format!("unknown routing profile: {profile_id}"))?;
    Ok(callable_id
        .and_then(|callable| profile.callable_targets.get(callable))
        .unwrap_or(&profile.default_target)
        .clone())
}

fn insert_profile(
    context: &ModelContext<'_, '_, '_>,
    profile: &RoutingProfile,
) -> Result<(), String> {
    let key = profile_key(&profile.id);
    let old_index = read_raw(context, PROFILE_INDEX)?;
    let mut ids: Vec<RoutingProfileId> = old_index
        .as_deref()
        .map(|value| serde_json::from_slice(value).map_err(|error| error.to_string()))
        .transpose()?
        .unwrap_or_default();
    if ids.contains(&profile.id) || read_raw(context, &key)?.is_some() {
        return Err(format!(
            "routing profile already registered: {}",
            profile.id
        ));
    }
    ids.push(profile.id.clone());
    ids.sort();
    context
        .kernel
        .transact_durable(
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

fn read_profile(
    context: &ModelContext<'_, '_, '_>,
    id: &RoutingProfileId,
) -> Result<Option<RoutingProfile>, String> {
    read_raw(context, &profile_key(id))?
        .map(|value| serde_json::from_slice(&value).map_err(|error| error.to_string()))
        .transpose()
}

fn load_profiles(context: &ModelContext<'_, '_, '_>) -> Result<Vec<RoutingProfile>, String> {
    let ids: Vec<RoutingProfileId> = read_raw(context, PROFILE_INDEX)?
        .as_deref()
        .map(|value| serde_json::from_slice(value).map_err(|error| error.to_string()))
        .transpose()?
        .unwrap_or_default();
    ids.into_iter()
        .map(|id| {
            read_profile(context, &id)?.ok_or_else(|| format!("missing routing profile: {id}"))
        })
        .collect()
}

fn read_raw(context: &ModelContext<'_, '_, '_>, key: &str) -> Result<Option<Vec<u8>>, String> {
    context
        .kernel
        .read_durable(&model_namespace(), key)
        .map_err(|error| error.to_string())
}

fn profile_key(id: &RoutingProfileId) -> String {
    format!("profile/{id}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_core::{Kernel, KernelConfig, LocalPersistence, PhenixValue, Project};
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
            provider_plugin: PluginId::parse(provider).unwrap(),
            model: ModelId::parse(model).unwrap(),
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
                &serde_json::to_vec(&PhenixValue::from(&command)).unwrap(),
                &routing_authority(),
                None,
            )
            .map_err(|error| error.to_string())?;
        {
            let output: PhenixValue =
                serde_json::from_slice(&output).map_err(|error| error.to_string())?;
            ModelResponse::try_from(Project(&output)).map_err(|error| error.to_string())
        }
    }

    #[test]
    fn routing_wire_ids_are_rejected_before_runtime_validation() {
        assert!(serde_json::from_value::<ModelCommand>(serde_json::json!({
            "operation": "get_profile",
            "id": "   "
        }))
        .is_err());
        assert!(serde_json::from_value::<ModelTarget>(serde_json::json!({
            "provider_plugin": "provider.default",
            "model": "",
            "options": {}
        }))
        .is_err());
        assert!(serde_json::from_value::<ModelTarget>(serde_json::json!({
            "provider_plugin": "has space",
            "model": "fixture",
            "options": {}
        }))
        .is_err());
    }

    #[test]
    fn routing_profiles_are_immutable_durable_and_callable_specific() {
        let path = temp_db("model-routing");
        let profile = RoutingProfile {
            id: RoutingProfileId::parse("default").unwrap(),
            default_target: target("provider.default", "root"),
            callable_targets: BTreeMap::from([(
                CallableId::parse("agent.scout").unwrap(),
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
                profile_id: RoutingProfileId::parse("default").unwrap(),
                callable_id: Some(CallableId::parse("agent.scout").unwrap()),
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
                    provider_plugin: PluginId::parse("provider.default").unwrap(),
                    authenticated: true,
                },
            )
            .unwrap();
        }
        let mut restored = kernel_with(&path);
        let response = invoke(
            &mut restored,
            ModelCommand::SetProviderAuthenticated {
                provider_plugin: PluginId::parse("provider.default").unwrap(),
                authenticated: false,
            },
        )
        .unwrap();
        assert_eq!(
            response,
            ModelResponse::Authentication {
                provider_plugin: PluginId::parse("provider.default").unwrap(),
                authenticated: false
            }
        );
        let _ = fs::remove_file(path);
    }
}
