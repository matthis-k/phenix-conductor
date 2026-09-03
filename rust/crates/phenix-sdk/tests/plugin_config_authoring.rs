use phenix_sdk::{PhenixSchema, StaticPluginConfigDescriptor, StaticPluginConfiguration};

#[derive(Clone, Debug, Eq, PartialEq, phenix_sdk::PhenixValue)]
struct Settings {
    retries: u64,
}

#[allow(dead_code)]
#[phenix_sdk::plugin("fixture.configured")]
struct Configured {
    #[phenix(config)]
    config: Settings,
}

#[test]
fn plugin_config_field_lowers_to_typed_structural_schema() {
    let descriptor = <Configured as StaticPluginConfiguration>::configuration()
        .expect("configured plugin exposes configuration metadata");

    assert_eq!(descriptor.field, "config");
    assert!(descriptor.config_type.ends_with("::Settings"));
    assert!(matches!(descriptor.schema, PhenixSchema::Table(_)));
}

#[test]
fn descriptor_uses_the_same_schema_as_the_config_type() {
    let direct = StaticPluginConfigDescriptor::of::<Settings>("config");
    let generated = <Configured as StaticPluginConfiguration>::configuration().unwrap();

    assert_eq!(generated, direct);
}
