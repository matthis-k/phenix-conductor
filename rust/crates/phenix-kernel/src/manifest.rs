use crate::{Authority, PluginId, ResourceNamespace, ServiceId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PluginExecution {
    Embedded,
    External { executable: String },
    ResourceOnly,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceContribution {
    pub service: ServiceId,
    pub priority: i32,
    pub required_authority: Authority,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginManifest {
    pub id: PluginId,
    pub version: u32,
    pub execution: PluginExecution,
    pub dependencies: Vec<PluginId>,
    pub services: Vec<ServiceContribution>,
    pub resource_namespaces: Vec<ResourceNamespace>,
    pub maximum_authority: Authority,
}

impl PluginManifest {
    pub fn resource_only(id: PluginId) -> Self {
        Self {
            id,
            version: 1,
            execution: PluginExecution::ResourceOnly,
            dependencies: Vec::new(),
            services: Vec::new(),
            resource_namespaces: Vec::new(),
            maximum_authority: Authority::default(),
        }
    }
}
