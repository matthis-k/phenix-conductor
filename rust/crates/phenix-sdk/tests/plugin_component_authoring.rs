use phenix_sdk::{
    Call, Optional, Required, StaticComponentBehavior, StaticComponentImports,
    StaticPluginComponents,
};

#[phenix_sdk::interface("fixture.models.inference@1")]
struct ModelsInference;

struct ConsumerRequest;
struct ConsumerResponse;

#[phenix_sdk::component]
struct Api {
    #[phenix(import)]
    models: Required<Call<ModelsInference, ConsumerRequest, ConsumerResponse>>,

    #[phenix(import)]
    fallback: Optional<Call<ModelsInference, ConsumerRequest, ConsumerResponse>>,
}

#[phenix_sdk::component]
impl Api {
    #[phenix(export(ModelsInference), public)]
    fn run(&mut self, _request: ConsumerRequest) -> ConsumerResponse {
        ConsumerResponse
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

    let exports = <Api as StaticComponentBehavior>::exports();
    assert_eq!(exports.len(), 1);
    assert_eq!(exports[0].interface.as_str(), "fixture.models.inference@1");
    assert_eq!(exports[0].method, "run");
    assert!(exports[0].public);

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
