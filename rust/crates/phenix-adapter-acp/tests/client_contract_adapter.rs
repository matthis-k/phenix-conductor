use phenix_adapter_acp::Plugin;

#[test]
fn adapter_public_surface_is_the_canonical_runtime_plugin() {
    let manifest = <Plugin as phenix_sdk::StaticPluginDefinition>::manifest();

    assert_eq!(manifest.id.as_str(), "phenix.adapter.acp");
    assert!(matches!(
        manifest.execution,
        phenix_sdk::PluginExecution::Embedded
    ));
    assert!(manifest.dependencies.is_empty());
    assert!(manifest.resource_namespaces.is_empty());
}
