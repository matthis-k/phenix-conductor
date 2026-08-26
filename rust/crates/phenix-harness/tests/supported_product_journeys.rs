use phenix_harness::{default_suite_authority, PhenixHarness};
use phenix_kernel::ServiceId;
use serde_json::{json, Value};

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
