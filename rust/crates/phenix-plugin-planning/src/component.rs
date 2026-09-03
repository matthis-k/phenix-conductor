use crate::Plugin;
use phenix_core::{ComponentId, ComponentManifest};
use phenix_sdk::StaticPluginDefinition;

const PLANNING_PLUGIN: &str = "phenix.planning";

#[must_use]
pub fn planning_component_id() -> ComponentId {
    planning_component_manifest().id
}

#[must_use]
pub fn planning_component_manifest() -> ComponentManifest {
    Plugin::component_manifests()
        .into_iter()
        .next()
        .expect("planning plugin has one generated component")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planning_manifest;
    use phenix_core::{ComponentInterface, ResolvedComponentGraph};
    use phenix_sdk::PlanningInterface;

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
