use phenix_core::{Authority, CapabilityId, ComponentInterface, PluginExecution, ServiceRole};
use phenix_plugin_options::{
    options_component_manifest, options_manifest, options_service, OptionsInterface,
    OPTIONS_COMPONENT, OPTIONS_PLUGIN,
};

fn capability(value: &str) -> CapabilityId {
    CapabilityId::parse(value).expect("test capability is valid")
}

#[test]
fn generated_authoring_preserves_the_public_runtime_contract() {
    let manifest = options_manifest();
    assert_eq!(manifest.id.as_str(), OPTIONS_PLUGIN);
    assert_eq!(manifest.version, 1);
    assert_eq!(manifest.execution, PluginExecution::Embedded);
    assert!(manifest.dependencies.is_empty());
    assert_eq!(manifest.services.len(), 1);
    assert_eq!(manifest.services[0].role, ServiceRole::Terminal);
    assert_eq!(manifest.services[0].service, options_service());
    assert_eq!(manifest.services[0].priority, 100);
    assert_eq!(
        manifest.services[0].required_authority,
        Authority::default()
    );
    assert_eq!(manifest.resource_namespaces.len(), 1);
    assert_eq!(
        manifest.resource_namespaces[0].as_str(),
        "phenix.options.state"
    );
    assert_eq!(
        manifest.maximum_authority,
        Authority::new([
            capability("kernel.persistence.schema"),
            capability("kernel.persistence.read"),
            capability("kernel.persistence.write"),
        ])
    );

    let component = options_component_manifest();
    assert_eq!(component.id.as_str(), OPTIONS_COMPONENT);
    assert_eq!(component.owner, manifest.id);
    assert!(component.imports.is_empty());
    assert_eq!(component.exports.len(), 1);
    assert_eq!(
        component.exports[0].interface,
        OptionsInterface::interface_id()
    );
    assert_eq!(component.exports[0].schema, OptionsInterface::schema());
    assert_eq!(component.exports[0].priority, 100);
    assert_eq!(
        component.exports[0].required_authority,
        Authority::default()
    );
    assert_eq!(component.maximum_authority, manifest.maximum_authority);
}
