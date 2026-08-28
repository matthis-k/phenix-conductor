use crate::{language_manifest, LanguageCommand, LanguageResponse, LANGUAGE_SERVICE};
use phenix_core::{
    Authority, ComponentExport, ComponentId, ComponentInterface, ComponentManifest, InterfaceId,
    PluginId,
};

const LANGUAGE_COMPONENT: &str = "phenix.language";
const LANGUAGE_PLUGIN: &str = "phenix.language";

pub struct LanguageInterface;

impl ComponentInterface for LanguageInterface {
    type Request = LanguageCommand;
    type Response = LanguageResponse;

    fn interface_id() -> InterfaceId {
        InterfaceId::parse(LANGUAGE_SERVICE).expect("static language interface id is valid")
    }
}

#[must_use]
pub fn language_component_id() -> ComponentId {
    ComponentId::parse(LANGUAGE_COMPONENT).expect("static language component id is valid")
}

#[must_use]
pub fn language_component_manifest() -> ComponentManifest {
    ComponentManifest {
        id: language_component_id(),
        owner: PluginId::parse(LANGUAGE_PLUGIN).expect("static language plugin id is valid"),
        imports: Vec::new(),
        exports: vec![ComponentExport {
            interface: LanguageInterface::interface_id(),
            priority: 100,
            required_authority: Authority::default(),
        }],
        maximum_authority: language_manifest().maximum_authority,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_core::{ComponentImport, PluginExecution, PluginManifest, ResolvedComponentGraph};

    fn consumer_manifest() -> PluginManifest {
        PluginManifest {
            id: PluginId::parse("fixture.language-consumer").unwrap(),
            version: 1,
            execution: PluginExecution::Embedded,
            dependencies: Vec::new(),
            services: Vec::new(),
            resource_namespaces: Vec::new(),
            maximum_authority: Authority::default(),
        }
    }

    fn consumer_component() -> ComponentManifest {
        ComponentManifest {
            id: ComponentId::parse("fixture.language-consumer").unwrap(),
            owner: PluginId::parse("fixture.language-consumer").unwrap(),
            imports: vec![ComponentImport {
                interface: LanguageInterface::interface_id(),
                required: true,
                authority: Authority::default(),
            }],
            exports: Vec::new(),
            maximum_authority: Authority::default(),
        }
    }

    #[test]
    fn language_is_an_ordinary_typed_component_binding() {
        let graph = ResolvedComponentGraph::compile(
            [consumer_manifest(), language_manifest()],
            [consumer_component(), language_component_manifest()],
            &language_manifest().maximum_authority,
        )
        .unwrap();

        let handle = graph
            .import_handle(
                &ComponentId::parse("fixture.language-consumer").unwrap(),
                &LanguageInterface::interface_id(),
            )
            .unwrap()
            .unwrap();

        assert_eq!(handle.exporter(), &language_component_id());
        assert_eq!(handle.effective_authority(), &Authority::default());
    }
}
