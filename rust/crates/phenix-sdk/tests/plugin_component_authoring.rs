use phenix_sdk::{
    Call, Emit, Host, Optional, Required, StaticComponentBehavior, StaticComponentImports,
    StaticPluginComponents,
};

#[phenix_sdk::interface("fixture.models.inference@1")]
struct ModelsInference;

struct ConsumerRequest;
struct ConsumerResponse;

const MODEL_POLICY_PRIORITY: i32 = 23;

#[phenix_sdk::component]
struct Api {
    #[phenix(import)]
    models: Required<Call<ModelsInference, ConsumerRequest, ConsumerResponse>>,

    #[phenix(import)]
    fallback: Optional<Call<ModelsInference, ConsumerRequest, ConsumerResponse>>,

    #[phenix(host)]
    model_host: Host<ModelsInference>,

    #[phenix(event("fixture.models.emitted"))]
    emitted: Emit<ConsumerResponse>,
}

#[phenix_sdk::component]
impl Api {
    #[phenix(export(ModelsInference), public)]
    fn run(&mut self, _request: ConsumerRequest) -> ConsumerResponse {
        ConsumerResponse
    }

    #[phenix(layer(ModelsInference, priority = MODEL_POLICY_PRIORITY))]
    fn model_policy(&mut self) {}

    #[phenix(listen("fixture.models.completed"))]
    fn model_completed(&mut self) {}

    #[phenix(value("fixture.component.status@1"), public)]
    fn status(&self) -> u64 {
        1
    }
}

#[phenix_sdk::plugin("fixture.component-owner")]
struct Plugin {
    #[phenix(component)]
    api: Api,
}

#[test]
fn component_fields_lower_to_typed_import_export_and_plugin_metadata() {
    let imports = <Api as StaticComponentImports>::imports();
    assert_eq!(imports.len(), 2);
    assert_eq!(imports[0].interface.as_str(), "fixture.models.inference@1");
    assert_eq!(imports[0].field, "models");
    assert!(imports[0].required);
    assert_eq!(imports[1].interface.as_str(), "fixture.models.inference@1");
    assert_eq!(imports[1].field, "fallback");
    assert!(!imports[1].required);

    let hosts = <Api as StaticComponentImports>::hosts();
    assert_eq!(hosts.len(), 1);
    assert_eq!(hosts[0].interface.as_str(), "fixture.models.inference@1");
    assert_eq!(hosts[0].field, "model_host");

    let events = <Api as StaticComponentImports>::events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event.as_str(), "fixture.models.emitted");
    assert_eq!(events[0].field, "emitted");
    assert!(events[0].payload_type.ends_with("::ConsumerResponse"));

    let exports = <Api as StaticComponentBehavior>::exports();
    assert_eq!(exports.len(), 1);
    assert_eq!(exports[0].interface.as_str(), "fixture.models.inference@1");
    assert_eq!(exports[0].method, "run");
    assert!(exports[0].public);

    let layers = <Api as StaticComponentBehavior>::layers();
    assert_eq!(layers.len(), 1);
    assert_eq!(layers[0].interface.as_str(), "fixture.models.inference@1");
    assert_eq!(layers[0].method, "model_policy");
    assert_eq!(layers[0].priority, MODEL_POLICY_PRIORITY);

    let listeners = <Api as StaticComponentBehavior>::listeners();
    assert_eq!(listeners.len(), 1);
    assert_eq!(listeners[0].event.as_str(), "fixture.models.completed");
    assert_eq!(listeners[0].method, "model_completed");

    let values = <Api as StaticComponentBehavior>::values();
    assert_eq!(values.len(), 1);
    assert_eq!(values[0].id.as_str(), "fixture.component.status@1");
    assert_eq!(values[0].method, "status");
    assert!(values[0].public);

    let components = <Plugin as StaticPluginComponents>::components();
    assert_eq!(components.len(), 1);
    assert_eq!(components[0].id.as_str(), "fixture.component-owner.api");
    assert_eq!(components[0].field, "api");
}

#[phenix_sdk::plugin("fixture.legacy-component-owner")]
struct LegacyPlugin {
    #[phenix(component, id = "legacy.component.api")]
    api: Api,
}

#[test]
fn explicit_component_identity_preserves_stable_runtime_id() {
    let components = <LegacyPlugin as StaticPluginComponents>::components();

    assert_eq!(components.len(), 1);
    assert_eq!(components[0].id.as_str(), "legacy.component.api");
}
