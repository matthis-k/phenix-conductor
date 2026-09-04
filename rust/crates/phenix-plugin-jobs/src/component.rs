use crate::Plugin;
use phenix_core::{ComponentId, ComponentManifest};
use phenix_sdk::StaticPluginDefinition;

#[must_use]
pub fn job_component_id() -> ComponentId {
    job_component_manifest().id
}

#[must_use]
pub fn job_component_manifest() -> ComponentManifest {
    Plugin::component_manifests()
        .into_iter()
        .next()
        .expect("jobs plugin has one generated component")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job_manifest;
    use phenix_core::{
        Authority, ComponentImport, ComponentInterface, PluginExecution, PluginId, PluginManifest,
        ResolvedComponentGraph,
    };
    use phenix_sdk::JobInterface;

    fn consumer_manifest() -> PluginManifest {
        PluginManifest {
            id: PluginId::parse("fixture.job-consumer").unwrap(),
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
            listeners: Vec::new(),
            id: ComponentId::parse("fixture.job-consumer").unwrap(),
            owner: PluginId::parse("fixture.job-consumer").unwrap(),
            imports: vec![ComponentImport {
                interface: JobInterface::interface_id(),
                schema: JobInterface::schema(),
                required: true,
                authority: Authority::default(),
            }],
            exports: Vec::new(),
            maximum_authority: Authority::default(),
        }
    }

    #[test]
    fn jobs_are_available_through_an_ordinary_typed_binding() {
        let graph = ResolvedComponentGraph::compile(
            [consumer_manifest(), job_manifest()],
            [consumer_component(), job_component_manifest()],
            &job_manifest().maximum_authority,
        )
        .unwrap();

        let handle = graph
            .import_handle(
                &ComponentId::parse("fixture.job-consumer").unwrap(),
                &JobInterface::interface_id(),
            )
            .unwrap()
            .unwrap();

        assert_eq!(handle.exporter(), &job_component_id());
        assert_eq!(handle.effective_authority(), &Authority::default());
    }
}
