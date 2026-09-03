use phenix_sdk::{
    Authority, Call, CapabilityId, Emit, Required, StaticComponentBehavior, StaticPluginComponents,
    StaticPluginConfiguration, StaticPluginLifecycle, StaticPluginResources,
};

mod sessions {
    #[phenix_sdk::plugin("fixture.attribute-gate.sessions")]
    pub struct Plugin;
}

#[derive(Clone, Debug, Eq, PartialEq, phenix_sdk::PhenixValue)]
struct Settings {
    retries: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, phenix_sdk::PhenixValue)]
struct Request {
    prompt: String,
}

#[derive(Clone, Debug, Eq, PartialEq, phenix_sdk::PhenixValue)]
struct Response {
    value: String,
}

#[phenix_sdk::interface("fixture.attribute-gate.models@1")]
struct Models;

#[phenix_sdk::interface("fixture.attribute-gate.internal@1")]
struct Internal;

fn authority(capability: &str) -> Authority {
    Authority::new([CapabilityId::parse(capability).unwrap()])
}

#[allow(dead_code)]
#[phenix_sdk::component]
struct Api {
    #[phenix(import, authority = authority("models.invoke"))]
    models: Required<Call<Models, Request, Response>>,

    #[phenix(event("fixture.attribute-gate.completed"))]
    completed: Emit<Response>,
}

#[allow(dead_code)]
#[phenix_sdk::component]
impl Api {
    #[phenix(
        export(Models),
        public,
        terminal,
        authority = authority("models.serve")
    )]
    fn run(&mut self, request: Request) -> Response {
        Response {
            value: request.prompt,
        }
    }

    #[phenix(export(Internal))]
    fn internal(&mut self, request: Request) -> Response {
        Response {
            value: request.prompt,
        }
    }

    #[phenix(
        layer(Models, priority = 17, authority = authority("models.layer"))
    )]
    fn policy(&mut self) {}

    #[phenix(
        listen("fixture.attribute-gate.observed"),
        authority = authority("events.observe")
    )]
    fn observed(&mut self, _context: &phenix_sdk::EventContext, _response: Response) {}

    #[phenix(value("fixture.attribute-gate.status@1"), public)]
    fn status(&self) -> u64 {
        1
    }

    #[phenix(value("fixture.attribute-gate.internal-status@1"))]
    fn internal_status(&self) -> u64 {
        0
    }
}

struct Store;

#[phenix_sdk::resource(schema = 1)]
impl Store {}

#[allow(dead_code)]
#[phenix_sdk::plugin(
    id = "fixture.attribute-gate",
    version = 7,
    authority = authority("plugin.run")
)]
struct Plugin {
    #[phenix(dep)]
    sessions: sessions::Plugin,

    #[phenix(component)]
    api: Api,

    #[phenix(resource)]
    state: phenix_sdk::Durable<Store>,

    #[phenix(config)]
    config: Settings,
}

#[allow(dead_code)]
#[phenix_sdk::plugin]
impl Plugin {
    #[phenix(start)]
    fn start(&mut self, _context: &phenix_sdk::PluginContext<'_, '_, ()>) -> Result<(), String> {
        Ok(())
    }

    #[phenix(stop)]
    fn stop(&mut self, _context: &phenix_sdk::PluginContext<'_, '_, ()>) -> Result<(), String> {
        Ok(())
    }
}

#[test]
fn attribute_only_plugin_builds_graph_and_manifest_without_parallel_wiring() {
    let graph = phenix_sdk::StaticPluginGraph::compose::<Plugin>().unwrap();
    let ids = graph
        .ids()
        .map(phenix_sdk::PluginId::as_str)
        .collect::<Vec<_>>();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&"fixture.attribute-gate.sessions"));
    assert!(ids.contains(&"fixture.attribute-gate"));

    let descriptor = graph
        .descriptor(&Plugin::plugin_id())
        .expect("root plugin descriptor exists");
    assert_eq!(descriptor.maximum_authority, authority("plugin.run"));

    let components = <Plugin as StaticPluginComponents>::components();
    assert_eq!(components.len(), 1);
    assert_eq!(components[0].id.as_str(), "fixture.attribute-gate.api");
    assert_eq!(components[0].imports().len(), 1);
    assert_eq!(
        components[0].imports()[0].authority,
        authority("models.invoke")
    );
    let component_manifest =
        components[0].manifest_with_authority(&Plugin::plugin_id(), &descriptor.maximum_authority);
    assert_eq!(
        component_manifest.maximum_authority,
        authority("plugin.run")
    );
    assert_eq!(
        component_manifest.imports[0].authority,
        authority("models.invoke")
    );
    let services = components[0].services();
    assert_eq!(services.len(), 2);
    assert_eq!(services[0].required_authority, authority("models.serve"));
    assert_eq!(services[1].required_authority, authority("models.layer"));

    let behavior = <Api as StaticComponentBehavior>::exports();
    assert_eq!(behavior.len(), 2);
    assert!(behavior[0].public);
    assert!(behavior[0].terminal);
    assert_eq!(behavior[0].required_authority, authority("models.serve"));
    assert!(!behavior[1].public);

    let layers = <Api as StaticComponentBehavior>::layers();
    assert_eq!(layers.len(), 1);
    assert_eq!(layers[0].required_authority, authority("models.layer"));

    let listeners = <Api as StaticComponentBehavior>::listeners();
    assert_eq!(listeners.len(), 1);
    assert_eq!(listeners[0].required_authority, authority("events.observe"));

    let public_callables = behavior
        .iter()
        .filter(|export| export.public)
        .map(|export| export.interface.as_str())
        .collect::<Vec<_>>();
    assert_eq!(public_callables, ["fixture.attribute-gate.models@1"]);

    let public_values = <Api as StaticComponentBehavior>::values()
        .into_iter()
        .filter(|value| value.public)
        .map(|value| value.id.to_string())
        .collect::<Vec<_>>();
    assert_eq!(public_values, ["fixture.attribute-gate.status@1"]);

    let resources = <Plugin as StaticPluginResources>::resources();
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].id.as_str(), "fixture.attribute-gate.state");
    assert_eq!(resources[0].schema.version, 1);

    let config = <Plugin as StaticPluginConfiguration>::configuration()
        .expect("config field generates configuration metadata");
    assert_eq!(config.field, "config");

    let lifecycle = Plugin::lifecycle();
    assert_eq!(lifecycle.start, Some("start"));
    assert_eq!(lifecycle.stop, Some("stop"));

    let manifest = <Plugin as phenix_sdk::StaticPluginDefinition>::manifest();
    assert_eq!(manifest.id.as_str(), "fixture.attribute-gate");
    assert_eq!(manifest.version, 7);
    assert_eq!(manifest.services.len(), 2);
    assert_eq!(manifest.maximum_authority, authority("plugin.run"));
    assert_eq!(
        manifest.resource_namespaces[0].as_str(),
        "fixture.attribute-gate.state"
    );
}
