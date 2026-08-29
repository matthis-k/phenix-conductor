use crate::{
    ComponentManifest, InterfaceId, PluginId, PluginManifest, ResolvedHarness, SdkNamespace,
    SdkResourceId,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt::{self, Display, Formatter},
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SdkContribution {
    pub provider: PluginId,
    pub namespace: SdkNamespace,
    pub interfaces: BTreeSet<InterfaceId>,
    pub resources: BTreeSet<SdkResourceId>,
}

impl SdkContribution {
    pub fn new(provider: PluginId, namespace: SdkNamespace) -> Self {
        Self {
            provider,
            namespace,
            interfaces: BTreeSet::new(),
            resources: BTreeSet::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SdkResolutionError {
    UnknownProvider {
        namespace: SdkNamespace,
        provider: PluginId,
    },
    DuplicateNamespace(SdkNamespace),
    UnavailableInterface {
        namespace: SdkNamespace,
        interface: InterfaceId,
    },
}

impl Display for SdkResolutionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownProvider {
                namespace,
                provider,
            } => write!(
                f,
                "SDK namespace {namespace} references unselected provider {provider}"
            ),
            Self::DuplicateNamespace(namespace) => {
                write!(f, "multiple plugins provide SDK namespace {namespace}")
            }
            Self::UnavailableInterface {
                namespace,
                interface,
            } => write!(
                f,
                "SDK namespace {namespace} references unavailable interface {interface}"
            ),
        }
    }
}

impl Error for SdkResolutionError {}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ResolvedSdkContributions {
    namespaces: BTreeMap<SdkNamespace, SdkContribution>,
}

impl ResolvedSdkContributions {
    pub fn resolve(
        plugins: &[PluginManifest],
        components: &[ComponentManifest],
        contributions: impl IntoIterator<Item = SdkContribution>,
    ) -> Result<Self, SdkResolutionError> {
        let providers: BTreeSet<_> = plugins.iter().map(|plugin| plugin.id.clone()).collect();
        let interfaces: BTreeSet<_> = components
            .iter()
            .flat_map(|component| component.exports.iter())
            .map(|export| export.interface.clone())
            .collect();
        let mut namespaces = BTreeMap::new();

        for contribution in contributions {
            if !providers.contains(&contribution.provider) {
                return Err(SdkResolutionError::UnknownProvider {
                    namespace: contribution.namespace,
                    provider: contribution.provider,
                });
            }
            if let Some(interface) = contribution
                .interfaces
                .iter()
                .find(|interface| !interfaces.contains(*interface))
            {
                return Err(SdkResolutionError::UnavailableInterface {
                    namespace: contribution.namespace,
                    interface: interface.clone(),
                });
            }
            let namespace = contribution.namespace.clone();
            if namespaces.insert(namespace.clone(), contribution).is_some() {
                return Err(SdkResolutionError::DuplicateNamespace(namespace));
            }
        }

        Ok(Self { namespaces })
    }

    pub fn get(&self, namespace: &SdkNamespace) -> Option<&SdkContribution> {
        self.namespaces.get(namespace)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&SdkNamespace, &SdkContribution)> {
        self.namespaces.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.namespaces.is_empty()
    }
}

impl ResolvedHarness {
    pub fn resolve_sdk_contributions(
        &self,
        contributions: impl IntoIterator<Item = SdkContribution>,
    ) -> Result<ResolvedSdkContributions, SdkResolutionError> {
        ResolvedSdkContributions::resolve(self.plugins(), self.components(), contributions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Authority, ComponentExport, ComponentId, PluginExecution};

    fn plugin(value: &str, execution: PluginExecution) -> PluginManifest {
        PluginManifest {
            id: PluginId::parse(value).unwrap(),
            version: 1,
            execution,
            dependencies: Vec::new(),
            services: Vec::new(),
            resource_namespaces: Vec::new(),
            maximum_authority: Authority::default(),
        }
    }

    fn component(owner: &str, interface: &str) -> ComponentManifest {
        ComponentManifest {
            id: ComponentId::parse(format!("{owner}.component")).unwrap(),
            owner: PluginId::parse(owner).unwrap(),
            imports: Vec::new(),
            exports: vec![ComponentExport {
                interface: InterfaceId::parse(interface).unwrap(),
                priority: 0,
                required_authority: Authority::default(),
            }],
            maximum_authority: Authority::default(),
        }
    }

    fn contribution(provider: &str, namespace: &str) -> SdkContribution {
        SdkContribution::new(
            PluginId::parse(provider).unwrap(),
            SdkNamespace::parse(namespace).unwrap(),
        )
    }

    #[test]
    fn distinct_plugins_extend_distinct_sdk_namespaces() {
        let plugins = [
            plugin("phenix-sdk", PluginExecution::ResourceOnly),
            plugin("testing", PluginExecution::ResourceOnly),
        ];
        let resolved = ResolvedSdkContributions::resolve(
            &plugins,
            &[],
            [
                contribution("phenix-sdk", "phenix"),
                contribution("testing", "testing"),
            ],
        )
        .unwrap();

        assert_eq!(resolved.iter().count(), 2);
    }

    #[test]
    fn resource_only_plugin_can_publish_client_helpers() {
        let plugins = [plugin("testing", PluginExecution::ResourceOnly)];
        let mut testing = contribution("testing", "testing");
        testing
            .resources
            .insert(SdkResourceId::parse("sdk/rust/testing").unwrap());

        let resolved = ResolvedSdkContributions::resolve(&plugins, &[], [testing]).unwrap();

        assert!(resolved
            .get(&SdkNamespace::parse("testing").unwrap())
            .unwrap()
            .resources
            .contains(&SdkResourceId::parse("sdk/rust/testing").unwrap()));
    }

    #[test]
    fn sdk_interface_must_exist_in_selected_component_graph() {
        let plugins = [plugin("testing", PluginExecution::Embedded)];
        let mut testing = contribution("testing", "testing");
        testing
            .interfaces
            .insert(InterfaceId::parse("testing.inspect@1").unwrap());

        assert!(matches!(
            ResolvedSdkContributions::resolve(&plugins, &[], [testing]),
            Err(SdkResolutionError::UnavailableInterface { interface, .. })
                if interface == InterfaceId::parse("testing.inspect@1").unwrap()
        ));

        let mut testing = contribution("testing", "testing");
        testing
            .interfaces
            .insert(InterfaceId::parse("testing.inspect@1").unwrap());
        ResolvedSdkContributions::resolve(
            &plugins,
            &[component("testing", "testing.inspect@1")],
            [testing],
        )
        .unwrap();
    }

    #[test]
    fn duplicate_namespace_is_rejected() {
        let plugins = [
            plugin("testing-a", PluginExecution::ResourceOnly),
            plugin("testing-b", PluginExecution::ResourceOnly),
        ];

        assert!(matches!(
            ResolvedSdkContributions::resolve(
                &plugins,
                &[],
                [
                    contribution("testing-a", "testing"),
                    contribution("testing-b", "testing"),
                ],
            ),
            Err(SdkResolutionError::DuplicateNamespace(namespace))
                if namespace == SdkNamespace::parse("testing").unwrap()
        ));
    }

    #[test]
    fn contribution_provider_must_be_selected() {
        assert!(matches!(
            ResolvedSdkContributions::resolve(
                &[],
                &[],
                [contribution("testing", "testing")],
            ),
            Err(SdkResolutionError::UnknownProvider { provider, .. })
                if provider == PluginId::parse("testing").unwrap()
        ));
    }

    #[test]
    fn empty_composition_has_no_sdk_namespaces() {
        assert!(ResolvedSdkContributions::resolve(&[], &[], [])
            .unwrap()
            .is_empty());
    }
}
