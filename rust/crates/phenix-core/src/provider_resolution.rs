use crate::{
    Authority, ComponentGraphError, ComponentId, ComponentManifest, ConfigContribution,
    InterfaceId, PluginManifest, ResolvedComponentGraph, ResolvedHarness, ResolvedHarnessError,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Product-owned inputs that affect provider selection during candidate resolution.
///
/// Plugin-authored `ComponentExport::priority` is capability metadata only on this
/// path. Effective preference comes from this policy. An interface without an
/// explicit policy uses equal effective priority and therefore resolves by stable
/// component identity.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProviderCompositionPolicy {
    interfaces: BTreeMap<InterfaceId, InterfaceProviderPolicy>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct InterfaceProviderPolicy {
    #[serde(default)]
    explicit: Option<ComponentId>,
    #[serde(default)]
    priorities: BTreeMap<ComponentId, i32>,
    #[serde(default)]
    disabled: BTreeSet<ComponentId>,
}

impl ProviderCompositionPolicy {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_priority(
        mut self,
        interface: InterfaceId,
        provider: ComponentId,
        priority: i32,
    ) -> Self {
        self.interfaces
            .entry(interface)
            .or_default()
            .priorities
            .insert(provider, priority);
        self
    }

    #[must_use]
    pub fn with_explicit_binding(mut self, interface: InterfaceId, provider: ComponentId) -> Self {
        self.interfaces.entry(interface).or_default().explicit = Some(provider);
        self
    }

    #[must_use]
    pub fn with_disabled_provider(mut self, interface: InterfaceId, provider: ComponentId) -> Self {
        self.interfaces
            .entry(interface)
            .or_default()
            .disabled
            .insert(provider);
        self
    }

    fn apply(
        &self,
        manifests: impl IntoIterator<Item = ComponentManifest>,
    ) -> Vec<ComponentManifest> {
        manifests
            .into_iter()
            .map(|mut manifest| {
                let provider = manifest.id.clone();
                manifest.exports.retain_mut(|export| {
                    let policy = self.interfaces.get(&export.interface);
                    if policy.is_some_and(|policy| policy.disabled.contains(&provider)) {
                        return false;
                    }

                    let configured = policy
                        .and_then(|policy| policy.priorities.get(&provider))
                        .copied()
                        .unwrap_or_default();
                    export.priority = if policy
                        .and_then(|policy| policy.explicit.as_ref())
                        .is_some_and(|explicit| explicit == &provider)
                    {
                        i32::MAX
                    } else {
                        configured.min(i32::MAX - 1)
                    };
                    true
                });
                manifest
            })
            .collect()
    }
}

impl ResolvedComponentGraph {
    /// Resolve component imports with product-owned provider preference.
    pub fn compile_with_provider_policy(
        plugin_manifests: impl IntoIterator<Item = PluginManifest>,
        component_manifests: impl IntoIterator<Item = ComponentManifest>,
        authority_ceiling: &Authority,
        policy: &ProviderCompositionPolicy,
    ) -> Result<Self, ComponentGraphError> {
        Self::compile(
            plugin_manifests,
            policy.apply(component_manifests),
            authority_ceiling,
        )
    }
}

impl ResolvedHarness {
    /// Resolve a complete candidate generation with product-owned provider policy.
    ///
    /// The policy is included in generation identity even when two policies happen
    /// to select the same provider. Reconciliation therefore never mutates provider
    /// preference inside an active generation.
    pub fn resolve_with_provider_policy(
        plugin_manifests: impl IntoIterator<Item = PluginManifest>,
        component_manifests: impl IntoIterator<Item = ComponentManifest>,
        contributions: impl IntoIterator<Item = ConfigContribution>,
        authority_ceiling: &Authority,
        policy: &ProviderCompositionPolicy,
    ) -> Result<Self, ResolvedHarnessError> {
        let mut resolved = Self::resolve(
            plugin_manifests,
            policy.apply(component_manifests),
            contributions,
            authority_ceiling,
        )?;
        resolved.incorporate_semantic_metadata(policy);
        Ok(resolved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CapabilityId, ComponentExport, ComponentImport, InterfaceSchema, PluginExecution, PluginId,
    };

    fn plugin(value: &str, authority: Authority) -> PluginManifest {
        PluginManifest {
            id: PluginId::parse(value).unwrap(),
            version: 1,
            execution: PluginExecution::Embedded,
            dependencies: Vec::new(),
            services: Vec::new(),
            resource_namespaces: Vec::new(),
            maximum_authority: authority,
        }
    }

    fn provider(
        id: &str,
        interface: &InterfaceId,
        authored_priority: i32,
        required_authority: Authority,
        maximum_authority: Authority,
    ) -> ComponentManifest {
        ComponentManifest {
            id: ComponentId::parse(id).unwrap(),
            owner: PluginId::parse(format!("plugin-{id}")).unwrap(),
            imports: Vec::new(),
            exports: vec![ComponentExport {
                interface: interface.clone(),
                schema: InterfaceSchema::default(),
                priority: authored_priority,
                required_authority,
            }],
            listeners: Vec::new(),
            maximum_authority,
        }
    }

    fn consumer(interface: &InterfaceId, authority: Authority) -> ComponentManifest {
        ComponentManifest {
            id: ComponentId::parse("consumer").unwrap(),
            owner: PluginId::parse("plugin-consumer").unwrap(),
            imports: vec![ComponentImport {
                interface: interface.clone(),
                schema: InterfaceSchema::default(),
                required: true,
                authority: authority.clone(),
            }],
            exports: Vec::new(),
            listeners: Vec::new(),
            maximum_authority: authority,
        }
    }

    fn selected(graph: &ResolvedComponentGraph, interface: &InterfaceId) -> ComponentId {
        graph
            .import_handle(&ComponentId::parse("consumer").unwrap(), interface)
            .unwrap()
            .unwrap()
            .exporter()
            .clone()
    }

    #[test]
    fn product_policy_owns_effective_provider_priority() {
        let interface = InterfaceId::parse("fixture.provider@1").unwrap();
        let authority = Authority::default();
        let components = [
            provider(
                "a-provider",
                &interface,
                -1000,
                Authority::default(),
                authority.clone(),
            ),
            provider(
                "z-provider",
                &interface,
                1000,
                Authority::default(),
                authority.clone(),
            ),
            consumer(&interface, authority.clone()),
        ];
        let plugins = [
            plugin("plugin-a-provider", authority.clone()),
            plugin("plugin-z-provider", authority.clone()),
            plugin("plugin-consumer", authority.clone()),
        ];

        let default = ResolvedComponentGraph::compile_with_provider_policy(
            plugins.clone(),
            components.clone(),
            &authority,
            &ProviderCompositionPolicy::default(),
        )
        .unwrap();
        assert_eq!(
            selected(&default, &interface),
            ComponentId::parse("a-provider").unwrap()
        );

        let policy = ProviderCompositionPolicy::new().with_priority(
            interface.clone(),
            ComponentId::parse("z-provider").unwrap(),
            10,
        );
        let preferred = ResolvedComponentGraph::compile_with_provider_policy(
            plugins, components, &authority, &policy,
        )
        .unwrap();
        assert_eq!(
            selected(&preferred, &interface),
            ComponentId::parse("z-provider").unwrap()
        );
    }

    #[test]
    fn explicit_binding_cannot_bypass_authority() {
        let interface = InterfaceId::parse("fixture.provider@1").unwrap();
        let read = CapabilityId::parse("fs.read").unwrap();
        let network = CapabilityId::parse("network.read").unwrap();
        let caller = Authority::new([read.clone()]);
        let broad = Authority::new([read.clone(), network.clone()]);
        let components = [
            provider(
                "a-authorized",
                &interface,
                0,
                Authority::new([read]),
                broad.clone(),
            ),
            provider(
                "z-unauthorized",
                &interface,
                0,
                Authority::new([network]),
                broad.clone(),
            ),
            consumer(&interface, caller.clone()),
        ];
        let plugins = [
            plugin("plugin-a-authorized", broad.clone()),
            plugin("plugin-z-unauthorized", broad),
            plugin("plugin-consumer", caller.clone()),
        ];
        let policy = ProviderCompositionPolicy::new().with_explicit_binding(
            interface.clone(),
            ComponentId::parse("z-unauthorized").unwrap(),
        );

        let graph = ResolvedComponentGraph::compile_with_provider_policy(
            plugins, components, &caller, &policy,
        )
        .unwrap();
        assert_eq!(
            selected(&graph, &interface),
            ComponentId::parse("a-authorized").unwrap()
        );
    }

    #[test]
    fn disabled_provider_is_removed_before_selection() {
        let interface = InterfaceId::parse("fixture.provider@1").unwrap();
        let components = [
            provider(
                "a-provider",
                &interface,
                0,
                Authority::default(),
                Authority::default(),
            ),
            provider(
                "b-provider",
                &interface,
                0,
                Authority::default(),
                Authority::default(),
            ),
            consumer(&interface, Authority::default()),
        ];
        let plugins = [
            plugin("plugin-a-provider", Authority::default()),
            plugin("plugin-b-provider", Authority::default()),
            plugin("plugin-consumer", Authority::default()),
        ];
        let policy = ProviderCompositionPolicy::new()
            .with_disabled_provider(interface.clone(), ComponentId::parse("a-provider").unwrap());

        let graph = ResolvedComponentGraph::compile_with_provider_policy(
            plugins,
            components,
            &Authority::default(),
            &policy,
        )
        .unwrap();
        assert_eq!(
            selected(&graph, &interface),
            ComponentId::parse("b-provider").unwrap()
        );
    }

    #[test]
    fn provider_policy_is_part_of_generation_identity() {
        let interface = InterfaceId::parse("fixture.provider@1").unwrap();
        let components = [
            provider(
                "provider",
                &interface,
                500,
                Authority::default(),
                Authority::default(),
            ),
            consumer(&interface, Authority::default()),
        ];
        let plugins = [
            plugin("plugin-provider", Authority::default()),
            plugin("plugin-consumer", Authority::default()),
        ];
        let first = ResolvedHarness::resolve_with_provider_policy(
            plugins.clone(),
            components.clone(),
            [],
            &Authority::default(),
            &ProviderCompositionPolicy::default(),
        )
        .unwrap();
        let second_policy = ProviderCompositionPolicy::new().with_priority(
            interface,
            ComponentId::parse("provider").unwrap(),
            1,
        );
        let second = ResolvedHarness::resolve_with_provider_policy(
            plugins,
            components,
            [],
            &Authority::default(),
            &second_policy,
        )
        .unwrap();

        assert_ne!(first.generation(), second.generation());
    }
}
