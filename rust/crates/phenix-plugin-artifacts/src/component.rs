use crate::{artifact_manifest, ARTIFACT_SERVICE};
use phenix_core::{
    Authority, ComponentExport, ComponentId, ComponentInterface, ComponentManifest, InterfaceId,
    PluginId,
};

const ARTIFACT_COMPONENT: &str = "phenix.artifacts";
const ARTIFACT_PLUGIN: &str = "phenix.artifacts";

pub struct ArtifactInterface;

impl ComponentInterface for ArtifactInterface {
    fn interface_id() -> InterfaceId {
        InterfaceId::parse(ARTIFACT_SERVICE).expect("static artifact interface id is valid")
    }

    fn schema() -> phenix_core::InterfaceSchema {
        phenix_core::InterfaceSchema::of::<crate::ArtifactCommand, crate::ArtifactResponse>()
    }
}

#[must_use]
pub fn artifact_component_id() -> ComponentId {
    ComponentId::parse(ARTIFACT_COMPONENT).expect("static artifact component id is valid")
}

#[must_use]
pub fn artifact_component_manifest() -> ComponentManifest {
    ComponentManifest {
        id: artifact_component_id(),
        owner: PluginId::parse(ARTIFACT_PLUGIN).expect("static artifact plugin id is valid"),
        imports: Vec::new(),
        exports: vec![ComponentExport {
            interface: ArtifactInterface::interface_id(),
            schema: ArtifactInterface::schema(),
            priority: 100,
            required_authority: Authority::default(),
        }],
        maximum_authority: artifact_manifest().maximum_authority,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_core::{ComponentImport, PluginExecution, PluginManifest, ResolvedComponentGraph};

    #[test]
    fn artifact_service_binds_as_an_ordinary_typed_component() {
        let consumer_plugin = PluginManifest {
            id: PluginId::parse("fixture.artifact-consumer").unwrap(),
            version: 1,
            execution: PluginExecution::Embedded,
            dependencies: Vec::new(),
            services: Vec::new(),
            resource_namespaces: Vec::new(),
            maximum_authority: Authority::default(),
        };
        let consumer = ComponentManifest {
            id: ComponentId::parse("fixture.artifact-consumer").unwrap(),
            owner: consumer_plugin.id.clone(),
            imports: vec![ComponentImport {
                interface: ArtifactInterface::interface_id(),
                schema: ArtifactInterface::schema(),
                required: true,
                authority: Authority::default(),
            }],
            exports: Vec::new(),
            maximum_authority: Authority::default(),
        };
        let graph = ResolvedComponentGraph::compile(
            [consumer_plugin, artifact_manifest()],
            [consumer, artifact_component_manifest()],
            &artifact_manifest().maximum_authority,
        )
        .unwrap();

        let handle = graph
            .import_handle(
                &ComponentId::parse("fixture.artifact-consumer").unwrap(),
                &ArtifactInterface::interface_id(),
            )
            .unwrap()
            .unwrap();
        assert_eq!(handle.exporter(), &artifact_component_id());
    }
}
