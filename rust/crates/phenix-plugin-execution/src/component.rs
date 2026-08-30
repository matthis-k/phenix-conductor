use crate::{execution_manifest, EXECUTION_SERVICE};
use phenix_core::{
    Authority, CapabilityId, ComponentExport, ComponentId, ComponentInterface, ComponentManifest,
    InterfaceId, PluginId,
};

const EXECUTION_COMPONENT: &str = "phenix.execution";
const EXECUTION_PLUGIN: &str = "phenix.execution";
const PERSISTENCE_SCHEMA: &str = "kernel.persistence.schema";
const PERSISTENCE_READ: &str = "kernel.persistence.read";
const PERSISTENCE_WRITE: &str = "kernel.persistence.write";

pub struct ExecutionInterface;

impl ComponentInterface for ExecutionInterface {
    fn interface_id() -> InterfaceId {
        InterfaceId::parse(EXECUTION_SERVICE).expect("static execution interface id is valid")
    }

    fn schema() -> phenix_core::InterfaceSchema {
        phenix_core::InterfaceSchema::of::<crate::ExecutionCommand, crate::ExecutionResponse>()
    }
}

#[must_use]
pub fn execution_component_id() -> ComponentId {
    ComponentId::parse(EXECUTION_COMPONENT).expect("static component id is valid")
}

#[must_use]
pub fn execution_component_manifest(maximum_authority: Authority) -> ComponentManifest {
    let authority = execution_manifest(maximum_authority).maximum_authority;
    ComponentManifest {
        id: execution_component_id(),
        owner: PluginId::parse(EXECUTION_PLUGIN).expect("static plugin id is valid"),
        imports: Vec::new(),
        exports: vec![ComponentExport {
            interface: ExecutionInterface::interface_id(),
            schema: ExecutionInterface::schema(),
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
    use phenix_core::ResolvedComponentGraph;

    #[test]
    fn execution_component_separates_package_ceiling_from_interface_minimum() {
        let capability = CapabilityId::parse("fixture.execution").unwrap();
        let authority = Authority::new([capability.clone()]);
        let plugin = execution_manifest(authority.clone());
        let component = execution_component_manifest(authority);
        let graph = ResolvedComponentGraph::compile(
            [plugin.clone()],
            [component.clone()],
            &plugin.maximum_authority,
        )
        .unwrap();

        assert_eq!(component.owner, plugin.id);
        assert!(component.maximum_authority.permits(&capability));
        assert_eq!(
            component.exports[0].interface,
            ExecutionInterface::interface_id()
        );
        assert!(!component.exports[0].required_authority.permits(&capability));
        assert_eq!(
            component.exports[0].required_authority,
            persistence_authority()
        );
        assert!(graph.component(&execution_component_id()).is_some());
    }
}
