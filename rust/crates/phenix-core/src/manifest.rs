use crate::{Authority, ComponentId, InterfaceId, PluginId, ResourceNamespace, ServiceId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PluginExecution {
    Embedded,
    External { executable: String },
    ResourceOnly,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceRole {
    Terminal,
    Layer,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ServiceContribution {
    pub service: ServiceId,
    pub role: ServiceRole,
    pub priority: i32,
    pub required_authority: Authority,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentImport {
    pub interface: InterfaceId,
    pub required: bool,
    pub authority: Authority,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentExport {
    pub interface: InterfaceId,
    pub priority: i32,
    pub required_authority: Authority,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentManifest {
    pub id: ComponentId,
    pub owner: PluginId,
    pub imports: Vec<ComponentImport>,
    pub exports: Vec<ComponentExport>,
    pub maximum_authority: Authority,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composition_metadata_is_inspectable_without_plugin_activation() {
        let manifest = PluginManifest {
            id: PluginId::parse("third-party").unwrap(),
            version: 7,
            execution: PluginExecution::External {
                executable: "third-party-host".into(),
            },
            dependencies: Vec::new(),
            services: Vec::new(),
            resource_namespaces: Vec::new(),
            maximum_authority: Authority::default(),
        };
        let encoded = serde_json::to_value(&manifest).unwrap();

        assert_eq!(encoded["id"], "third-party");
        assert_eq!(encoded["version"], 7);
        assert_eq!(encoded["execution"]["kind"], "external");
        assert_eq!(encoded["execution"]["executable"], "third-party-host");
    }
}
