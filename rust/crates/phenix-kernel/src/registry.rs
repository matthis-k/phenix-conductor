use crate::{Authority, PluginExecution, PluginId, PluginManifest, ResourceNamespace, ServiceId};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt::{self, Display, Formatter},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum KernelError {
    DuplicatePlugin(PluginId),
    UnknownDependency {
        plugin: PluginId,
        dependency: PluginId,
    },
    DependencyCycle(PluginId),
    DuplicateResourceNamespace(ResourceNamespace),
    DuplicateServiceContribution {
        plugin: PluginId,
        service: ServiceId,
    },
    ResourceOnlyService(PluginId),
    UnknownPlugin(PluginId),
    NoEligibleProvider(ServiceId),
    BoundProviderUnavailable {
        service: ServiceId,
        plugin: PluginId,
    },
    PluginNotActive(PluginId),
    HostOperationDenied {
        plugin: PluginId,
        operation: String,
    },
    Persistence {
        plugin: PluginId,
        message: String,
    },
    EmbeddedFactoryMissing(PluginId),
    WrongExecutionKind(PluginId),
    ExternalHostUnavailable(PluginId),
    PluginStart {
        plugin: PluginId,
        message: String,
    },
    ServiceInvoke {
        plugin: PluginId,
        service: ServiceId,
        message: String,
    },
}

impl Display for KernelError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicatePlugin(plugin) => write!(f, "duplicate plugin: {plugin}"),
            Self::UnknownDependency { plugin, dependency } => {
                write!(f, "plugin {plugin} depends on unknown plugin {dependency}")
            }
            Self::DependencyCycle(plugin) => write!(f, "plugin dependency cycle includes {plugin}"),
            Self::DuplicateResourceNamespace(namespace) => {
                write!(f, "resource namespace has multiple owners: {namespace}")
            }
            Self::DuplicateServiceContribution { plugin, service } => write!(
                f,
                "plugin {plugin} contributes service {service} more than once"
            ),
            Self::ResourceOnlyService(plugin) => {
                write!(
                    f,
                    "resource-only plugin cannot provide executable services: {plugin}"
                )
            }
            Self::UnknownPlugin(plugin) => write!(f, "unknown plugin: {plugin}"),
            Self::NoEligibleProvider(service) => {
                write!(f, "no eligible provider for service {service}")
            }
            Self::BoundProviderUnavailable { service, plugin } => write!(
                f,
                "bound provider {plugin} is unavailable for service {service}"
            ),
            Self::PluginNotActive(plugin) => write!(f, "plugin is not active: {plugin}"),
            Self::HostOperationDenied { plugin, operation } => {
                write!(f, "plugin {plugin} is not allowed to perform host operation {operation}")
            }
            Self::Persistence { plugin, message } => {
                write!(f, "plugin {plugin} persistence operation failed: {message}")
            }
            Self::EmbeddedFactoryMissing(plugin) => {
                write!(f, "embedded plugin has no registered factory: {plugin}")
            }
            Self::WrongExecutionKind(plugin) => write!(
                f,
                "plugin execution kind does not match requested host: {plugin}"
            ),
            Self::ExternalHostUnavailable(plugin) => {
                write!(f, "external plugin host is not implemented for {plugin}")
            }
            Self::PluginStart { plugin, message } => {
                write!(f, "plugin {plugin} failed to start: {message}")
            }
            Self::ServiceInvoke {
                plugin,
                service,
                message,
            } => write!(f, "plugin {plugin} failed service {service}: {message}"),
        }
    }
}

impl Error for KernelError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderBinding {
    pub service: ServiceId,
    pub plugin: PluginId,
    pub priority: i32,
}

#[derive(Clone, Debug)]
pub struct KernelConfig {
    manifests: BTreeMap<PluginId, PluginManifest>,
    activation_order: Vec<PluginId>,
    namespace_owners: BTreeMap<ResourceNamespace, PluginId>,
}

impl KernelConfig {
    pub fn new(manifests: impl IntoIterator<Item = PluginManifest>) -> Result<Self, KernelError> {
        let mut indexed = BTreeMap::new();
        for manifest in manifests {
            let id = manifest.id.clone();
            if indexed.insert(id.clone(), manifest).is_some() {
                return Err(KernelError::DuplicatePlugin(id));
            }
        }

        let namespace_owners = validate_namespaces(&indexed)?;
        validate_contributions(&indexed)?;
        let activation_order = dependency_order(&indexed)?;

        Ok(Self {
            manifests: indexed,
            activation_order,
            namespace_owners,
        })
    }

    pub fn empty() -> Self {
        Self::new([]).expect("empty kernel configuration is valid")
    }

    pub fn manifest(&self, plugin: &PluginId) -> Option<&PluginManifest> {
        self.manifests.get(plugin)
    }

    pub fn manifests(&self) -> impl Iterator<Item = &PluginManifest> {
        self.manifests.values()
    }

    pub fn activation_order(&self) -> &[PluginId] {
        &self.activation_order
    }

    pub fn resource_owner(&self, namespace: &ResourceNamespace) -> Option<&PluginId> {
        self.namespace_owners.get(namespace)
    }

    pub fn resolve(
        &self,
        service: &ServiceId,
        caller_authority: &Authority,
        binding: Option<&PluginId>,
    ) -> Result<ProviderBinding, KernelError> {
        let mut candidates = Vec::new();

        for manifest in self.manifests.values() {
            if binding.is_some_and(|bound| bound != &manifest.id) {
                continue;
            }
            for contribution in &manifest.services {
                if &contribution.service != service {
                    continue;
                }
                if caller_authority.permits_all(&contribution.required_authority) {
                    candidates.push(ProviderBinding {
                        service: service.clone(),
                        plugin: manifest.id.clone(),
                        priority: contribution.priority,
                    });
                }
            }
        }

        candidates.sort_by(|left, right| {
            right
                .priority
                .cmp(&left.priority)
                .then_with(|| left.plugin.cmp(&right.plugin))
        });

        if let Some(candidate) = candidates.into_iter().next() {
            return Ok(candidate);
        }

        if let Some(plugin) = binding {
            return Err(KernelError::BoundProviderUnavailable {
                service: service.clone(),
                plugin: plugin.clone(),
            });
        }

        Err(KernelError::NoEligibleProvider(service.clone()))
    }

    pub fn can_execute(&self, plugin: &PluginId) -> Result<bool, KernelError> {
        let manifest = self
            .manifests
            .get(plugin)
            .ok_or_else(|| KernelError::UnknownPlugin(plugin.clone()))?;
        Ok(!matches!(manifest.execution, PluginExecution::ResourceOnly))
    }
}

fn validate_namespaces(
    manifests: &BTreeMap<PluginId, PluginManifest>,
) -> Result<BTreeMap<ResourceNamespace, PluginId>, KernelError> {
    let mut owners = BTreeMap::new();
    for manifest in manifests.values() {
        for namespace in &manifest.resource_namespaces {
            if owners
                .insert(namespace.clone(), manifest.id.clone())
                .is_some()
            {
                return Err(KernelError::DuplicateResourceNamespace(namespace.clone()));
            }
        }
    }
    Ok(owners)
}

fn validate_contributions(
    manifests: &BTreeMap<PluginId, PluginManifest>,
) -> Result<(), KernelError> {
    for manifest in manifests.values() {
        if matches!(manifest.execution, PluginExecution::ResourceOnly)
            && !manifest.services.is_empty()
        {
            return Err(KernelError::ResourceOnlyService(manifest.id.clone()));
        }
        let mut seen = BTreeSet::new();
        for contribution in &manifest.services {
            if !seen.insert(contribution.service.clone()) {
                return Err(KernelError::DuplicateServiceContribution {
                    plugin: manifest.id.clone(),
                    service: contribution.service.clone(),
                });
            }
        }
    }
    Ok(())
}

fn dependency_order(
    manifests: &BTreeMap<PluginId, PluginManifest>,
) -> Result<Vec<PluginId>, KernelError> {
    for manifest in manifests.values() {
        for dependency in &manifest.dependencies {
            if !manifests.contains_key(dependency) {
                return Err(KernelError::UnknownDependency {
                    plugin: manifest.id.clone(),
                    dependency: dependency.clone(),
                });
            }
        }
    }

    #[derive(Clone, Copy, Eq, PartialEq)]
    enum Visit {
        Visiting,
        Done,
    }

    fn visit(
        plugin: &PluginId,
        manifests: &BTreeMap<PluginId, PluginManifest>,
        visits: &mut BTreeMap<PluginId, Visit>,
        order: &mut Vec<PluginId>,
    ) -> Result<(), KernelError> {
        match visits.get(plugin) {
            Some(Visit::Done) => return Ok(()),
            Some(Visit::Visiting) => return Err(KernelError::DependencyCycle(plugin.clone())),
            None => {}
        }
        visits.insert(plugin.clone(), Visit::Visiting);
        for dependency in &manifests[plugin].dependencies {
            visit(dependency, manifests, visits, order)?;
        }
        visits.insert(plugin.clone(), Visit::Done);
        order.push(plugin.clone());
        Ok(())
    }

    let mut visits = BTreeMap::new();
    let mut order = Vec::new();
    for plugin in manifests.keys() {
        visit(plugin, manifests, &mut visits, &mut order)?;
    }
    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CapabilityId, ServiceContribution};

    fn plugin(value: &str) -> PluginId {
        PluginId::parse(value).unwrap()
    }

    fn service(value: &str) -> ServiceId {
        ServiceId::parse(value).unwrap()
    }

    fn capability(value: &str) -> CapabilityId {
        CapabilityId::parse(value).unwrap()
    }

    fn manifest(id: &str, priority: i32, required: Authority) -> PluginManifest {
        PluginManifest {
            id: plugin(id),
            version: 1,
            execution: PluginExecution::Embedded,
            dependencies: Vec::new(),
            services: vec![ServiceContribution {
                service: service("demo.service@1"),
                priority,
                required_authority: required,
            }],
            resource_namespaces: Vec::new(),
            maximum_authority: Authority::default(),
        }
    }

    #[test]
    fn provider_resolution_is_deterministic() {
        let config = KernelConfig::new([
            manifest("z-provider", 10, Authority::default()),
            manifest("a-provider", 10, Authority::default()),
            manifest("higher", 20, Authority::default()),
        ])
        .unwrap();

        let selected = config
            .resolve(&service("demo.service@1"), &Authority::default(), None)
            .unwrap();
        assert_eq!(selected.plugin, plugin("higher"));

        let equal_priority = KernelConfig::new([
            manifest("z-provider", 10, Authority::default()),
            manifest("a-provider", 10, Authority::default()),
        ])
        .unwrap();
        assert_eq!(
            equal_priority
                .resolve(&service("demo.service@1"), &Authority::default(), None)
                .unwrap()
                .plugin,
            plugin("a-provider")
        );
    }

    #[test]
    fn authority_filters_before_priority() {
        let read = capability("fs.read");
        let write = capability("fs.write");
        let config = KernelConfig::new([
            manifest("readable", 1, Authority::new([read.clone()])),
            manifest("forbidden", 100, Authority::new([write])),
        ])
        .unwrap();

        let selected = config
            .resolve(&service("demo.service@1"), &Authority::new([read]), None)
            .unwrap();
        assert_eq!(selected.plugin, plugin("readable"));
    }

    #[test]
    fn provider_grant_does_not_become_caller_requirement() {
        let read = capability("fs.read");
        let network = capability("network.read");
        let mut provider = manifest("provider", 1, Authority::new([read.clone()]));
        provider.maximum_authority = Authority::new([network]);
        let config = KernelConfig::new([provider]).unwrap();

        assert_eq!(
            config
                .resolve(&service("demo.service@1"), &Authority::new([read]), None)
                .unwrap()
                .plugin,
            plugin("provider")
        );
    }

    #[test]
    fn dependencies_are_order_independent_and_cycles_fail() {
        let mut a = PluginManifest::resource_only(plugin("a"));
        a.dependencies.push(plugin("b"));
        let b = PluginManifest::resource_only(plugin("b"));
        let config = KernelConfig::new([a.clone(), b]).unwrap();
        assert_eq!(config.activation_order(), &[plugin("b"), plugin("a")]);

        let mut b = PluginManifest::resource_only(plugin("b"));
        b.dependencies.push(plugin("a"));
        assert!(matches!(
            KernelConfig::new([a, b]),
            Err(KernelError::DependencyCycle(_))
        ));
    }

    #[test]
    fn resource_namespace_has_one_owner() {
        let namespace = ResourceNamespace::parse("demo.data").unwrap();
        let mut a = PluginManifest::resource_only(plugin("a"));
        a.resource_namespaces.push(namespace.clone());
        let mut b = PluginManifest::resource_only(plugin("b"));
        b.resource_namespaces.push(namespace.clone());

        assert_eq!(
            KernelConfig::new([a, b]).unwrap_err(),
            KernelError::DuplicateResourceNamespace(namespace)
        );
    }

    #[test]
    fn resource_only_plugins_cannot_execute_or_contribute_services() {
        let id = plugin("resources");
        let config = KernelConfig::new([PluginManifest::resource_only(id.clone())]).unwrap();
        assert!(!config.can_execute(&id).unwrap());

        let mut invalid = PluginManifest::resource_only(id.clone());
        invalid.services.push(ServiceContribution {
            service: service("demo.service@1"),
            priority: 0,
            required_authority: Authority::default(),
        });
        assert_eq!(
            KernelConfig::new([invalid]).unwrap_err(),
            KernelError::ResourceOnlyService(id)
        );
    }
}
