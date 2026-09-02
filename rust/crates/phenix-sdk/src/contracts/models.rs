pub use phenix_core::{
    model_inference_service, ModelInferenceInterface, ModelInferenceRequest,
    ModelInferenceResponse, MODEL_INFERENCE_SERVICE,
};
use phenix_core::{
    Bytes, CallableId, ComponentInterface, InterfaceId, ModelId, PluginId, RoutingProfileId,
    ServiceId,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const MODEL_ROUTING_SERVICE: &str = "phenix.models.routing@1";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, phenix_sdk_macros::PhenixValue)]
pub struct ModelTarget {
    pub provider_plugin: PluginId,
    pub model: ModelId,
    #[serde(default)]
    pub options: BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, phenix_sdk_macros::PhenixValue)]
pub struct RoutingProfile {
    pub id: RoutingProfileId,
    pub default_target: ModelTarget,
    #[serde(default)]
    pub callable_targets: BTreeMap<CallableId, ModelTarget>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, phenix_sdk_macros::PhenixValue)]
pub struct RoutingProfileDescriptor {
    pub id: RoutingProfileId,
    pub providers: Vec<PluginId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, phenix_sdk_macros::PhenixValue)]
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, phenix_sdk_macros::PhenixValue)]
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

pub struct ModelRoutingInterface;

impl ComponentInterface for ModelRoutingInterface {
    fn interface_id() -> InterfaceId {
        InterfaceId::parse(MODEL_ROUTING_SERVICE)
            .expect("static model routing interface id is valid")
    }

    fn schema() -> phenix_core::InterfaceSchema {
        phenix_core::InterfaceSchema::of::<ModelCommand, ModelResponse>()
    }
}

#[must_use]
pub fn model_routing_service() -> ServiceId {
    ServiceId::parse(MODEL_ROUTING_SERVICE).expect("static model routing service id is valid")
}
