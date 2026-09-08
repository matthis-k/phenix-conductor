use crate::{
    ComponentManifest, InterfaceId, ObservableStore, PhenixSchema, PluginId, PluginManifest,
    ResolvedHarness, SdkNamespace, SdkResourceId, ValueAddress, ValueId, ValuePath,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt::{self, Display, Formatter},
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SdkObservableResource {
    pub id: SdkResourceId,
    pub binding_path: Vec<String>,
    pub value: ValueId,
    pub path: ValuePath,
    pub schema: PhenixSchema,
}

impl SdkObservableResource {
    #[must_use]
    pub fn new(
        id: SdkResourceId,
        binding_path: impl IntoIterator<Item = impl Into<String>>,
        value: ValueId,
        path: ValuePath,
        schema: PhenixSchema,
    ) -> Self {
        Self {
            id,
            binding_path: binding_path.into_iter().map(Into::into).collect(),
            value,
            path,
            schema,
        }
    }

    #[must_use]
    pub fn address(&self) -> ValueAddress {
        ValueAddress {
            value: self.value.clone(),
            path: self.path.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SdkContribution {
    pub provider: PluginId,
    pub namespace: SdkNamespace,
    pub interfaces: BTreeSet<InterfaceId>,
    pub resources: BTreeSet<SdkResourceId>,
    #[serde(default)]
    pub observables: BTreeMap<SdkResourceId, SdkObservableResource>,
}

impl SdkContribution {
    pub fn new(provider: PluginId, namespace: SdkNamespace) -> Self {
        Self {
            provider,
            namespace,
            interfaces: BTreeSet::new(),
            resources: BTreeSet::new(),
            observables: BTreeMap::new(),
        }
    }

    pub fn insert_observable(&mut self, observable: SdkObservableResource) {
        self.resources.insert(observable.id.clone());
        self.observables.insert(observable.id.clone(), observable);
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
    InvalidObservableResource {
        namespace: SdkNamespace,
        resource: SdkResourceId,
        message: String,
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
            Self::InvalidObservableResource {
                namespace,
                resource,
                message,
            } => write!(
                f,
                "SDK namespace {namespace} observable resource {resource} is invalid: {message}"
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
            for (resource, observable) in &contribution.observables {
                if resource != &observable.id || !contribution.resources.contains(resource) {
                    return Err(SdkResolutionError::InvalidObservableResource {
                        namespace: contribution.namespace.clone(),
                        resource: resource.clone(),
                        message: "observable identity must match and be present in resources"
                            .to_owned(),
                    });
                }
                if observable.binding_path.is_empty()
                    || observable.binding_path.iter().any(String::is_empty)
                {
                    return Err(SdkResolutionError::InvalidObservableResource {
                        namespace: contribution.namespace.clone(),
                        resource: resource.clone(),
                        message: "binding path must contain non-empty segments".to_owned(),
                    });
                }
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

    pub fn validate_observables(&self, store: &ObservableStore) -> Result<(), SdkResolutionError> {
        for (namespace, contribution) in &self.namespaces {
            for (resource, observable) in &contribution.observables {
                let metadata = store.metadata(&observable.value).map_err(|error| {
                    SdkResolutionError::InvalidObservableResource {
                        namespace: namespace.clone(),
                        resource: resource.clone(),
                        message: error.to_string(),
                    }
                })?;
                if metadata.owner != contribution.provider {
                    return Err(SdkResolutionError::InvalidObservableResource {
                        namespace: namespace.clone(),
                        resource: resource.clone(),
                        message: format!(
                            "value {} is owned by {}, not {}",
                            observable.value, metadata.owner, contribution.provider
                        ),
                    });
                }
                let schema = store.schema(&observable.address()).map_err(|error| {
                    SdkResolutionError::InvalidObservableResource {
                        namespace: namespace.clone(),
                        resource: resource.clone(),
                        message: error.to_string(),
                    }
                })?;
                if schema != observable.schema {
                    return Err(SdkResolutionError::InvalidObservableResource {
                        namespace: namespace.clone(),
                        resource: resource.clone(),
                        message: "declared observable schema does not match the registered address"
                            .to_owned(),
                    });
                }
            }
        }
        Ok(())
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
    use crate::{
        Authority, ComponentExport, ComponentId, ObservableRegistration, PhenixValue,
        PluginExecution, SnapshotPolicy, Type,
    };

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
            listeners: Vec::new(),
            id: ComponentId::parse(format!("{owner}.component")).unwrap(),
            owner: PluginId::parse(owner).unwrap(),
            imports: Vec::new(),
            exports: vec![ComponentExport {
                interface: InterfaceId::parse(interface).unwrap(),
                schema: Default::default(),
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

    #[test]
    fn observable_metadata_is_validated_against_the_live_store() {
        let plugins = [plugin("testing", PluginExecution::ResourceOnly)];
        let mut testing = contribution("testing", "testing");
        testing.insert_observable(SdkObservableResource::new(
            SdkResourceId::parse("sdk/testing/state").unwrap(),
            ["state"],
            ValueId::parse("testing.state@1").unwrap(),
            ValuePath::root(),
            Type::U64,
        ));
        let resolved = ResolvedSdkContributions::resolve(&plugins, &[], [testing]).unwrap();
        let store = ObservableStore::default();
        store
            .register(ObservableRegistration {
                id: ValueId::parse("testing.state@1").unwrap(),
                owner: PluginId::parse("testing").unwrap(),
                schema: Type::U64,
                snapshot_policy: SnapshotPolicy::CurrentOnly,
                initial: PhenixValue::U64(0),
            })
            .unwrap();
        resolved.validate_observables(&store).unwrap();
    }
}
