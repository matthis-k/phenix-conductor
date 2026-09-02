use phenix_core::{
    Authority, ComponentInterface, EventBus, EventFailurePolicy, EventSubscription, EventTypeId,
    InterfaceCompatibility, Kernel, KernelConfig, PhenixValue, PluginHost, PluginId,
    PluginInstance, Project, ResolvedHarness, ResolvedHarnessActivation, ServiceId, SubscriptionId,
    SubscriptionSpec,
};
use phenix_sdk::{
    phenix_plugin, EventName, HookName, ListenerProjection, PhenixValue as DerivePhenixValue,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[derive(Clone, Debug, Eq, PartialEq, DerivePhenixValue)]
struct AuthoringModelRequest {
    prompt: String,
}

#[derive(Clone, Debug, Eq, PartialEq, DerivePhenixValue)]
struct AuthoringModelNeeds {
    value: String,
}

#[derive(Clone, Debug, Eq, PartialEq, DerivePhenixValue)]
struct PlanningRequest {
    goal: String,
}

#[derive(Clone, Debug, Eq, PartialEq, DerivePhenixValue)]
struct PlanningResponse {
    plan_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, DerivePhenixValue)]
struct HookInput {
    value: String,
}

#[derive(Clone, Debug, Eq, PartialEq, DerivePhenixValue)]
struct HookOutput {
    value: String,
}

#[derive(Clone, Debug, Eq, PartialEq, DerivePhenixValue)]
struct ProviderEvent {
    value: String,
    extra: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, DerivePhenixValue)]
struct ProjectedEvent {
    value: String,
}

#[derive(Clone, Debug, Eq, PartialEq, DerivePhenixValue)]
struct ExactEvent {
    value: String,
}

static PROJECTED_CALLS: AtomicUsize = AtomicUsize::new(0);
static EXACT_CALLS: AtomicUsize = AtomicUsize::new(0);
static DIAGNOSTIC_CALLS: AtomicUsize = AtomicUsize::new(0);
static EVENT_WARNINGS: AtomicUsize = AtomicUsize::new(0);

fn on_projected(event: ProjectedEvent) -> Result<(), String> {
    PROJECTED_CALLS.fetch_add(1, Ordering::SeqCst);
    if event.value == "ok" {
        Ok(())
    } else {
        Err(format!("unexpected projected value: {}", event.value))
    }
}

fn on_exact(event: ExactEvent) -> Result<(), String> {
    EXACT_CALLS.fetch_add(1, Ordering::SeqCst);
    if event.value == "ok" {
        Ok(())
    } else {
        Err(format!("unexpected exact value: {}", event.value))
    }
}

phenix_plugin! {
    "fixture.authoring";

    uses {
        models: "fixture.models@1" => AuthoringModelRequest => AuthoringModelNeeds,
    }

    provides {
        planning: "fixture.planning@1" => PlanningRequest => PlanningResponse,
    }

    emits {
        completed: "fixture.planning.completed",
    }

    listens {
        projected: "fixture.session.created" => ProjectedEvent => on_projected,
    }

    exact_listens {
        exact: "fixture.session.created" => ExactEvent => on_exact,
    }

    hooks {
        provides {
            before_plan: "fixture.planning.before@1" => HookInput => HookOutput,
        }
        uses {
            model_request: "fixture.model.request@1" => HookInput => HookOutput,
        }
    }
}

mod minimal {
    use phenix_sdk::phenix_plugin;

    phenix_plugin! {
        "fixture.minimal";
    }
}

mod model_provider {
    use super::*;

    #[derive(Clone, Debug, Eq, PartialEq, DerivePhenixValue)]
    struct ModelRequest {
        prompt: String,
    }

    #[derive(Clone, Debug, Eq, PartialEq, DerivePhenixValue)]
    struct ModelResponse {
        value: String,
        tokens: u64,
    }

    phenix_plugin! {
        "fixture.model-provider";

        provides {
            models: "fixture.models@1" => ModelRequest => ModelResponse,
        }
    }

    pub struct Plugin;

    impl PluginInstance for Plugin {
        fn start(&mut self, _host: &PluginHost<'_>) -> Result<(), String> {
            Ok(())
        }

        fn invoke(
            &mut self,
            service: &ServiceId,
            input: &[u8],
            host: &PluginHost<'_>,
        ) -> Result<Vec<u8>, String> {
            if service != &models_service() {
                return Err(format!("unsupported model service: {service}"));
            }
            phenix_plugin::provides::models::dispatch(
                host,
                input,
                |request: ModelRequest| -> Result<ModelResponse, String> {
                    Ok(ModelResponse {
                        value: format!("{}!", request.prompt),
                        tokens: 7,
                    })
                },
            )
        }
    }

    pub fn models_service() -> ServiceId {
        ServiceId::parse("fixture.models@1").unwrap()
    }
}

mod model_consumer {
    use super::*;

    #[derive(Clone, Debug, Eq, PartialEq, DerivePhenixValue)]
    struct ModelRequest {
        prompt: String,
        trace_id: String,
    }

    #[derive(Clone, Debug, Eq, PartialEq, DerivePhenixValue)]
    struct ModelResponse {
        value: String,
    }

    #[derive(Clone, Debug, Eq, PartialEq, DerivePhenixValue)]
    pub struct RunRequest {
        pub prompt: String,
        pub exact: bool,
    }

    #[derive(Clone, Debug, Eq, PartialEq, DerivePhenixValue)]
    pub struct RunResponse {
        pub value: String,
    }

    phenix_plugin! {
        "fixture.model-consumer";

        uses {
            models: "fixture.models@1" => ModelRequest => ModelResponse,
        }

        provides {
            run: "fixture.model-consumer.run@1" => RunRequest => RunResponse,
        }
    }

    pub struct Plugin;

    impl PluginInstance for Plugin {
        fn start(&mut self, _host: &PluginHost<'_>) -> Result<(), String> {
            Ok(())
        }

        fn invoke(
            &mut self,
            service: &ServiceId,
            input: &[u8],
            host: &PluginHost<'_>,
        ) -> Result<Vec<u8>, String> {
            if service != &run_service() {
                return Err(format!("unsupported consumer service: {service}"));
            }
            phenix_plugin::provides::run::dispatch(
                host,
                input,
                |request: RunRequest| -> Result<RunResponse, String> {
                    let context = phenix_plugin::context(host, (), ());
                    let model_request = ModelRequest {
                        prompt: request.prompt,
                        trace_id: "consumer-only".into(),
                    };
                    let response = if request.exact {
                        context.sdk.models.invoke_exact(&model_request)
                    } else {
                        context.sdk.models.invoke(&model_request)
                    }
                    .map_err(|error| error.to_string())?;
                    Ok(RunResponse {
                        value: response.value,
                    })
                },
            )
        }
    }

    pub fn run_service() -> ServiceId {
        ServiceId::parse("fixture.model-consumer.run@1").unwrap()
    }
}

mod event_emitter {
    use super::*;

    phenix_plugin! {
        "fixture.event-emitter";

        provides {
            trigger: "fixture.event.trigger@1" => PlanningRequest => PlanningResponse,
        }

        emits {
            created: "fixture.session.created",
        }
    }

    pub struct Plugin;

    impl PluginInstance for Plugin {
        fn start(&mut self, _host: &PluginHost<'_>) -> Result<(), String> {
            Ok(())
        }

        fn invoke(
            &mut self,
            service: &ServiceId,
            input: &[u8],
            host: &PluginHost<'_>,
        ) -> Result<Vec<u8>, String> {
            if service != &trigger_service() {
                return Err(format!("unsupported emitter service: {service}"));
            }
            phenix_plugin::provides::trigger::dispatch(
                host,
                input,
                |_request: PlanningRequest| -> Result<PlanningResponse, String> {
                    let context = phenix_plugin::context(host, (), ());
                    let report = context
                        .sdk
                        .events
                        .created
                        .emit(&ProviderEvent {
                            value: "ok".into(),
                            extra: 7,
                        })
                        .map_err(|error| error.to_string())?;
                    EVENT_WARNINGS.store(report.warnings.len(), Ordering::SeqCst);
                    Ok(PlanningResponse {
                        plan_id: "event".into(),
                    })
                },
            )
        }
    }

    pub fn trigger_service() -> ServiceId {
        ServiceId::parse("fixture.event.trigger@1").unwrap()
    }
}

mod hook_provider {
    use super::*;

    #[derive(Clone, Debug, Eq, PartialEq, DerivePhenixValue)]
    struct Request {
        value: String,
    }

    #[derive(Clone, Debug, Eq, PartialEq, DerivePhenixValue)]
    struct Response {
        value: String,
    }

    phenix_plugin! {
        "fixture.hook-provider";

        hooks {
            provides {
                before_plan: "fixture.planning.before@1" => Request => Response,
            }
        }
    }

    pub struct Plugin;

    impl PluginInstance for Plugin {
        fn start(&mut self, _host: &PluginHost<'_>) -> Result<(), String> {
            Ok(())
        }

        fn invoke(
            &mut self,
            service: &ServiceId,
            input: &[u8],
            host: &PluginHost<'_>,
        ) -> Result<Vec<u8>, String> {
            if service != &hook_service() {
                return Err(format!("unsupported hook service: {service}"));
            }
            phenix_plugin::hook_providers::before_plan::dispatch(
                host,
                input,
                |request: Request| -> Result<Response, String> {
                    if request.value == "reject" {
                        return Err("hook rejected request".into());
                    }
                    Ok(Response {
                        value: format!("{}!", request.value),
                    })
                },
            )
        }
    }

    pub fn hook_service() -> ServiceId {
        ServiceId::parse("fixture.planning.before@1").unwrap()
    }
}

#[allow(dead_code)]
fn generated_sdk_exposes_typed_dependencies<'host, 'runtime>(
    sdk: &phenix_plugin::Sdk<'host, 'runtime>,
) {
    let _: &phenix_plugin::dependencies::models::Client<'host, 'runtime> = &sdk.models;
    let _ = &sdk.events.completed;
    let _: &phenix_plugin::hook_consumers::model_request::Client<'host, 'runtime> =
        &sdk.hooks.model_request;
}

#[test]
fn macro_generates_composable_manifests() {
    let plugin = phenix_plugin::plugin_manifest(Authority::default());
    let component = phenix_plugin::component_manifest(Authority::default());

    assert_eq!(plugin.id.as_str(), "fixture.authoring");
    assert_eq!(plugin.services.len(), 2);
    assert_eq!(component.imports.len(), 2);
    assert_eq!(component.exports.len(), 2);
}

#[test]
fn generated_interfaces_preserve_independent_structural_schemas() {
    let consumer =
        <model_consumer::phenix_plugin::dependencies::models::Interface as ComponentInterface>::schema();
    let provider =
        <model_provider::phenix_plugin::provides::models::Interface as ComponentInterface>::schema(
        );

    assert_eq!(
        consumer.accepts_provider(&provider),
        InterfaceCompatibility::Compatible
    );

    let component = model_consumer::phenix_plugin::component_manifest(Authority::default());
    assert_eq!(component.imports[0].schema, consumer);
}

#[test]
fn independent_provider_and_consumer_types_work_through_the_live_component_graph() {
    let authority = Authority::default();
    let provider = model_provider::phenix_plugin::plugin_manifest(authority.clone());
    let consumer = model_consumer::phenix_plugin::plugin_manifest(authority.clone());
    let manifests = [provider.clone(), consumer.clone()];
    let resolved = ResolvedHarness::resolve(
        manifests.clone(),
        [
            model_provider::phenix_plugin::component_manifest(authority.clone()),
            model_consumer::phenix_plugin::component_manifest(authority.clone()),
        ],
        [],
        &authority,
    )
    .unwrap();

    let mut kernel = Kernel::new(KernelConfig::new(manifests).unwrap());
    kernel
        .register_embedded_factory(provider.id, || Box::new(model_provider::Plugin))
        .unwrap();
    kernel
        .register_embedded_factory(consumer.id, || Box::new(model_consumer::Plugin))
        .unwrap();
    kernel.activate_resolved_harness(&resolved).unwrap();
    kernel.activate_all().unwrap();

    let projected = serde_json::to_vec(&PhenixValue::from(&model_consumer::RunRequest {
        prompt: "hello".into(),
        exact: false,
    }))
    .unwrap();
    let output = kernel
        .invoke(&model_consumer::run_service(), &projected, &authority, None)
        .unwrap();
    let value: PhenixValue = serde_json::from_slice(&output).unwrap();
    assert_eq!(
        model_consumer::RunResponse::try_from(Project(&value)).unwrap(),
        model_consumer::RunResponse {
            value: "hello!".into(),
        }
    );

    let exact = serde_json::to_vec(&PhenixValue::from(&model_consumer::RunRequest {
        prompt: "hello".into(),
        exact: true,
    }))
    .unwrap();
    assert!(kernel
        .invoke(&model_consumer::run_service(), &exact, &authority, None,)
        .is_err());
}

#[test]
fn unused_sections_can_be_omitted() {
    let plugin = minimal::phenix_plugin::plugin_manifest(Authority::default());
    let component = minimal::phenix_plugin::component_manifest(Authority::default());
    let events = Arc::new(EventBus::default());

    assert_eq!(plugin.id.as_str(), "fixture.minimal");
    assert!(plugin.services.is_empty());
    assert!(component.imports.is_empty());
    assert!(component.exports.is_empty());
    assert!(minimal::phenix_plugin::listeners().is_empty());
    assert!(minimal::phenix_plugin::event_subscriptions(&events, Authority::default()).is_empty());
}

#[test]
fn hook_names_and_generated_interfaces_use_runtime_ids_only() {
    let hook = HookName::parse("fixture.model.request@1").unwrap();

    assert_eq!(hook.as_str(), "fixture.model.request@1");
    assert!(HookName::parse("").is_err());
    assert_eq!(
        phenix_plugin::hook_consumers::model_request::Interface::interface_id().as_str(),
        hook.as_str()
    );
    assert_eq!(
        phenix_plugin::hook_providers::before_plan::Interface::interface_id().as_str(),
        "fixture.planning.before@1"
    );
}

#[test]
fn listener_declarations_preserve_projection_mode() {
    let listeners = phenix_plugin::listeners();

    assert_eq!(listeners.len(), 2);
    assert_eq!(listeners[0].local_name, "projected");
    assert_eq!(listeners[0].projection, ListenerProjection::Project);
    assert_eq!(listeners[1].local_name, "exact");
    assert_eq!(listeners[1].projection, ListenerProjection::Exact);
}

fn on_diagnostic(
    _event: &phenix_core::EventEnvelope,
    _authority: &Authority,
) -> Result<(), String> {
    DIAGNOSTIC_CALLS.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

fn diagnostic_subscription() -> EventSubscription {
    EventSubscription {
        spec: SubscriptionSpec {
            id: SubscriptionId::parse("fixture.diagnostic-listener").unwrap(),
            owner: PluginId::parse("fixture.diagnostics").unwrap(),
            event_type: EventTypeId::parse("kernel.structural_value_mismatch").unwrap(),
            event_version: 1,
            dependencies: Vec::new(),
            failure_policy: EventFailurePolicy::Ignore,
            required_authority: Authority::default(),
            maximum_authority: Authority::default(),
            kernel_policy_revision: 0,
        },
        handler: Arc::new(on_diagnostic),
    }
}

#[test]
fn typed_event_emission_isolates_listener_mismatch_and_emits_diagnostic() {
    PROJECTED_CALLS.store(0, Ordering::SeqCst);
    EXACT_CALLS.store(0, Ordering::SeqCst);
    DIAGNOSTIC_CALLS.store(0, Ordering::SeqCst);
    EVENT_WARNINGS.store(0, Ordering::SeqCst);

    let manifest = event_emitter::phenix_plugin::plugin_manifest(Authority::default());
    let mut kernel = Kernel::new(KernelConfig::new([manifest.clone()]).unwrap());
    kernel
        .register_embedded_factory(manifest.id, || Box::new(event_emitter::Plugin))
        .unwrap();

    let events = kernel.events();
    let mut subscriptions = phenix_plugin::event_subscriptions(&events, Authority::default());
    subscriptions.push(diagnostic_subscription());
    events.replace_subscriptions(subscriptions).unwrap();

    kernel.activate_all().unwrap();
    let input = serde_json::to_vec(&PhenixValue::from(&PlanningRequest {
        goal: "emit".into(),
    }))
    .unwrap();
    kernel
        .invoke(
            &event_emitter::trigger_service(),
            &input,
            &Authority::default(),
            None,
        )
        .unwrap();

    assert_eq!(PROJECTED_CALLS.load(Ordering::SeqCst), 1);
    assert_eq!(EXACT_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(DIAGNOSTIC_CALLS.load(Ordering::SeqCst), 1);
    assert_eq!(EVENT_WARNINGS.load(Ordering::SeqCst), 1);
}

#[test]
fn generated_hook_provider_can_transform_or_reject() {
    let manifest = hook_provider::phenix_plugin::plugin_manifest(Authority::default());
    let mut kernel = Kernel::new(KernelConfig::new([manifest.clone()]).unwrap());
    kernel
        .register_embedded_factory(manifest.id, || Box::new(hook_provider::Plugin))
        .unwrap();
    kernel.activate_all().unwrap();

    let input = serde_json::to_vec(&PhenixValue::from(&HookInput { value: "ok".into() })).unwrap();
    let output = kernel
        .invoke(
            &hook_provider::hook_service(),
            &input,
            &Authority::default(),
            None,
        )
        .unwrap();
    let value: PhenixValue = serde_json::from_slice(&output).unwrap();
    assert_eq!(
        HookOutput::try_from(Project(&value)).unwrap(),
        HookOutput {
            value: "ok!".into(),
        }
    );

    let rejected = serde_json::to_vec(&PhenixValue::from(&HookInput {
        value: "reject".into(),
    }))
    .unwrap();
    let error = kernel
        .invoke(
            &hook_provider::hook_service(),
            &rejected,
            &Authority::default(),
            None,
        )
        .unwrap_err();
    assert!(error.to_string().contains("hook rejected request"));
}

#[test]
fn event_and_hook_names_validate_runtime_identifiers() {
    let event = EventName::parse("fixture.session.created").unwrap();
    assert_eq!(event.as_str(), "fixture.session.created");
    assert!(EventName::parse("").is_err());
}

#[allow(dead_code)]
mod attribute_composition {
    #[phenix_sdk::plugin(id = "fixture.attr.leaf")]
    pub struct Leaf;

    #[phenix_sdk::plugin(id = "fixture.attr.left")]
    pub struct Left {
        #[phenix(dep)]
        pub leaf: Leaf,
    }

    #[phenix_sdk::plugin(id = "fixture.attr.right")]
    pub struct Right {
        #[phenix(dep)]
        pub leaf: Leaf,
    }

    #[phenix_sdk::plugin(id = "fixture.attr.root")]
    pub struct Root {
        #[phenix(dep)]
        pub left: Left,
        #[phenix(dep)]
        pub right: Right,
    }

    #[phenix_sdk::plugin(id = "fixture.attr.conflict")]
    pub struct ConflictA;

    #[phenix_sdk::plugin(id = "fixture.attr.conflict")]
    pub struct ConflictB;

    #[phenix_sdk::plugin(id = "fixture.attr.conflict-root")]
    pub struct ConflictRoot {
        #[phenix(dep)]
        pub first: ConflictA,
        #[phenix(dep)]
        pub second: ConflictB,
    }

    #[phenix_sdk::interface("fixture.attr.models@1")]
    pub struct Models;

    #[phenix_sdk::plugin("fixture.attr.stateless")]
    pub mod stateless {
        #[phenix(export("fixture.attr.stateless.run@1"), public)]
        pub fn run() {}
    }

    pub struct PlanStore;

    #[phenix_sdk::resource(schema = 3)]
    impl PlanStore {
        #[phenix(migrate(from = 2))]
        fn v2_to_v3() {}
    }

    #[phenix_sdk::plugin("fixture.attr.resource-owner")]
    pub struct ResourceOwner {
        #[phenix(
            resource,
            id = "fixture.attr.plans",
            features(Transactions, Migrations)
        )]
        pub plans: phenix_sdk::Durable<PlanStore>,
    }
}

#[test]
fn attribute_plugin_dependencies_expand_recursively_and_deduplicate_diamonds() {
    let graph = phenix_sdk::StaticPluginGraph::compose::<attribute_composition::Root>().unwrap();
    let ids = graph
        .ids()
        .map(phenix_core::PluginId::as_str)
        .collect::<Vec<_>>();

    assert_eq!(
        ids,
        vec![
            "fixture.attr.leaf",
            "fixture.attr.left",
            "fixture.attr.right",
            "fixture.attr.root",
        ]
    );
}

#[test]
fn attribute_plugin_dependencies_reject_incompatible_duplicate_ids() {
    let error = phenix_sdk::StaticPluginGraph::compose::<attribute_composition::ConflictRoot>()
        .unwrap_err();
    assert!(matches!(
        error,
        phenix_sdk::StaticPluginGraphError::DuplicateId { .. }
    ));
}

#[test]
fn interface_attribute_owns_canonical_runtime_identity() {
    let id = <attribute_composition::Models as phenix_sdk::InterfaceMarker>::interface_id();

    assert_eq!(id.as_str(), "fixture.attr.models@1");
}

#[test]
fn stateless_plugin_module_generates_default_component_and_export() {
    let plugin_id = attribute_composition::stateless::Plugin::plugin_id();
    let graph =
        phenix_sdk::StaticPluginGraph::compose::<attribute_composition::stateless::Plugin>()
            .unwrap();
    let components = <attribute_composition::stateless::Plugin as phenix_sdk::StaticPluginComponents>::components();
    let exports = <attribute_composition::stateless::Component as phenix_sdk::StaticComponentBehavior>::exports();

    assert_eq!(plugin_id.as_str(), "fixture.attr.stateless");
    assert_eq!(
        graph.ids().next().unwrap().as_str(),
        "fixture.attr.stateless"
    );
    assert_eq!(components.len(), 1);
    assert_eq!(components[0].id.as_str(), "fixture.attr.stateless");
    assert_eq!(exports.len(), 1);
    assert_eq!(
        exports[0].interface.as_str(),
        "fixture.attr.stateless.run@1"
    );
    assert!(exports[0].public);
}

#[test]
fn resource_attribute_owns_schema_and_migration_metadata() {
    let migrations =
        <attribute_composition::PlanStore as phenix_sdk::StaticResourceDefinition>::migrations();

    assert_eq!(
        <attribute_composition::PlanStore as phenix_sdk::StaticResourceDefinition>::schema_version(
        ),
        3
    );
    assert_eq!(migrations.len(), 1);
    assert_eq!(migrations[0].from_version, 2);
    assert_eq!(migrations[0].to_version, 3);
    assert_eq!(migrations[0].method, "v2_to_v3");
}

#[test]
fn plugin_resource_field_preserves_identity_schema_and_backend_features() {
    let resources =
        <attribute_composition::ResourceOwner as phenix_sdk::StaticPluginResources>::resources();

    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].id.as_str(), "fixture.attr.plans");
    assert_eq!(resources[0].schema.version, 3);
    assert_eq!(resources[0].field, "plans");
    assert!(resources[0]
        .schema
        .required_features
        .contains(&phenix_sdk::BackendFeature::Transactions));
    assert!(resources[0]
        .schema
        .required_features
        .contains(&phenix_sdk::BackendFeature::Migrations));
}
