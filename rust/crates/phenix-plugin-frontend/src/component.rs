use crate::frontend_manifest;
use phenix_core::{
    Authority, ComponentExport, ComponentId, ComponentImport, ComponentInterface,
    ComponentManifest, PluginId,
};
use phenix_sdk::{ExecutionInterface, FrontendInterface};

const FRONTEND_COMPONENT: &str = "phenix.frontend-services";
const FRONTEND_PLUGIN: &str = "phenix.frontend-services";

#[must_use]
pub fn frontend_component_id() -> ComponentId {
    ComponentId::parse(FRONTEND_COMPONENT).expect("static frontend component id is valid")
}

#[must_use]
pub fn frontend_component_manifest(maximum_authority: Authority) -> ComponentManifest {
    let authority = frontend_manifest(maximum_authority).maximum_authority;
    ComponentManifest {
        id: frontend_component_id(),
        owner: PluginId::parse(FRONTEND_PLUGIN).expect("static frontend plugin id is valid"),
        imports: vec![ComponentImport {
            interface: ExecutionInterface::interface_id(),
            schema: ExecutionInterface::schema(),
            required: true,
            authority: authority.clone(),
        }],
        exports: vec![ComponentExport {
            interface: FrontendInterface::interface_id(),
            schema: FrontendInterface::schema(),
            priority: 100,
            required_authority: Authority::default(),
        }],
        maximum_authority: authority,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_core::{ComponentGraphError, ResolvedComponentGraph};
    use phenix_plugin_execution::{execution_component_manifest, execution_manifest};

    #[test]
    fn frontend_requires_execution_binding_before_activation() {
        let authority = Authority::default();
        let error = ResolvedComponentGraph::compile(
            [
                execution_manifest(authority.clone()),
                frontend_manifest(authority.clone()),
            ],
            [frontend_component_manifest(authority.clone())],
            &authority,
        )
        .unwrap_err();

        assert!(matches!(
            error,
            ComponentGraphError::MissingRequiredImport { component, interface }
                if component == frontend_component_id()
                    && interface == ExecutionInterface::interface_id()
        ));
    }

    #[test]
    fn frontend_execution_import_binds_deterministically() {
        let execution = execution_manifest(Authority::default());
        let authority = execution.maximum_authority.clone();
        let graph = ResolvedComponentGraph::compile(
            [execution, frontend_manifest(authority.clone())],
            [
                execution_component_manifest(authority.clone()),
                frontend_component_manifest(authority.clone()),
            ],
            &authority,
        )
        .unwrap();
        let handle = graph
            .import_handle(
                &frontend_component_id(),
                &ExecutionInterface::interface_id(),
            )
            .unwrap()
            .unwrap();

        assert_eq!(
            handle.exporter(),
            &execution_component_manifest(authority).id
        );
    }
}
