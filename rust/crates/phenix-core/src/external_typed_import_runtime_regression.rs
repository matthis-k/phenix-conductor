use crate::{
    Authority, CapabilityId, ComponentExport, ComponentId, ComponentImport, ComponentInterface,
    ComponentManifest, ExternalPluginProcess, ExternalSandbox, ExternalTransportConfig,
    InterfaceId, Kernel, KernelConfig, PluginExecution, PluginHost, PluginId, PluginInstance,
    PluginManifest, ResolvedHarness, ResolvedHarnessActivation, ServiceContribution, ServiceId,
    ServiceRole,
};
use serde::{Deserialize, Serialize};
use std::{
    io,
    process::{Child, Command, Stdio},
    sync::Arc,
    time::Duration,
};

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
        InterfaceId::parse("fixture.external-typed@1").unwrap()
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
        let request: EchoRequest =
            serde_json::from_slice(input).map_err(|error| error.to_string())?;
        let response = host
            .invoke_import::<EchoInterface>(&component("consumer"), &request)
            .map_err(|error| error.to_string())?;
        serde_json::to_vec(&response).map_err(|error| error.to_string())
    }
}

#[derive(Clone)]
struct ScriptSandbox {
    script: String,
}

impl ExternalSandbox for ScriptSandbox {
    fn spawn(&self, _executable: &str) -> io::Result<Child> {
        Command::new("sh")
            .arg("-c")
            .arg(&self.script)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
    }
}

fn plugin(value: &str) -> PluginId {
    PluginId::parse(value).unwrap()
}

fn component(value: &str) -> ComponentId {
    ComponentId::parse(value).unwrap()
}

fn service(value: &str) -> ServiceId {
    ServiceId::parse(value).unwrap()
}

fn capability(value: &str) -> CapabilityId {
    CapabilityId::parse(value).unwrap()
}

#[test]
fn typed_component_import_executes_through_the_external_process_host() {
    let read = capability("workspace.read");
    let write = capability("workspace.write");
    let provider_authority = Authority::new([read.clone()]);
    let consumer_authority = Authority::new([read.clone(), write.clone()]);
    let external = PluginManifest {
        id: plugin("external-provider"),
        version: 1,
        execution: PluginExecution::External {
            executable: "fixture-external-host".into(),
        },
        dependencies: Vec::new(),
        services: vec![ServiceContribution {
            role: ServiceRole::Terminal,
            service: service("fixture.external-typed@1"),
            priority: 1,
            required_authority: Authority::default(),
        }],
        resource_namespaces: Vec::new(),
        maximum_authority: provider_authority.clone(),
    };
    let consumer = PluginManifest {
        id: plugin("consumer-plugin"),
        version: 1,
        execution: PluginExecution::Embedded,
        dependencies: Vec::new(),
        services: vec![ServiceContribution {
            role: ServiceRole::Terminal,
            service: service("fixture.consumer@1"),
            priority: 1,
            required_authority: Authority::default(),
        }],
        resource_namespaces: Vec::new(),
        maximum_authority: consumer_authority.clone(),
    };
    let components = [
        ComponentManifest {
            id: component("external-provider"),
            owner: external.id.clone(),
            imports: Vec::new(),
            exports: vec![ComponentExport {
                interface: EchoInterface::interface_id(),
                priority: 100,
                required_authority: provider_authority.clone(),
            }],
            maximum_authority: provider_authority.clone(),
        },
        ComponentManifest {
            id: component("consumer"),
            owner: consumer.id.clone(),
            imports: vec![ComponentImport {
                interface: EchoInterface::interface_id(),
                required: true,
                authority: consumer_authority.clone(),
            }],
            exports: Vec::new(),
            maximum_authority: consumer_authority.clone(),
        },
    ];
    let manifests = [external.clone(), consumer.clone()];
    let resolved =
        ResolvedHarness::resolve(manifests.clone(), components, [], &consumer_authority).unwrap();
    let script = r#"
        read handshake
        generation=${handshake#*\"generation\":}
        generation=${generation%%,*}
        echo "{\"type\":\"handshake_ok\",\"protocol\":3,\"plugin\":\"external-provider\",\"generation\":$generation,\"services\":[{\"service\":\"fixture.external-typed@1\",\"role\":\"terminal\"}]}"
        read request
        case "$request" in
          *'"authority":["workspace.read"]'*) ;;
          *) echo "{\"type\":\"error\",\"request_id\":1,\"generation\":$generation,\"message\":\"authority was not attenuated\"}"; exit 0 ;;
        esac
        echo "{\"type\":\"result\",\"request_id\":1,\"generation\":$generation,\"output\":[123,34,112,114,111,118,105,100,101,114,34,58,34,101,120,116,101,114,110,97,108,34,44,34,118,97,108,117,101,34,58,34,104,101,108,108,111,34,125]}"
        read stop || true
    "#;
    let transport = ExternalTransportConfig::new(
        Arc::new(ScriptSandbox {
            script: script.into(),
        }),
        Duration::from_secs(2),
    );
    let mut kernel = Kernel::new(KernelConfig::new(manifests).unwrap());
    kernel.activate_resolved_harness(&resolved).unwrap();
    kernel
        .register_external_factory(external.id.clone(), move |manifest| {
            Ok(Box::new(ExternalPluginProcess::new(
                manifest.clone(),
                "fixture-external-host",
                transport.clone(),
            )))
        })
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
            &consumer_authority,
            None,
        )
        .unwrap();
    let response: EchoResponse = serde_json::from_slice(&output).unwrap();

    assert_eq!(
        response,
        EchoResponse {
            provider: "external".into(),
            value: "hello".into(),
        }
    );
    assert_eq!(kernel.graph_generation(), Some(resolved.generation()));
}
