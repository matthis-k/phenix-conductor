use phenix_sdk::{
    phenix_plugin, Authority, ComponentInterface, EventName, HookName, ListenerProjection,
    PhenixValue,
};

#[derive(Clone, Debug, Eq, PartialEq, PhenixValue)]
struct ProviderEvent {
    value: String,
    extra: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, PhenixValue)]
struct ProjectedEvent {
    value: String,
}

#[derive(Clone, Debug, Eq, PartialEq, PhenixValue)]
struct ExactEvent {
    value: String,
}

fn on_projected(event: ProjectedEvent) -> Result<(), String> {
    if event.value == "ok" {
        Ok(())
    } else {
        Err(format!("unexpected projected value: {}", event.value))
    }
}

fn on_exact(event: ExactEvent) -> Result<(), String> {
    if event.value == "ok" {
        Ok(())
    } else {
        Err(format!("unexpected exact value: {}", event.value))
    }
}

phenix_plugin! {
    "fixture.authoring";

    uses {
        models: "fixture.models@1",
    }

    provides {
        planning: "fixture.planning@1",
    }

    emits {
        completed: "fixture.planning.completed",
    }

    listens {
        projected: "fixture.session.created" => ProjectedEvent => on_projected,
    }

    exact_listens {
        exact: "fixture.session.exact" => ExactEvent => on_exact,
    }

    hooks {
        provides {
            before_plan: "fixture.planning.before@1",
        }
        uses {
            model_request: "fixture.model.request@1",
        }
    }
}

mod minimal {
    use phenix_sdk::phenix_plugin;

    phenix_plugin! {
        "fixture.minimal";
    }
}

#[allow(dead_code)]
fn generated_sdk_exposes_dependencies_directly<'host, 'runtime>(
    sdk: &phenix_plugin::Sdk<'host, 'runtime>,
) {
    let _ = &sdk.models;
    let _ = &sdk.events.completed;
    let _ = &sdk.hooks.model_request;
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
fn unused_sections_can_be_omitted() {
    let plugin = minimal::phenix_plugin::plugin_manifest(Authority::default());
    let component = minimal::phenix_plugin::component_manifest(Authority::default());

    assert_eq!(plugin.id.as_str(), "fixture.minimal");
    assert!(plugin.services.is_empty());
    assert!(component.imports.is_empty());
    assert!(component.exports.is_empty());
    assert!(minimal::phenix_plugin::listeners().is_empty());
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

#[test]
fn projected_listener_accepts_provider_only_fields() {
    let value = PhenixValue::from(&ProviderEvent {
        value: "ok".into(),
        extra: 7,
    });
    let event = EventName::parse("fixture.session.created").unwrap();

    assert!(phenix_plugin::dispatch_listener(&event, &value).unwrap());
}

#[test]
fn exact_listener_rejects_provider_only_fields_without_panicking() {
    let value = PhenixValue::from(&ProviderEvent {
        value: "ok".into(),
        extra: 7,
    });
    let event = EventName::parse("fixture.session.exact").unwrap();

    let error = phenix_plugin::dispatch_listener(&event, &value).unwrap_err();
    assert!(error.contains("unexpected key"));
}
