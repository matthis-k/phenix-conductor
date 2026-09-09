use crate::{
    CapabilityOwnerId, ComponentManifest, InterfaceId, ObservableStore, PhenixSchema, PhenixValue,
    PluginId, PluginManifest, ResolvedHarness, SdkNamespace, SdkResourceId, Type, ValueAddress,
    ValueId, ValuePath,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt::{self, Display, Formatter},
};

/// A language-facing SDK is always published as one authoritative schema/value pair.
///
/// Callable values cannot recover their input and output schemas from a raw reference,
/// so consumers must retain this pair through every transport boundary.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SdkValue {
    pub schema: PhenixSchema,
    pub value: crate::PhenixValue,
}

impl SdkValue {
    pub fn new(
        schema: PhenixSchema,
        value: crate::PhenixValue,
    ) -> Result<Self, SdkResolutionError> {
        schema
            .parse(&value)
            .map_err(|error| SdkResolutionError::InvalidValue {
                message: error.to_string(),
            })?;
        Ok(Self { schema, value })
    }
}

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

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SdkContribution {
    pub provider: PluginId,
    pub namespace: SdkNamespace,
    pub interfaces: BTreeSet<InterfaceId>,
    pub resources: BTreeSet<SdkResourceId>,
    #[serde(default)]
    pub observables: BTreeMap<SdkResourceId, SdkObservableResource>,
    #[serde(default)]
    pub value: Option<SdkValue>,
}

impl SdkContribution {
    pub fn new(provider: PluginId, namespace: SdkNamespace) -> Self {
        Self {
            provider,
            namespace,
            interfaces: BTreeSet::new(),
            resources: BTreeSet::new(),
            observables: BTreeMap::new(),
            value: None,
        }
    }

    pub fn publish(&mut self, value: SdkValue) {
        self.value = Some(value);
    }

    pub fn insert_observable(&mut self, observable: SdkObservableResource) {
        self.resources.insert(observable.id.clone());
        self.observables.insert(observable.id.clone(), observable);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SdkResolutionError {
    InvalidValue {
        message: String,
    },
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
            Self::InvalidValue { message } => {
                write!(f, "SDK value does not match its schema: {message}")
            }
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

#[derive(Clone, Debug, Default, PartialEq)]
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
            if let Some(value) = &contribution.value {
                validate_capability_owners(&contribution.provider, &value.value)
                    .map_err(|message| SdkResolutionError::InvalidValue { message })?;
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

    pub fn value(&self) -> Result<SdkValue, SdkResolutionError> {
        let mut schema = BTreeMap::new();
        let mut value = BTreeMap::new();
        for (namespace, contribution) in &self.namespaces {
            let Some(entry) = &contribution.value else {
                continue;
            };
            schema.insert(
                crate::Key::parse(namespace.as_str().to_owned())
                    .expect("SDK namespaces are structural keys"),
                entry.schema.clone(),
            );
            value.insert(
                crate::Key::parse(namespace.as_str().to_owned())
                    .expect("SDK namespaces are structural keys"),
                entry.value.clone(),
            );
        }
        SdkValue::new(Type::Table(schema), crate::PhenixValue::Table(value))
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

fn validate_capability_owners(provider: &PluginId, value: &PhenixValue) -> Result<(), String> {
    match value {
        PhenixValue::Callable(reference) => match reference.owner() {
            CapabilityOwnerId::Plugin(owner) if owner == provider => Ok(()),
            CapabilityOwnerId::Plugin(owner) => Err(format!(
                "callable {} belongs to plugin {owner}, not contribution provider {provider}",
                reference.id()
            )),
            owner => Err(format!(
                "SDK contribution provider {provider} cannot publish {owner:?}-owned callable {}",
                reference.id()
            )),
        },
        PhenixValue::Object(reference) => match reference.owner() {
            CapabilityOwnerId::Plugin(owner) if owner == provider => Ok(()),
            CapabilityOwnerId::Plugin(owner) => Err(format!(
                "object {} belongs to plugin {owner}, not contribution provider {provider}",
                reference.id()
            )),
            owner => Err(format!(
                "SDK contribution provider {provider} cannot publish {owner:?}-owned object {}",
                reference.id()
            )),
        },
        PhenixValue::Option(Some(value)) | PhenixValue::Variant { value, .. } => {
            validate_capability_owners(provider, value)
        }
        PhenixValue::List(values) => values
            .iter()
            .try_for_each(|value| validate_capability_owners(provider, value)),
        PhenixValue::Map(values) | PhenixValue::Table(values) => values
            .values()
            .try_for_each(|value| validate_capability_owners(provider, value)),
        PhenixValue::Unit
        | PhenixValue::Bool(_)
        | PhenixValue::I64(_)
        | PhenixValue::U64(_)
        | PhenixValue::F64(_)
        | PhenixValue::String(_)
        | PhenixValue::Bytes(_)
        | PhenixValue::Option(None) => Ok(()),
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
        Authority, CallableRef, CapabilityGenerationId, CapabilityOwnerId, ClientConnectionId,
        ComponentExport, ComponentId, ContractId, Key, ObservableRegistration, PhenixValue,
        PluginExecution, ReferenceId, SnapshotPolicy, Type,
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

    #[test]
    fn sdk_value_cannot_publish_a_value_outside_its_authoritative_schema() {
        assert!(SdkValue::new(Type::U64, PhenixValue::U64(1)).is_ok());
        assert!(matches!(
            SdkValue::new(Type::U64, PhenixValue::String("wrong".to_owned())),
            Err(SdkResolutionError::InvalidValue { .. })
        ));
    }

    #[test]
    fn contributions_cannot_publish_other_owner_capabilities() {
        let plugins = [plugin("testing", PluginExecution::ResourceOnly)];
        let reference = CallableRef::new(
            ContractId::parse("testing.callback@1").unwrap(),
            CapabilityOwnerId::Client(ClientConnectionId::parse("nvim").unwrap()),
            CapabilityGenerationId::parse("connection-1").unwrap(),
            ReferenceId::parse("callback-1").unwrap(),
        );
        let schema = Type::Callable {
            contract: reference.contract().clone(),
            input: Box::new(Type::Unit),
            output: Box::new(Type::Unit),
        };
        let mut testing = contribution("testing", "testing");
        testing.publish(
            SdkValue::new(schema, PhenixValue::Callable(reference)).expect("matching value"),
        );

        assert!(matches!(
            ResolvedSdkContributions::resolve(&plugins, &[], [testing]),
            Err(SdkResolutionError::InvalidValue { message })
                if message.contains("cannot publish")
        ));
    }

    #[test]
    fn resolved_values_are_projected_under_stable_sdk_namespaces() {
        let plugins = [
            plugin("phenix-sdk", PluginExecution::ResourceOnly),
            plugin("testing", PluginExecution::ResourceOnly),
        ];
        let mut phenix = contribution("phenix-sdk", "phenix");
        phenix.publish(SdkValue::new(Type::U64, PhenixValue::U64(1)).unwrap());
        let mut testing = contribution("testing", "testing");
        testing
            .publish(SdkValue::new(Type::String, PhenixValue::String("ready".to_owned())).unwrap());

        let value = ResolvedSdkContributions::resolve(&plugins, &[], [testing, phenix])
            .unwrap()
            .value()
            .unwrap();

        assert_eq!(
            value.schema,
            Type::Table(BTreeMap::from([
                (Key::parse("phenix").unwrap(), Type::U64),
                (Key::parse("testing").unwrap(), Type::String),
            ]))
        );
        assert_eq!(
            value.value,
            PhenixValue::Table(BTreeMap::from([
                (Key::parse("phenix").unwrap(), PhenixValue::U64(1)),
                (
                    Key::parse("testing").unwrap(),
                    PhenixValue::String("ready".to_owned()),
                ),
            ]))
        );
    }
}
