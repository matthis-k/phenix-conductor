use phenix_core::{
    model_inference_service, Authority, ComponentExport, ComponentId, ComponentInterface,
    ComponentManifest, InterfaceId, ModelInferenceRequest, ModelInferenceResponse, PluginExecution,
    PluginHost, PluginId, PluginInstance, PluginManifest, ServiceContribution, ServiceId,
    ServiceRole, MODEL_INFERENCE_SERVICE,
};
use std::collections::BTreeMap;

pub const BASIC_MODEL_PLUGIN: &str = "phenix.basic-model";
pub const BASIC_MODEL_COMPONENT: &str = "phenix.basic-model";

pub struct BasicModelInterface;

impl ComponentInterface for BasicModelInterface {
    fn interface_id() -> InterfaceId {
        InterfaceId::parse(MODEL_INFERENCE_SERVICE).expect("static model interface id is valid")
    }
}

#[must_use]
pub fn basic_model_manifest() -> PluginManifest {
    PluginManifest {
        id: PluginId::parse(BASIC_MODEL_PLUGIN).expect("static plugin id is valid"),
        version: 1,
        execution: PluginExecution::Embedded,
        dependencies: Vec::new(),
        services: vec![ServiceContribution {
            role: ServiceRole::Terminal,
            service: model_inference_service(),
            priority: 10,
            required_authority: Authority::default(),
        }],
        resource_namespaces: Vec::new(),
        maximum_authority: Authority::default(),
    }
}

#[must_use]
pub fn basic_model_component_manifest() -> ComponentManifest {
    ComponentManifest {
        id: ComponentId::parse(BASIC_MODEL_COMPONENT).expect("static component id is valid"),
        owner: basic_model_manifest().id,
        imports: Vec::new(),
        exports: vec![ComponentExport {
            interface: BasicModelInterface::interface_id(),
            schema: BasicModelInterface::schema(),
            priority: 10,
            required_authority: Authority::default(),
        }],
        maximum_authority: Authority::default(),
    }
}

#[must_use]
pub fn basic_model_factory() -> Box<dyn PluginInstance> {
    Box::new(BasicModel)
}

struct BasicModel;

impl PluginInstance for BasicModel {
    fn start(&mut self, _host: &PluginHost<'_>) -> Result<(), String> {
        Ok(())
    }

    fn invoke(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        _host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        if service != &model_inference_service() {
            return Err(format!("unsupported basic model service: {service}"));
        }
        let request: ModelInferenceRequest =
            serde_json::from_slice(input).map_err(|error| error.to_string())?;
        serde_json::to_vec(&ModelInferenceResponse {
            output: request.input,
            provider_metadata: BTreeMap::from([
                ("provider".into(), serde_json::json!(BASIC_MODEL_PLUGIN)),
                ("model".into(), serde_json::json!(request.model)),
            ]),
        })
        .map_err(|error| error.to_string())
    }
}
