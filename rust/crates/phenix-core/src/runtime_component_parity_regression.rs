use crate::{
    runtime_provider_service, Authority, ComponentExport, ComponentId, ComponentImport,
    ComponentInterface, ComponentManifest, InterfaceId, Kernel, PhenixValue, PluginArtifact,
    PluginExecution, PluginHost, PluginId, PluginInstance, PluginManifest, PluginRuntimeProvider,
    ResolvedHarness, ResolvedHarnessActivation, ResolvedImportHandle, RuntimeId,
    RuntimePluginCandidate, ServiceContribution, ServiceId, ServiceRole,
};
use std::collections::BTreeMap;

fn plugin(value: &str) -> PluginId {
    PluginId::parse(value).unwrap()
}

fn component(value: &str) -> ComponentId {
    ComponentId::parse(value).unwrap()
}

struct EchoInterface;

impl ComponentInterface for EchoInterface {
    fn interface_id() -> InterfaceId {
        InterfaceId::parse("fixture.echo@1").unwrap()
    }
}

struct Noop;

impl PluginInstance for Noop {
    fn start(&mut self, _host: &PluginHost<'_>) -> Result<(), String> {
        Ok(())
    }
}

struct EchoProvider(&'static str);

impl PluginInstance for EchoProvider {
    fn start(&mut self, _host: &PluginHost<'_>) -> Result<(), String> {
        Ok(())
    }

    fn invoke(
        &mut self,
        _service: &ServiceId,
        input: &[u8],
        _host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        let request: PhenixValue =
            serde_json::from_slice(input).map_err(|error| error.to_string())?;
        let PhenixValue::String(value) = request else {
            return Err("expected string request".into());
        };
        serde_json::to_vec(&PhenixValue::String(format!("{}:{value}", self.0)))
            .map_err(|error| error.to_string())
    }
}

struct EchoRuntimeBridge;

impl PluginRuntimeProvider for EchoRuntimeBridge {
    fn prepare(
        &mut self,
        _candidate: RuntimePluginCandidate<'_>,
    ) -> Result<Box<dyn PluginInstance>, String> {
        Ok(Box::new(EchoProvider("runtime")))
    }
}

impl PluginInstance for EchoRuntimeBridge {
    fn start(&mut self, _host: &PluginHost<'_>) -> Result<(), String> {
        Ok(())
    }

    fn runtime_provider(&mut self) -> Option<&mut dyn PluginRuntimeProvider> {
        Some(self)
    }
}

fn consumer_manifest() -> PluginManifest {
    PluginManifest {
        id: plugin("fixture.consumer"),
        version: 1,
        execution: PluginExecution::Embedded,
        dependencies: Vec::new(),
        services: Vec::new(),
        resource_namespaces: Vec::new(),
        maximum_authority: Authority::default(),
    }
}

fn embedded_provider_manifest() -> PluginManifest {
    PluginManifest {
        id: plugin("fixture.provider"),
        version: 1,
        execution: PluginExecution::Embedded,
        dependencies: Vec::new(),
        services: Vec::new(),
        resource_namespaces: Vec::new(),
        maximum_authority: Authority::default(),
    }
}

fn runtime_bridge_manifest(runtime: &RuntimeId) -> PluginManifest {
    PluginManifest {
        id: plugin("fixture.runtime-bridge"),
        version: 1,
        execution: PluginExecution::Embedded,
        dependencies: Vec::new(),
        services: vec![ServiceContribution {
            service: runtime_provider_service(runtime),
            role: ServiceRole::Terminal,
            priority: 0,
            required_authority: Authority::default(),
        }],
        resource_namespaces: Vec::new(),
        maximum_authority: Authority::default(),
    }
}

fn runtime_provider_manifest(runtime: RuntimeId) -> PluginManifest {
    PluginManifest {
        id: plugin("fixture.provider"),
        version: 1,
        execution: PluginExecution::Runtime {
            runtime,
            artifact: PluginArtifact {
                locator: "fixture.echo".into(),
                revision: crate::ArtifactRevision::from_content(b"echo-v1"),
                configuration: BTreeMap::new(),
            },
        },
        dependencies: Vec::new(),
        services: Vec::new(),
        resource_namespaces: Vec::new(),
        maximum_authority: Authority::default(),
    }
}

fn components() -> [ComponentManifest; 2] {
    [
        ComponentManifest {
            listeners: Vec::new(),
            id: component("fixture.consumer"),
            owner: plugin("fixture.consumer"),
            imports: vec![ComponentImport {
                interface: EchoInterface::interface_id(),
                schema: EchoInterface::schema(),
                required: true,
                authority: Authority::default(),
            }],
            exports: Vec::new(),
            maximum_authority: Authority::default(),
        },
        ComponentManifest {
            listeners: Vec::new(),
            id: component("fixture.provider"),
            owner: plugin("fixture.provider"),
            imports: Vec::new(),
            exports: vec![ComponentExport {
                interface: EchoInterface::interface_id(),
                schema: EchoInterface::schema(),
                priority: 0,
                required_authority: Authority::default(),
            }],
            maximum_authority: Authority::default(),
        },
    ]
}

fn handle(resolved: &ResolvedHarness) -> ResolvedImportHandle {
    resolved
        .component_graph()
        .import_handle(
            &component("fixture.consumer"),
            &EchoInterface::interface_id(),
        )
        .unwrap()
        .unwrap()
        .clone()
}

fn invoke(handle: &ResolvedImportHandle, kernel: &mut Kernel) -> PhenixValue {
    handle
        .invoke_value::<EchoInterface>(kernel, &PhenixValue::String("hello".into()))
        .unwrap()
}

#[test]
fn typed_component_consumer_is_runtime_agnostic() {
    let consumer = consumer_manifest();
    let embedded_provider = embedded_provider_manifest();
    let embedded_resolved = ResolvedHarness::resolve(
        [consumer.clone(), embedded_provider.clone()],
        components(),
        [],
        &Authority::default(),
    )
    .unwrap();
    let embedded_handle = handle(&embedded_resolved);
    let mut embedded_kernel = Kernel::new(embedded_resolved.kernel_config().clone());
    embedded_kernel
        .activate_resolved_harness(&embedded_resolved)
        .unwrap();
    embedded_kernel
        .register_embedded_factory(consumer.id.clone(), || Box::new(Noop))
        .unwrap();
    embedded_kernel
        .register_embedded_factory(embedded_provider.id, || Box::new(EchoProvider("embedded")))
        .unwrap();
    embedded_kernel.activate_all().unwrap();

    let runtime = RuntimeId::parse("fixture.runtime").unwrap();
    let bridge = runtime_bridge_manifest(&runtime);
    let runtime_provider = runtime_provider_manifest(runtime);
    let runtime_resolved = ResolvedHarness::resolve(
        [consumer.clone(), bridge.clone(), runtime_provider],
        components(),
        [],
        &Authority::default(),
    )
    .unwrap();
    let runtime_handle = handle(&runtime_resolved);
    let mut runtime_kernel = Kernel::new(runtime_resolved.kernel_config().clone());
    runtime_kernel
        .activate_resolved_harness(&runtime_resolved)
        .unwrap();
    runtime_kernel
        .register_embedded_factory(consumer.id, || Box::new(Noop))
        .unwrap();
    runtime_kernel
        .register_embedded_factory(bridge.id, || Box::new(EchoRuntimeBridge))
        .unwrap();
    runtime_kernel.activate_all().unwrap();

    assert_eq!(
        invoke(&embedded_handle, &mut embedded_kernel),
        PhenixValue::String("embedded:hello".into())
    );
    assert_eq!(
        invoke(&runtime_handle, &mut runtime_kernel),
        PhenixValue::String("runtime:hello".into())
    );
}
