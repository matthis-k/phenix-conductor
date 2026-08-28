use crate::{
    Authority, CapabilityId, ComponentExport, ComponentId, ComponentImport, ComponentInterface,
    ComponentManifest, InterfaceId, Kernel, KernelConfig, PluginExecution, PluginHost, PluginId,
    PluginInstance, PluginManifest, ResolvedHarness, ResolvedHarnessActivation,
    ServiceContribution, ServiceId, ServiceRole,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
struct EchoRequest {
    value: String,
}
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
struct EchoResponse {
    provider: String,
    value: String,
}

struct EchoInterface;
impl ComponentInterface for EchoInterface {
    type Request = EchoRequest;
    type Response = EchoResponse;
    fn interface_id() -> InterfaceId {
        InterfaceId::parse("fixture.echo@1").unwrap()
    }
}

struct Provider(&'static str);
impl PluginInstance for Provider {
    fn start(&mut self, _host: &PluginHost<'_>) -> Result<(), String> {
        Ok(())
    }
    fn invoke(
        &mut self,
        _service: &ServiceId,
        input: &[u8],
        _host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        let request: EchoRequest = serde_json::from_slice(input).unwrap();
        serde_json::to_vec(&EchoResponse {
            provider: self.0.into(),
            value: request.value,
        })
        .map_err(|error| error.to_string())
    }
}

struct Consumer;
impl PluginInstance for Consumer {
    fn start(&mut self, _host: &PluginHost<'_>) -> Result<(), String> {
        Ok(())
    }
    fn invoke(
        &mut self,
        _service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        let request: EchoRequest = serde_json::from_slice(input).unwrap();
        let response = host
            .invoke_import::<EchoInterface>(&component("consumer"), &request)
            .map_err(|error| error.to_string())?;
        serde_json::to_vec(&response).map_err(|error| error.to_string())
    }
}

fn plugin(value: &str) -> PluginId {
    PluginId::parse(value).unwrap()
}
fn component(value: &str) -> ComponentId {
    ComponentId::parse(value).unwrap()
}
fn interface(value: &str) -> InterfaceId {
    InterfaceId::parse(value).unwrap()
}
fn service(value: &str) -> ServiceId {
    ServiceId::parse(value).unwrap()
}
fn capability(value: &str) -> CapabilityId {
    CapabilityId::parse(value).unwrap()
}

fn plugin_manifest(id: &str, service_id: &str, priority: i32) -> PluginManifest {
    PluginManifest {
        id: plugin(id),
        version: 1,
        execution: PluginExecution::Embedded,
        dependencies: Vec::new(),
        services: vec![ServiceContribution {
            role: ServiceRole::Terminal,
            service: service(service_id),
            priority,
            required_authority: Authority::default(),
        }],
        resource_namespaces: Vec::new(),
        maximum_authority: Authority::default(),
    }
}

#[test]
fn plugin_host_uses_pre_resolved_typed_binding_instead_of_global_provider_order() {
    let selected = plugin_manifest("selected-provider", "fixture.echo@1", 1);
    let global = plugin_manifest("global-preferred", "fixture.echo@1", 100);
    let consumer = plugin_manifest("consumer-plugin", "fixture.consumer@1", 1);
    let components = vec![
        ComponentManifest {
            id: component("selected"),
            owner: selected.id.clone(),
            imports: Vec::new(),
            exports: vec![ComponentExport {
                interface: interface("fixture.echo@1"),
                priority: 100,
                required_authority: Authority::default(),
            }],
            maximum_authority: Authority::default(),
        },
        ComponentManifest {
            id: component("decoy"),
            owner: global.id.clone(),
            imports: Vec::new(),
            exports: vec![ComponentExport {
                interface: interface("fixture.echo@1"),
                priority: 1,
                required_authority: Authority::default(),
            }],
            maximum_authority: Authority::default(),
        },
        ComponentManifest {
            id: component("consumer"),
            owner: consumer.id.clone(),
            imports: vec![ComponentImport {
                interface: interface("fixture.echo@1"),
                required: true,
                authority: Authority::default(),
            }],
            exports: Vec::new(),
            maximum_authority: Authority::default(),
        },
    ];
    let manifests = [selected.clone(), global.clone(), consumer.clone()];
    let resolved =
        ResolvedHarness::resolve(manifests.clone(), components, [], &Authority::default()).unwrap();
    let mut kernel = Kernel::new(KernelConfig::new(manifests).unwrap());
    kernel.activate_resolved_harness(&resolved).unwrap();
    kernel
        .register_embedded_factory(selected.id.clone(), || Box::new(Provider("selected")))
        .unwrap();
    kernel
        .register_embedded_factory(global.id.clone(), || Box::new(Provider("global")))
        .unwrap();
    kernel
        .register_embedded_factory(consumer.id.clone(), || Box::new(Consumer))
        .unwrap();
    kernel.activate_all().unwrap();

    let output = kernel
        .invoke(
            &service("fixture.consumer@1"),
            &serde_json::to_vec(&EchoRequest {
                value: "hello".into(),
            })
            .unwrap(),
            &Authority::default(),
            None,
        )
        .unwrap();
    let response: EchoResponse = serde_json::from_slice(&output).unwrap();
    assert_eq!(response.provider, "selected");
    assert_eq!(response.value, "hello");
    assert_eq!(kernel.graph_generation(), Some(resolved.generation()));
}

#[test]
fn external_component_uses_the_same_typed_binding_and_authority_attenuation() {
    let read = capability("workspace.read");
    let write = capability("workspace.write");
    let network = capability("network.read");

    let external_provider = PluginManifest {
        id: plugin("external-provider"),
        version: 1,
        execution: PluginExecution::External {
            executable: "fixture-external-host".into(),
        },
        dependencies: Vec::new(),
        services: Vec::new(),
        resource_namespaces: Vec::new(),
        maximum_authority: Authority::new([read.clone(), network]),
    };
    let consumer = PluginManifest {
        id: plugin("consumer-plugin"),
        version: 1,
        execution: PluginExecution::Embedded,
        dependencies: Vec::new(),
        services: Vec::new(),
        resource_namespaces: Vec::new(),
        maximum_authority: Authority::new([read.clone(), write.clone()]),
    };
    let interface = interface("fixture.external@1");
    let components = [
        ComponentManifest {
            id: component("external-provider"),
            owner: external_provider.id.clone(),
            imports: Vec::new(),
            exports: vec![ComponentExport {
                interface: interface.clone(),
                priority: 100,
                required_authority: Authority::new([read.clone()]),
            }],
            maximum_authority: Authority::new([read.clone()]),
        },
        ComponentManifest {
            id: component("consumer"),
            owner: consumer.id.clone(),
            imports: vec![ComponentImport {
                interface: interface.clone(),
                required: true,
                authority: Authority::new([read.clone(), write]),
            }],
            exports: Vec::new(),
            maximum_authority: Authority::new([read.clone()]),
        },
    ];

    let resolved = ResolvedHarness::resolve(
        [external_provider, consumer],
        components,
        [],
        &Authority::new([read.clone(), capability("workspace.write")]),
    )
    .unwrap();
    let handle = resolved
        .component_graph()
        .import_handle(&component("consumer"), &interface)
        .unwrap()
        .unwrap();

    assert_eq!(handle.exporter(), &component("external-provider"));
    assert_eq!(
        handle.execution(),
        &PluginExecution::External {
            executable: "fixture-external-host".into(),
        }
    );
    assert!(handle.effective_authority().permits(&read));
    assert!(!handle
        .effective_authority()
        .permits(&capability("workspace.write")));
    assert!(!handle
        .effective_authority()
        .permits(&capability("network.read")));
}
