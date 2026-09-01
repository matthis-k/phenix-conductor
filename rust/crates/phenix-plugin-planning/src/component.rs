use crate::planning_manifest;
use phenix_core::{ComponentExport, ComponentId, ComponentInterface, ComponentManifest, PluginId};
use phenix_sdk::PlanningInterface;

const PLANNING_COMPONENT: &str = "phenix.planning";
const PLANNING_PLUGIN: &str = "phenix.planning";

#[must_use]
pub fn planning_component_id() -> ComponentId {
    ComponentId::parse(PLANNING_COMPONENT).expect("static planning component id is valid")
}

#[must_use]
pub fn planning_component_manifest() -> ComponentManifest {
    let authority = planning_manifest().maximum_authority;
    ComponentManifest {
        id: planning_component_id(),
        owner: PluginId::parse(PLANNING_PLUGIN).expect("static planning plugin id is valid"),
        imports: Vec::new(),
        exports: vec![ComponentExport {
            interface: PlanningInterface::interface_id(),
            schema: PlanningInterface::schema(),
            priority: 100,
            required_authority: authority.clone(),
        }],
        maximum_authority: authority,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_core::ResolvedComponentGraph;

    #[test]
    fn planning_is_exposed_as_an_ordinary_typed_component() {
        let manifest = planning_component_manifest();
        assert_eq!(manifest.id, planning_component_id());
        assert_eq!(manifest.owner.as_str(), PLANNING_PLUGIN);
        assert_eq!(manifest.exports.len(), 1);
        assert_eq!(
            manifest.exports[0].interface,
            PlanningInterface::interface_id()
        );

        let graph = ResolvedComponentGraph::compile(
            [planning_manifest()],
            [manifest.clone()],
            &planning_manifest().maximum_authority,
        )
        .unwrap();
        let component = graph.component(&planning_component_id()).unwrap();
        assert_eq!(component.owning_plugin.as_str(), PLANNING_PLUGIN);
        assert_eq!(component.maximum_authority, manifest.maximum_authority);
    }
}
