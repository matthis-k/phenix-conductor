use crate::{basic_model_component_manifest, basic_model_factory, basic_model_manifest};
use phenix_core::{
    Authority, ComponentExport, ComponentId, ComponentImport, ComponentInterface,
    ComponentManifest, ExternalPluginProcess, ExternalSandbox, ExternalTransportConfig, Kernel,
    KernelConfig, ModelId, PhenixValue, PluginExecution, PluginHost, PluginId, PluginInstance,
    PluginManifest, Project, ResolvedHarness, ResolvedHarnessActivation, ServiceContribution,
    ServiceId, ServiceRole,
};
use phenix_sdk::{
    model_inference_service, ModelInferenceInterface, ModelInferenceRequest, ModelInferenceResponse,
};
use std::{
    collections::BTreeMap,
    io,
    process::{Child, Command, Stdio},
    sync::Arc,
    time::Duration,
};

const CONSUMER_SERVICE: &str = "fixture.basic-model-consumer@1";

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
        let request: ModelInferenceRequest =
            serde_json::from_slice(input).map_err(|error| error.to_string())?;
        let response = host
            .invoke_import::<ModelInferenceInterface>(
                &component("fixture.basic-model-consumer"),
                &PhenixValue::from(&request),
            )
            .map_err(|error| error.to_string())?;
        let response = ModelInferenceResponse::try_from(Project(&response))
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

fn external_manifest() -> PluginManifest {
    PluginManifest {
        id: plugin("fixture.external-basic-model"),
        version: 1,
        execution: PluginExecution::External {
            executable: "fixture-external-basic-model".into(),
        },
        dependencies: Vec::new(),
        services: vec![ServiceContribution {
            role: ServiceRole::Terminal,
            service: model_inference_service(),
            priority: 100,
            required_authority: Authority::default(),
        }],
        resource_namespaces: Vec::new(),
        maximum_authority: Authority::default(),
    }
}

fn external_component() -> ComponentManifest {
    ComponentManifest {
        id: component("fixture.external-basic-model"),
        owner: external_manifest().id,
        imports: Vec::new(),
        exports: vec![ComponentExport {
            interface: ModelInferenceInterface::interface_id(),
            schema: ModelInferenceInterface::schema(),
            priority: 100,
            required_authority: Authority::default(),
        }],
        maximum_authority: Authority::default(),
    }
}

fn consumer_manifest() -> PluginManifest {
    PluginManifest {
        id: plugin("fixture.basic-model-consumer"),
        version: 1,
        execution: PluginExecution::Embedded,
        dependencies: Vec::new(),
        services: vec![ServiceContribution {
            role: ServiceRole::Terminal,
            service: service(CONSUMER_SERVICE),
            priority: 1,
            required_authority: Authority::default(),
        }],
        resource_namespaces: Vec::new(),
        maximum_authority: Authority::default(),
    }
}

fn consumer_component() -> ComponentManifest {
    ComponentManifest {
        id: component("fixture.basic-model-consumer"),
        owner: consumer_manifest().id,
        imports: vec![ComponentImport {
            interface: ModelInferenceInterface::interface_id(),
            schema: ModelInferenceInterface::schema(),
            required: true,
            authority: Authority::default(),
        }],
        exports: Vec::new(),
        maximum_authority: Authority::default(),
    }
}

#[test]
fn external_component_replaces_basic_model_without_changing_the_consumer_contract() {
    let basic = basic_model_manifest();
    let external = external_manifest();
    let consumer = consumer_manifest();
    let manifests = [basic.clone(), external.clone(), consumer.clone()];
    let resolved = ResolvedHarness::resolve(
        manifests.clone(),
        [
            basic_model_component_manifest(),
            external_component(),
            consumer_component(),
        ],
        [],
        &Authority::default(),
    )
    .unwrap();
    let binding = resolved
        .component_graph()
        .import_handle(
            &component("fixture.basic-model-consumer"),
            &ModelInferenceInterface::interface_id(),
        )
        .unwrap()
        .unwrap();
    assert_eq!(
        binding.exporter(),
        &component("fixture.external-basic-model")
    );
    assert_eq!(binding.owning_plugin(), &external.id);

    let script = r#"
        read handshake
        generation=${handshake#*\"generation\":}
        generation=${generation%%,*}
        echo "{\"type\":\"handshake_ok\",\"protocol\":3,\"plugin\":\"fixture.external-basic-model\",\"generation\":$generation,\"services\":[{\"service\":\"phenix.models.inference@1\",\"role\":\"terminal\"}]}"
        read request
        echo "{\"type\":\"result\",\"request_id\":1,\"generation\":$generation,\"output\":[123,34,116,121,112,101,34,58,34,116,97,98,108,101,34,44,34,118,97,108,117,101,34,58,123,34,111,117,116,112,117,116,34,58,123,34,116,121,112,101,34,58,34,98,121,116,101,115,34,44,34,118,97,108,117,101,34,58,91,49,48,49,44,49,50,48,44,49,49,54,44,49,48,49,44,49,49,52,44,49,49,48,44,57,55,44,49,48,56,93,125,44,34,112,114,111,118,105,100,101,114,95,109,101,116,97,100,97,116,97,34,58,123,34,116,121,112,101,34,58,34,109,97,112,34,44,34,118,97,108,117,101,34,58,123,34,112,114,111,118,105,100,101,114,34,58,123,34,116,121,112,101,34,58,34,115,116,114,105,110,103,34,44,34,118,97,108,117,101,34,58,34,101,120,116,101,114,110,97,108,34,125,125,125,125,125]}"
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
        .register_embedded_factory(basic.id, basic_model_factory)
        .unwrap();
    kernel
        .register_external_factory(external.id.clone(), move |manifest| {
            Ok(Box::new(ExternalPluginProcess::new(
                manifest.clone(),
                "fixture-external-basic-model",
                transport.clone(),
            )))
        })
        .unwrap();
    kernel
        .register_embedded_factory(consumer.id, || Box::new(Consumer))
        .unwrap();
    kernel.activate_all().unwrap();

    let request = ModelInferenceRequest {
        model: ModelId::parse("same-request").unwrap(),
        input: b"hello".to_vec().into(),
        options: BTreeMap::new(),
    };
    let output = kernel
        .invoke(
            &service(CONSUMER_SERVICE),
            &serde_json::to_vec(&request).unwrap(),
            &Authority::default(),
            None,
        )
        .unwrap();
    let response: ModelInferenceResponse = serde_json::from_slice(&output).unwrap();
    assert_eq!(response.output.as_ref(), b"external");
    assert_eq!(
        response.provider_metadata.get("provider"),
        Some(&serde_json::json!("external"))
    );
}
