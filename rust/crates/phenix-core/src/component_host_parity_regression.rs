use crate::{
    Authority, CapabilityId, ComponentExport, ComponentId, ComponentImport, ComponentManifest,
    InterfaceId, PluginExecution, PluginId, PluginManifest, ResolvedComponentGraph,
};

fn plugin(value: &str) -> PluginId {
    PluginId::parse(value).unwrap()
}

fn component(value: &str) -> ComponentId {
    ComponentId::parse(value).unwrap()
}

fn interface() -> InterfaceId {
    InterfaceId::parse("fixture.host-parity@1").unwrap()
}

fn capability(value: &str) -> CapabilityId {
    CapabilityId::parse(value).unwrap()
}

fn owner(id: &str, execution: PluginExecution, authority: Authority) -> PluginManifest {
    PluginManifest {
        id: plugin(id),
        version: 1,
        execution,
        dependencies: Vec::new(),
        services: Vec::new(),
        resource_namespaces: Vec::new(),
        maximum_authority: authority,
    }
}

fn graph(provider_execution: PluginExecution) -> ResolvedComponentGraph {
    let read = capability("fs.read");
    let write = capability("fs.write");
    let ceiling = Authority::new([read.clone(), write.clone()]);
    let plugins = [
        owner("consumer-owner", PluginExecution::Embedded, ceiling.clone()),
        owner(
            "provider-owner",
            provider_execution,
            Authority::new([read.clone()]),
        ),
    ];
    let components = [
        ComponentManifest {
            id: component("consumer"),
            owner: plugin("consumer-owner"),
            imports: vec![ComponentImport {
                interface: interface(),
                required: true,
                authority: ceiling.clone(),
            }],
            exports: Vec::new(),
            maximum_authority: ceiling.clone(),
        },
        ComponentManifest {
            id: component("provider"),
            owner: plugin("provider-owner"),
            imports: Vec::new(),
            exports: vec![ComponentExport {
                interface: interface(),
                priority: 10,
                required_authority: Authority::new([read.clone()]),
            }],
            maximum_authority: Authority::new([read]),
        },
    ];

    ResolvedComponentGraph::compile(plugins, components, &ceiling).unwrap()
}

fn assert_external_host_parity(executable: &str) {
    let embedded = graph(PluginExecution::Embedded);
    let external_execution = PluginExecution::External {
        executable: executable.into(),
    };
    let external = graph(external_execution.clone());

    let embedded_handle = embedded
        .import_handle(&component("consumer"), &interface())
        .unwrap()
        .unwrap();
    let external_handle = external
        .import_handle(&component("consumer"), &interface())
        .unwrap()
        .unwrap();

    assert_eq!(embedded_handle.exporter(), external_handle.exporter());
    assert_eq!(
        embedded_handle.effective_authority(),
        external_handle.effective_authority()
    );
    assert!(embedded_handle
        .effective_authority()
        .permits(&capability("fs.read")));
    assert!(!embedded_handle
        .effective_authority()
        .permits(&capability("fs.write")));
    assert_eq!(embedded_handle.execution(), &PluginExecution::Embedded);
    assert_eq!(external_handle.execution(), &external_execution);
}

#[test]
fn external_component_host_uses_the_same_binding_and_authority_semantics_as_embedded() {
    assert_external_host_parity("fixture-component-host");
}

#[test]
fn alternate_external_hosts_share_one_component_composition_model() {
    for executable in [
        "fixture-lua-component-host",
        "fixture-ipc-component-host",
        "fixture-wasm-component-host",
    ] {
        assert_external_host_parity(executable);
    }
}
