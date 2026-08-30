use crate::{model_routing_manifest, MODEL_ROUTING_SERVICE};
use phenix_core::{
    Authority, CapabilityId, ComponentExport, ComponentId, ComponentInterface, ComponentManifest,
    InterfaceId, PluginId,
};

const MODEL_ROUTING_COMPONENT: &str = "phenix.models";
const MODEL_ROUTING_PLUGIN: &str = "phenix.models";
const PERSISTENCE_SCHEMA: &str = "kernel.persistence.schema";
const PERSISTENCE_READ: &str = "kernel.persistence.read";
const PERSISTENCE_WRITE: &str = "kernel.persistence.write";

pub struct ModelRoutingInterface;

impl ComponentInterface for ModelRoutingInterface {
    fn interface_id() -> InterfaceId {
        InterfaceId::parse(MODEL_ROUTING_SERVICE)
            .expect("static model routing interface id is valid")
    }

    fn schema() -> phenix_core::InterfaceSchema {
        phenix_core::InterfaceSchema::of::<crate::ModelCommand, crate::ModelResponse>()
    }
}

#[must_use]
pub fn model_routing_component_id() -> ComponentId {
    ComponentId::parse(MODEL_ROUTING_COMPONENT).expect("static model routing component id is valid")
}

#[must_use]
pub fn model_routing_component_manifest(maximum_authority: Authority) -> ComponentManifest {
    let authority = model_routing_manifest(maximum_authority).maximum_authority;
    ComponentManifest {
        id: model_routing_component_id(),
        owner: PluginId::parse(MODEL_ROUTING_PLUGIN)
            .expect("static model routing plugin id is valid"),
        imports: Vec::new(),
        exports: vec![ComponentExport {
            interface: ModelRoutingInterface::interface_id(),
            schema: ModelRoutingInterface::schema(),
            priority: 100,
            required_authority: persistence_authority(),
        }],
        maximum_authority: authority,
    }
}

fn persistence_authority() -> Authority {
    Authority::new([
        CapabilityId::parse(PERSISTENCE_SCHEMA).expect("static capability is valid"),
        CapabilityId::parse(PERSISTENCE_READ).expect("static capability is valid"),
        CapabilityId::parse(PERSISTENCE_WRITE).expect("static capability is valid"),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_core::{
        ComponentImport, PluginExecution, PluginManifest, ResolvedComponentGraph,
        ServiceContribution, ServiceId, ServiceRole,
    };

    fn plugin(value: &str) -> PluginId {
        PluginId::parse(value).unwrap()
    }

    fn component(value: &str) -> ComponentId {
        ComponentId::parse(value).unwrap()
    }

    fn consumer_manifest(authority: Authority) -> PluginManifest {
        PluginManifest {
            id: plugin("fixture.model-consumer"),
            version: 1,
            execution: PluginExecution::Embedded,
            dependencies: Vec::new(),
            services: Vec::new(),
            resource_namespaces: Vec::new(),
            maximum_authority: authority,
        }
    }

    fn consumer_component(authority: Authority) -> ComponentManifest {
        ComponentManifest {
            id: component("fixture.model-consumer"),
            owner: plugin("fixture.model-consumer"),
            imports: vec![ComponentImport {
                interface: ModelRoutingInterface::interface_id(),
                schema: ModelRoutingInterface::schema(),
                required: true,
                authority: authority.clone(),
            }],
            exports: Vec::new(),
            maximum_authority: authority,
        }
    }

    fn replacement_manifest() -> PluginManifest {
        PluginManifest {
            id: plugin("fixture.model-replacement"),
            version: 1,
            execution: PluginExecution::Embedded,
            dependencies: Vec::new(),
            services: vec![ServiceContribution {
                role: ServiceRole::Terminal,
                service: ServiceId::parse(MODEL_ROUTING_SERVICE).unwrap(),
                priority: 200,
                required_authority: Authority::default(),
            }],
            resource_namespaces: Vec::new(),
            maximum_authority: Authority::default(),
        }
    }

    fn replacement_component() -> ComponentManifest {
        ComponentManifest {
            id: component("fixture.model-replacement"),
            owner: plugin("fixture.model-replacement"),
            imports: Vec::new(),
            exports: vec![ComponentExport {
                interface: ModelRoutingInterface::interface_id(),
                schema: ModelRoutingInterface::schema(),
                priority: 200,
                required_authority: Authority::default(),
            }],
            maximum_authority: Authority::default(),
        }
    }

    #[test]
    fn first_party_model_is_an_ordinary_typed_component() {
        let manifest = model_routing_component_manifest(Authority::default());
        assert_eq!(manifest.id, model_routing_component_id());
        assert_eq!(manifest.owner.as_str(), MODEL_ROUTING_PLUGIN);
        assert_eq!(manifest.exports.len(), 1);
        assert_eq!(
            manifest.exports[0].interface,
            ModelRoutingInterface::interface_id()
        );
        assert_eq!(
            manifest.exports[0].required_authority,
            persistence_authority()
        );
    }

    #[test]
    fn model_interface_minimum_does_not_consume_broader_provider_authority() {
        let network = CapabilityId::parse("network.openai").unwrap();
        let full = Authority::new(
            persistence_authority()
                .capabilities()
                .cloned()
                .chain([network.clone()]),
        );
        let manifest = model_routing_component_manifest(full);
        assert!(manifest.maximum_authority.permits(&network));
        assert!(!manifest.exports[0].required_authority.permits(&network));
    }

    #[test]
    fn replacement_model_component_uses_the_same_typed_import_without_first_party_privilege() {
        let first_party = model_routing_manifest(Authority::default());
        let graph = ResolvedComponentGraph::compile(
            [
                consumer_manifest(Authority::default()),
                first_party,
                replacement_manifest(),
            ],
            [
                consumer_component(Authority::default()),
                model_routing_component_manifest(Authority::default()),
                replacement_component(),
            ],
            &Authority::default(),
        )
        .unwrap();

        let handle = graph
            .import_handle(
                &component("fixture.model-consumer"),
                &ModelRoutingInterface::interface_id(),
            )
            .unwrap()
            .unwrap();
        assert_eq!(handle.exporter(), &component("fixture.model-replacement"));
        assert_eq!(handle.owning_plugin(), &plugin("fixture.model-replacement"));
    }

    #[test]
    fn first_party_model_binds_with_its_interface_minimum_not_its_package_ceiling() {
        let network = CapabilityId::parse("network.openai").unwrap();
        let package_authority = Authority::new(
            persistence_authority()
                .capabilities()
                .cloned()
                .chain([network]),
        );
        let consumer_authority = persistence_authority();
        let graph = ResolvedComponentGraph::compile(
            [
                consumer_manifest(consumer_authority.clone()),
                model_routing_manifest(package_authority.clone()),
            ],
            [
                consumer_component(consumer_authority.clone()),
                model_routing_component_manifest(package_authority),
            ],
            &consumer_authority,
        )
        .unwrap();

        assert!(graph
            .import_handle(
                &component("fixture.model-consumer"),
                &ModelRoutingInterface::interface_id(),
            )
            .unwrap()
            .is_some());
    }

    #[test]
    fn model_binding_retains_granted_authority_beyond_the_interface_minimum() {
        let network = CapabilityId::parse("network.openai").unwrap();
        let full = Authority::new(
            persistence_authority()
                .capabilities()
                .cloned()
                .chain([network.clone()]),
        );
        let graph = ResolvedComponentGraph::compile(
            [
                consumer_manifest(full.clone()),
                model_routing_manifest(full.clone()),
            ],
            [
                consumer_component(full.clone()),
                model_routing_component_manifest(full.clone()),
            ],
            &full,
        )
        .unwrap();

        let handle = graph
            .import_handle(
                &component("fixture.model-consumer"),
                &ModelRoutingInterface::interface_id(),
            )
            .unwrap()
            .unwrap();
        assert!(handle.effective_authority().permits(&network));
        assert!(handle
            .effective_authority()
            .permits_all(&persistence_authority()));
    }
}
