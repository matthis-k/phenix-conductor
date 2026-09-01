use crate::job_manifest;
use phenix_core::{
    Authority, ComponentExport, ComponentId, ComponentInterface, ComponentManifest, PluginId,
};
use phenix_sdk::JobInterface;

const JOB_COMPONENT: &str = "phenix.jobs";
const JOB_PLUGIN: &str = "phenix.jobs";

#[must_use]
pub fn job_component_id() -> ComponentId {
    ComponentId::parse(JOB_COMPONENT).expect("static job component id is valid")
}

#[must_use]
pub fn job_component_manifest() -> ComponentManifest {
    ComponentManifest {
        id: job_component_id(),
        owner: PluginId::parse(JOB_PLUGIN).expect("static job plugin id is valid"),
        imports: Vec::new(),
        exports: vec![ComponentExport {
            interface: JobInterface::interface_id(),
            schema: JobInterface::schema(),
            priority: 100,
            required_authority: Authority::default(),
        }],
        maximum_authority: job_manifest().maximum_authority,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_core::{ComponentImport, PluginExecution, PluginManifest, ResolvedComponentGraph};

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
