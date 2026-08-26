use phenix_harness::{default_suite_authority, HarnessBuilder, PhenixHarness};
use phenix_kernel::{
    Authority, PluginExecution, PluginHost, PluginId, PluginInstance, PluginManifest,
    ServiceContribution, ServiceId,
};
use phenix_plugin_suite::{
    execution_service, model_inference_service, model_routing_service, ExecutionAuthority,
    ExecutionCommand, ExecutionResponse, ModelCommand, ModelInferenceRequest,
    ModelInferenceResponse, ModelResponse, ModelTarget, RoutingProfile,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn invoke(harness: &mut PhenixHarness, service: &str, input: Value) -> Value {
    let service = ServiceId::parse(service).unwrap();
    let output = harness
        .invoke(
            &service,
            &serde_json::to_vec(&input).unwrap(),
            &default_suite_authority(),
            None,
        )
        .unwrap_or_else(|error| panic!("{service}: {error}"));
    serde_json::from_slice(&output).unwrap_or_else(|error| panic!("{service}: {error}"))
}

fn fixture_manifest(id: &str, service: ServiceId) -> PluginManifest {
    PluginManifest {
        id: PluginId::parse(id).unwrap(),
        version: 1,
        execution: PluginExecution::Embedded,
        dependencies: Vec::new(),
        services: vec![ServiceContribution {
            service,
            priority: 100,
            required_authority: Authority::default(),
        }],
        resource_namespaces: Vec::new(),
        maximum_authority: Authority::default(),
    }
}

struct ModelProvider;

impl PluginInstance for ModelProvider {
    fn start(&mut self, _host: &PluginHost<'_>) -> Result<(), String> {
        Ok(())
    }

    fn invoke(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        _host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        if service != &model_inference_service() {
            return Err(format!("unsupported fixture model service: {service}"));
        }
        let request: ModelInferenceRequest =
            serde_json::from_slice(input).map_err(|error| error.to_string())?;
        let response = ModelInferenceResponse {
            output: [b"answer:".as_slice(), request.input.as_slice()].concat(),
            provider_metadata: BTreeMap::from([(
                "model".into(),
                serde_json::Value::String(request.model),
            )]),
        };
        serde_json::to_vec(&response).map_err(|error| error.to_string())
    }
}

struct EchoTool;

impl PluginInstance for EchoTool {
    fn start(&mut self, _host: &PluginHost<'_>) -> Result<(), String> {
        Ok(())
    }

    fn invoke(
        &mut self,
        _service: &ServiceId,
        input: &[u8],
        _host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        Ok(input.to_vec())
    }
}

#[test]
fn supported_harness_routes_first_party_domains_through_kernel_services() {
    let mut harness = PhenixHarness::default_suite().unwrap();
    harness.activate().unwrap();

    assert_eq!(
        harness.kernel().config().manifests().count(),
        14,
        "the supported Harness owns the complete first-party suite",
    );

    let repository = invoke(
        &mut harness,
        "phenix.repository.worker-queue@1",
        json!({"pull_requests": [], "issues": []}),
    );
    assert!(repository.is_null());

    let sessions = invoke(
        &mut harness,
        "phenix.sessions@1",
        json!({"operation": "list"}),
    );
    assert_eq!(sessions["result"], "sessions");

    let artifact = invoke(
        &mut harness,
        "phenix.artifacts@1",
        json!({
            "operation": "get",
            "id": "missing",
            "content_identity": "missing"
        }),
    );
    assert_eq!(artifact["response"], "artifact");
    assert!(artifact["artifact"].is_null());

    let context = invoke(
        &mut harness,
        "phenix.context@1",
        json!({"operation": "list"}),
    );
    assert_eq!(context["result"], "resources");

    let execution = invoke(
        &mut harness,
        "phenix.execution@1",
        json!({"operation": "runnable_tasks"}),
    );
    assert_eq!(execution["response"], "runnable_tasks");

    let language = invoke(
        &mut harness,
        "phenix.language@1",
        json!({"operation": "current_diagnostics", "workspace_id": "system"}),
    );
    assert_eq!(language["response"], "diagnostics");

    let planning = invoke(
        &mut harness,
        "phenix.planning@1",
        json!({"operation": "search_history", "objective_id": null, "query": ""}),
    );
    assert_eq!(planning["response"], "history");

    let models = invoke(
        &mut harness,
        "phenix.models.routing@1",
        json!({"operation": "list_profiles"}),
    );
    assert_eq!(models["kind"], "profiles");

    let jobs = invoke(&mut harness, "phenix.jobs@1", json!({"operation": "list"}));
    assert_eq!(jobs["response"], "resources");

    let frontends = invoke(
        &mut harness,
        "phenix.frontend-services@1",
        json!({"operation": "catalog"}),
    );
    assert_eq!(frontends["response"], "providers");

    let hooks = invoke(
        &mut harness,
        "phenix.hooks@1",
        json!({"operation": "get_configuration", "revision": "missing"}),
    );
    assert_eq!(hooks["response"], "configuration");
    assert!(hooks["configuration"].is_null());

    let workspace = invoke(
        &mut harness,
        "phenix.workspace@1",
        json!({"operation": "search", "needle": "definitely-not-a-phenix-match", "path": null, "case_sensitive": true}),
    );
    assert_eq!(workspace["response"], "search");

    let debug = invoke(
        &mut harness,
        "phenix.debug@1",
        json!({"operation": "snapshot"}),
    );
    assert_eq!(debug["response"], "snapshot");
    assert!(debug["snapshot"]["services"]
        .as_object()
        .unwrap()
        .values()
        .all(|entry| entry["available"] == true));

    let cli = ServiceId::parse("phenix.cli.discover@1").unwrap();
    let error = harness
        .invoke(
            &cli,
            &serde_json::to_vec(&json!({"name": "not-a-supported-cli"})).unwrap(),
            &default_suite_authority(),
            None,
        )
        .unwrap_err();
    assert!(error.to_string().contains("unsupported CLI probe target"));
}

#[test]
fn supported_harness_routes_model_inference_and_tool_calls_through_plugins() {
    let provider = "fixture.model-provider";
    let tool_service = ServiceId::parse("fixture.echo@1").unwrap();
    let mut builder = HarnessBuilder::with_default_suite().unwrap();
    builder
        .add_embedded(
            fixture_manifest(provider, model_inference_service()),
            || Box::new(ModelProvider),
        )
        .unwrap();
    builder
        .add_embedded(
            fixture_manifest("fixture.echo-tool", tool_service.clone()),
            || Box::new(EchoTool),
        )
        .unwrap();
    let mut harness = builder.build().unwrap();
    harness.activate().unwrap();

    let profile = RoutingProfile {
        id: "parity".into(),
        default_target: ModelTarget {
            provider_plugin: provider.into(),
            model: "fixture-model".into(),
            options: BTreeMap::new(),
        },
        callable_targets: BTreeMap::new(),
    };
    let register = serde_json::to_vec(&ModelCommand::RegisterProfile { profile }).unwrap();
    harness
        .invoke(
            &model_routing_service(),
            &register,
            &default_suite_authority(),
            None,
        )
        .unwrap();
    let authenticate = serde_json::to_vec(&ModelCommand::SetProviderAuthenticated {
        provider_plugin: provider.into(),
        authenticated: true,
    })
    .unwrap();
    harness
        .invoke(
            &model_routing_service(),
            &authenticate,
            &default_suite_authority(),
            None,
        )
        .unwrap();
    let model = serde_json::to_vec(&ModelCommand::Invoke {
        profile_id: "parity".into(),
        callable_id: None,
        input: b"hello".to_vec(),
    })
    .unwrap();
    let output = harness
        .invoke(
            &model_routing_service(),
            &model,
            &default_suite_authority(),
            None,
        )
        .unwrap();
    let output: ModelResponse = serde_json::from_slice(&output).unwrap();
    match output {
        ModelResponse::Inference { target, response } => {
            assert_eq!(target.provider_plugin, provider);
            assert_eq!(target.model, "fixture-model");
            assert_eq!(response.output, b"answer:hello");
            assert_eq!(response.provider_metadata["model"], "fixture-model");
        }
        other => panic!("unexpected model response: {other:?}"),
    }

    let create = serde_json::to_vec(&ExecutionCommand::CreateExecution {
        id: "root".into(),
        requested_authority: ExecutionAuthority::new(Vec::<String>::new()),
    })
    .unwrap();
    harness
        .invoke(
            &execution_service(),
            &create,
            &default_suite_authority(),
            None,
        )
        .unwrap();
    let register = serde_json::to_vec(&ExecutionCommand::RegisterCallable {
        id: "echo".into(),
        service: tool_service.to_string(),
        required_authority: ExecutionAuthority::new(Vec::<String>::new()),
    })
    .unwrap();
    harness
        .invoke(
            &execution_service(),
            &register,
            &default_suite_authority(),
            None,
        )
        .unwrap();
    let invoke_tool = serde_json::to_vec(&ExecutionCommand::InvokeCallable {
        execution_id: "root".into(),
        callable_id: "echo".into(),
        input: br#"{"value":"hello"}"#.to_vec(),
    })
    .unwrap();
    let output = harness
        .invoke(
            &execution_service(),
            &invoke_tool,
            &default_suite_authority(),
            None,
        )
        .unwrap();
    let output: ExecutionResponse = serde_json::from_slice(&output).unwrap();
    match output {
        ExecutionResponse::Invocation { output } => {
            assert_eq!(output, br#"{"value":"hello"}"#);
        }
        other => panic!("unexpected execution response: {other:?}"),
    }
}
