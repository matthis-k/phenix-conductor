use crate::{
    Authority, ComponentExport, ComponentId, ComponentManifest, InterfaceCompatibility,
    InterfaceId, InterfaceSchemaMismatch, PluginExecution, PluginId, PluginManifest,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt::{self, Display, Formatter},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComponentGraphError {
    DuplicatePlugin(PluginId),
    DuplicateComponent(ComponentId),
    UnknownOwningPlugin {
        component: ComponentId,
        plugin: PluginId,
    },
    ResourceOnlyComponentOwner {
        component: ComponentId,
        plugin: PluginId,
    },
    DuplicateImport {
        component: ComponentId,
        interface: InterfaceId,
    },
    DuplicateExport {
        component: ComponentId,
        interface: InterfaceId,
    },
    MissingRequiredImport {
        component: ComponentId,
        interface: InterfaceId,
    },
    IncompatibleRequiredImport {
        component: ComponentId,
        interface: InterfaceId,
        exporter: ComponentId,
        mismatch: Box<InterfaceSchemaMismatch>,
    },
    ImportNotDeclared {
        component: ComponentId,
        interface: InterfaceId,
    },
    RequiredImportCycle {
        path: Vec<ComponentId>,
    },
    UnknownComponent(ComponentId),
}

impl Display for ComponentGraphError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicatePlugin(plugin) => write!(f, "duplicate plugin trust owner: {plugin}"),
            Self::DuplicateComponent(component) => write!(f, "duplicate component: {component}"),
            Self::UnknownOwningPlugin { component, plugin } => {
                write!(
                    f,
                    "component {component} has unknown owning plugin {plugin}"
                )
            }
            Self::ResourceOnlyComponentOwner { component, plugin } => write!(
                f,
                "executable component {component} cannot belong to resource-only plugin {plugin}"
            ),
            Self::DuplicateImport {
                component,
                interface,
            } => write!(
                f,
                "component {component} imports interface {interface} more than once"
            ),
            Self::DuplicateExport {
                component,
                interface,
            } => write!(
                f,
                "component {component} exports interface {interface} more than once"
            ),
            Self::MissingRequiredImport {
                component,
                interface,
            } => write!(
                f,
                "component {component} has unresolved required import {interface}"
            ),
            Self::IncompatibleRequiredImport {
                component,
                interface,
                exporter,
                mismatch,
            } => write!(
                f,
                "component {component} import {interface} is structurally incompatible with exporter {exporter}: {mismatch}"
            ),
            Self::ImportNotDeclared {
                component,
                interface,
            } => write!(
                f,
                "component {component} has no capability for undeclared import {interface}"
            ),
            Self::RequiredImportCycle { path } => {
                let rendered = path
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" -> ");
                write!(f, "required component import cycle: {rendered}")
            }
            Self::UnknownComponent(component) => write!(f, "unknown component: {component}"),
        }
    }
}

impl Error for ComponentGraphError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedImportHandle {
    importer: ComponentId,
    interface: InterfaceId,
    exporter: ComponentId,
    owning_plugin: PluginId,
    execution: PluginExecution,
    effective_authority: Authority,
}

impl ResolvedImportHandle {
    pub fn importer(&self) -> &ComponentId {
        &self.importer
    }

    pub fn interface(&self) -> &InterfaceId {
        &self.interface
    }

    pub fn exporter(&self) -> &ComponentId {
        &self.exporter
    }

    pub fn owning_plugin(&self) -> &PluginId {
        &self.owning_plugin
    }

    pub fn execution(&self) -> &PluginExecution {
        &self.execution
    }

    pub fn effective_authority(&self) -> &Authority {
        &self.effective_authority
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedImport {
    pub interface: InterfaceId,
    pub required: bool,
    pub binding: Option<ResolvedImportHandle>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedComponent {
    pub id: ComponentId,
    pub owning_plugin: PluginId,
    pub execution: PluginExecution,
    pub imports: Vec<ResolvedImport>,
    pub maximum_authority: Authority,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedComponentGraph {
    components: BTreeMap<ComponentId, ResolvedComponent>,
}

impl ResolvedComponentGraph {
    pub fn empty() -> Self {
        Self {
            components: BTreeMap::new(),
        }
    }

    pub fn compile(
        plugin_manifests: impl IntoIterator<Item = PluginManifest>,
        component_manifests: impl IntoIterator<Item = ComponentManifest>,
        harness_authority: &Authority,
    ) -> Result<Self, ComponentGraphError> {
        let mut plugins = BTreeMap::new();
        for manifest in plugin_manifests {
            let id = manifest.id.clone();
            if plugins.insert(id.clone(), manifest).is_some() {
                return Err(ComponentGraphError::DuplicatePlugin(id));
            }
        }

        let mut components = BTreeMap::new();
        for manifest in component_manifests {
            validate_component_manifest(&manifest)?;
            let owner = plugins.get(&manifest.owner).ok_or_else(|| {
                ComponentGraphError::UnknownOwningPlugin {
                    component: manifest.id.clone(),
                    plugin: manifest.owner.clone(),
                }
            })?;
            if matches!(owner.execution, PluginExecution::ResourceOnly) {
                return Err(ComponentGraphError::ResourceOnlyComponentOwner {
                    component: manifest.id.clone(),
                    plugin: manifest.owner.clone(),
                });
            }
            let id = manifest.id.clone();
            if components.insert(id.clone(), manifest).is_some() {
                return Err(ComponentGraphError::DuplicateComponent(id));
            }
        }

        let mut exporters: BTreeMap<InterfaceId, Vec<(&ComponentManifest, &ComponentExport)>> =
            BTreeMap::new();
        for manifest in components.values() {
            for export in &manifest.exports {
                exporters
                    .entry(export.interface.clone())
                    .or_default()
                    .push((manifest, export));
            }
        }
        for candidates in exporters.values_mut() {
            candidates.sort_by(|(left, left_export), (right, right_export)| {
                right_export
                    .priority
                    .cmp(&left_export.priority)
                    .then_with(|| left.id.cmp(&right.id))
            });
        }

        let mut resolved = BTreeMap::new();
        for manifest in components.values() {
            let owner = &plugins[&manifest.owner];
            let component_authority = harness_authority
                .attenuate(&owner.maximum_authority)
                .attenuate(&manifest.maximum_authority);
            let mut imports = Vec::with_capacity(manifest.imports.len());
            for import in &manifest.imports {
                let mut selected = None;
                let mut incompatible = None;
                if let Some(candidates) = exporters.get(&import.interface) {
                    for (candidate, export) in candidates {
                        let exporter_owner = &plugins[&candidate.owner];
                        let effective_authority = component_authority
                            .attenuate(&import.authority)
                            .attenuate(&exporter_owner.maximum_authority)
                            .attenuate(&candidate.maximum_authority);
                        if !effective_authority.permits_all(&export.required_authority) {
                            continue;
                        }
                        match import.schema.accepts_provider(&export.schema) {
                            InterfaceCompatibility::Exact | InterfaceCompatibility::Compatible => {
                                selected = Some((*candidate, effective_authority));
                                break;
                            }
                            InterfaceCompatibility::Incompatible(mismatch) => {
                                incompatible
                                    .get_or_insert_with(|| (candidate.id.clone(), mismatch));
                            }
                        }
                    }
                }
                let binding = if let Some((exporter, effective_authority)) = selected {
                    let exporter_owner = &plugins[&exporter.owner];
                    Some(ResolvedImportHandle {
                        importer: manifest.id.clone(),
                        interface: import.interface.clone(),
                        exporter: exporter.id.clone(),
                        owning_plugin: exporter.owner.clone(),
                        execution: exporter_owner.execution.clone(),
                        effective_authority,
                    })
                } else if import.required {
                    if let Some((exporter, mismatch)) = incompatible {
                        return Err(ComponentGraphError::IncompatibleRequiredImport {
                            component: manifest.id.clone(),
                            interface: import.interface.clone(),
                            exporter,
                            mismatch: Box::new(mismatch),
                        });
                    }
                    return Err(ComponentGraphError::MissingRequiredImport {
                        component: manifest.id.clone(),
                        interface: import.interface.clone(),
                    });
                } else {
                    None
                };
                imports.push(ResolvedImport {
                    interface: import.interface.clone(),
                    required: import.required,
                    binding,
                });
            }
            resolved.insert(
                manifest.id.clone(),
                ResolvedComponent {
                    id: manifest.id.clone(),
                    owning_plugin: manifest.owner.clone(),
                    execution: owner.execution.clone(),
                    imports,
                    maximum_authority: component_authority,
                },
            );
        }

        validate_required_import_dag(&resolved)?;

        Ok(Self {
            components: resolved,
        })
    }

    pub fn components(&self) -> impl Iterator<Item = &ResolvedComponent> {
        self.components.values()
    }

    pub fn component(&self, component: &ComponentId) -> Option<&ResolvedComponent> {
        self.components.get(component)
    }

    pub fn import_handle(
        &self,
        component: &ComponentId,
        interface: &InterfaceId,
    ) -> Result<Option<&ResolvedImportHandle>, ComponentGraphError> {
        let resolved = self
            .components
            .get(component)
            .ok_or_else(|| ComponentGraphError::UnknownComponent(component.clone()))?;
        let import = resolved
            .imports
            .iter()
            .find(|import| &import.interface == interface)
            .ok_or_else(|| ComponentGraphError::ImportNotDeclared {
                component: component.clone(),
                interface: interface.clone(),
            })?;
        Ok(import.binding.as_ref())
    }
}

fn validate_required_import_dag(
    components: &BTreeMap<ComponentId, ResolvedComponent>,
) -> Result<(), ComponentGraphError> {
    fn visit(
        component: &ComponentId,
        components: &BTreeMap<ComponentId, ResolvedComponent>,
        visiting: &mut Vec<ComponentId>,
        visited: &mut BTreeSet<ComponentId>,
    ) -> Result<(), ComponentGraphError> {
        if let Some(start) = visiting.iter().position(|candidate| candidate == component) {
            let mut path = visiting[start..].to_vec();
            path.push(component.clone());
            return Err(ComponentGraphError::RequiredImportCycle { path });
        }
        if visited.contains(component) {
            return Ok(());
        }

        visiting.push(component.clone());
        if let Some(resolved) = components.get(component) {
            for import in &resolved.imports {
                if !import.required {
                    continue;
                }
                if let Some(binding) = &import.binding {
                    visit(binding.exporter(), components, visiting, visited)?;
                }
            }
        }
        visiting.pop();
        visited.insert(component.clone());
        Ok(())
    }

    let mut visiting = Vec::new();
    let mut visited = BTreeSet::new();
    for component in components.keys() {
        visit(component, components, &mut visiting, &mut visited)?;
    }
    Ok(())
}

fn validate_component_manifest(manifest: &ComponentManifest) -> Result<(), ComponentGraphError> {
    let mut imports = BTreeSet::new();
    for import in &manifest.imports {
        if !imports.insert(import.interface.clone()) {
            return Err(ComponentGraphError::DuplicateImport {
                component: manifest.id.clone(),
                interface: import.interface.clone(),
            });
        }
    }
    let mut exports = BTreeSet::new();
    for export in &manifest.exports {
        if !exports.insert(export.interface.clone()) {
            return Err(ComponentGraphError::DuplicateExport {
                component: manifest.id.clone(),
                interface: export.interface.clone(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CapabilityId, ComponentExport, ComponentImport};

    fn component(value: &str) -> ComponentId {
        ComponentId::parse(value).unwrap()
    }

    fn interface(value: &str) -> InterfaceId {
        InterfaceId::parse(value).unwrap()
    }

    fn plugin(value: &str) -> PluginId {
        PluginId::parse(value).unwrap()
    }

    fn capability(value: &str) -> CapabilityId {
        CapabilityId::parse(value).unwrap()
    }

    fn plugin_manifest(id: &str, authority: Authority) -> PluginManifest {
        PluginManifest {
            id: plugin(id),
            version: 1,
            execution: PluginExecution::Embedded,
            dependencies: Vec::new(),
            services: Vec::new(),
            resource_namespaces: Vec::new(),
            maximum_authority: authority,
        }
    }

    fn exporter(id: &str, priority: i32, authority: Authority) -> ComponentManifest {
        ComponentManifest {
            id: component(id),
            owner: plugin(&format!("plugin-{id}")),
            imports: Vec::new(),
            exports: vec![ComponentExport {
                interface: interface("phenix.demo@1"),
                schema: Default::default(),
                priority,
                required_authority: Authority::default(),
            }],
            maximum_authority: authority,
        }
    }

    fn importer(required: bool, authority: Authority) -> ComponentManifest {
        ComponentManifest {
            id: component("consumer"),
            owner: plugin("plugin-consumer"),
            imports: vec![ComponentImport {
                interface: interface("phenix.demo@1"),
                schema: Default::default(),
                required,
                authority: authority.clone(),
            }],
            exports: Vec::new(),
            maximum_authority: authority,
        }
    }

    fn owners(authority: Authority, exporters: &[&str]) -> Vec<PluginManifest> {
        let mut plugins = vec![plugin_manifest("plugin-consumer", authority.clone())];
        plugins.extend(
            exporters
                .iter()
                .map(|id| plugin_manifest(&format!("plugin-{id}"), authority.clone())),
        );
        plugins
    }

    #[test]
    fn required_import_must_resolve_before_component_use() {
        let error = ResolvedComponentGraph::compile(
            owners(Authority::default(), &[]),
            [importer(true, Authority::default())],
            &Authority::default(),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            ComponentGraphError::MissingRequiredImport { .. }
        ));
    }

    #[test]
    fn provider_selection_is_deterministic_and_registration_order_independent() {
        let authority = Authority::new([capability("demo.use")]);
        let first = ResolvedComponentGraph::compile(
            owners(authority.clone(), &["z-provider", "a-provider"]),
            [
                exporter("z-provider", 10, authority.clone()),
                importer(true, authority.clone()),
                exporter("a-provider", 10, authority.clone()),
            ],
            &authority,
        )
        .unwrap();
        let second = ResolvedComponentGraph::compile(
            owners(authority.clone(), &["a-provider", "z-provider"]),
            [
                exporter("a-provider", 10, authority.clone()),
                exporter("z-provider", 10, authority.clone()),
                importer(true, authority.clone()),
            ],
            &authority,
        )
        .unwrap();
        let selected = |graph: &ResolvedComponentGraph| {
            graph
                .import_handle(&component("consumer"), &interface("phenix.demo@1"))
                .unwrap()
                .unwrap()
                .exporter()
                .clone()
        };
        assert_eq!(selected(&first), component("a-provider"));
        assert_eq!(selected(&first), selected(&second));
    }

    #[test]
    fn provider_selection_skips_exports_whose_required_authority_cannot_be_granted() {
        let read = capability("fs.read");
        let network = capability("network.read");
        let importer_authority = Authority::new([read.clone()]);
        let provider_authority = Authority::new([read.clone(), network.clone()]);
        let mut high = exporter("high-provider", 20, provider_authority.clone());
        high.exports[0].required_authority = Authority::new([network]);
        let mut low = exporter("low-provider", 10, provider_authority.clone());
        low.exports[0].required_authority = Authority::new([read.clone()]);
        let graph = ResolvedComponentGraph::compile(
            vec![
                plugin_manifest("plugin-consumer", importer_authority.clone()),
                plugin_manifest("plugin-high-provider", provider_authority.clone()),
                plugin_manifest("plugin-low-provider", provider_authority),
            ],
            [high, low, importer(true, importer_authority.clone())],
            &importer_authority,
        )
        .unwrap();

        let handle = graph
            .import_handle(&component("consumer"), &interface("phenix.demo@1"))
            .unwrap()
            .unwrap();
        assert_eq!(handle.exporter(), &component("low-provider"));
        assert!(handle.effective_authority().permits(&read));
    }

    #[test]
    fn required_import_with_only_authority_incompatible_exports_fails_before_activation() {
        let read = capability("fs.read");
        let network = capability("network.read");
        let importer_authority = Authority::new([read]);
        let provider_authority = Authority::new([network.clone()]);
        let mut provider = exporter("provider", 10, provider_authority.clone());
        provider.exports[0].required_authority = provider_authority.clone();
        let graph = ResolvedComponentGraph::compile(
            vec![
                plugin_manifest("plugin-consumer", importer_authority.clone()),
                plugin_manifest("plugin-provider", provider_authority),
            ],
            [provider, importer(true, importer_authority.clone())],
            &Authority::new([capability("fs.read"), network]),
        );

        assert!(matches!(
            graph,
            Err(ComponentGraphError::MissingRequiredImport {
                component: missing_component,
                interface: missing_interface,
            }) if missing_component == component("consumer")
                && missing_interface == interface("phenix.demo@1")
        ));
    }

    #[test]
    fn binding_keeps_granted_authority_beyond_the_export_minimum() {
        let read = capability("fs.read");
        let authority = Authority::new([read.clone()]);
        let graph = ResolvedComponentGraph::compile(
            owners(authority.clone(), &["provider"]),
            [
                exporter("provider", 10, authority.clone()),
                importer(true, authority.clone()),
            ],
            &authority,
        )
        .unwrap();

        let handle = graph
            .import_handle(&component("consumer"), &interface("phenix.demo@1"))
            .unwrap()
            .unwrap();
        assert!(handle.effective_authority().permits(&read));
    }

    #[test]
    fn import_handle_authority_is_attenuated_at_plugin_and_component_boundaries() {
        let read = capability("fs.read");
        let write = capability("fs.write");
        let network = capability("network.read");
        let plugins = vec![
            plugin_manifest(
                "plugin-consumer",
                Authority::new([read.clone(), write.clone()]),
            ),
            plugin_manifest(
                "plugin-provider",
                Authority::new([read.clone(), network.clone()]),
            ),
        ];
        let graph = ResolvedComponentGraph::compile(
            plugins,
            [
                exporter("provider", 1, Authority::new([read.clone()])),
                importer(true, Authority::new([read.clone(), write.clone()])),
            ],
            &Authority::new([read.clone(), write, network]),
        )
        .unwrap();
        let handle = graph
            .import_handle(&component("consumer"), &interface("phenix.demo@1"))
            .unwrap()
            .unwrap();
        assert!(handle.effective_authority().permits(&read));
        assert!(!handle
            .effective_authority()
            .permits(&capability("fs.write")));
        assert!(!handle
            .effective_authority()
            .permits(&capability("network.read")));
    }

    #[test]
    fn optional_and_undeclared_imports_are_distinct() {
        let optional = ResolvedComponentGraph::compile(
            owners(Authority::default(), &[]),
            [importer(false, Authority::default())],
            &Authority::default(),
        )
        .unwrap();
        assert!(optional
            .import_handle(&component("consumer"), &interface("phenix.demo@1"))
            .unwrap()
            .is_none());
        assert!(matches!(
            optional.import_handle(&component("consumer"), &interface("other@1")),
            Err(ComponentGraphError::ImportNotDeclared { .. })
        ));
    }

    #[test]
    fn required_import_cycles_report_the_concrete_component_path() {
        let authority = Authority::default();
        let interface_a = interface("phenix.a@1");
        let interface_b = interface("phenix.b@1");
        let component_a = ComponentManifest {
            id: component("component-a"),
            owner: plugin("plugin-a"),
            imports: vec![ComponentImport {
                interface: interface_b.clone(),
                schema: Default::default(),
                required: true,
                authority: authority.clone(),
            }],
            exports: vec![ComponentExport {
                interface: interface_a.clone(),
                schema: Default::default(),
                priority: 0,
                required_authority: authority.clone(),
            }],
            maximum_authority: authority.clone(),
        };
        let component_b = ComponentManifest {
            id: component("component-b"),
            owner: plugin("plugin-b"),
            imports: vec![ComponentImport {
                interface: interface_a,
                schema: Default::default(),
                required: true,
                authority: authority.clone(),
            }],
            exports: vec![ComponentExport {
                interface: interface_b,
                schema: Default::default(),
                priority: 0,
                required_authority: authority.clone(),
            }],
            maximum_authority: authority.clone(),
        };

        let error = ResolvedComponentGraph::compile(
            [
                plugin_manifest("plugin-a", authority.clone()),
                plugin_manifest("plugin-b", authority.clone()),
            ],
            [component_a, component_b],
            &authority,
        )
        .unwrap_err();

        assert_eq!(
            error,
            ComponentGraphError::RequiredImportCycle {
                path: vec![
                    component("component-a"),
                    component("component-b"),
                    component("component-a"),
                ],
            }
        );
        assert_eq!(
            error.to_string(),
            "required component import cycle: component-a -> component-b -> component-a"
        );
    }

    #[test]
    fn component_owner_is_a_real_plugin_trust_boundary() {
        let component = ComponentManifest {
            id: component("orphan"),
            owner: plugin("missing-plugin"),
            imports: Vec::new(),
            exports: Vec::new(),
            maximum_authority: Authority::default(),
        };
        assert!(matches!(
            ResolvedComponentGraph::compile([], [component], &Authority::default()),
            Err(ComponentGraphError::UnknownOwningPlugin { .. })
        ));
    }
}

#[cfg(test)]
mod interface_schema_binding_tests {
    use super::*;
    use crate::{InterfaceSchema, Key, PhenixSchema};

    fn key(value: &str) -> Key {
        Key::parse(value).unwrap()
    }

    fn table(fields: &[(&str, PhenixSchema)]) -> PhenixSchema {
        PhenixSchema::Table(
            fields
                .iter()
                .map(|(name, schema)| (key(name), schema.clone()))
                .collect(),
        )
    }

    fn plugin(id: &str) -> PluginManifest {
        PluginManifest {
            id: PluginId::parse(id).unwrap(),
            version: 1,
            execution: PluginExecution::Embedded,
            dependencies: Vec::new(),
            services: Vec::new(),
            resource_namespaces: Vec::new(),
            maximum_authority: Authority::default(),
        }
    }

    fn interface() -> InterfaceId {
        InterfaceId::parse("fixture.schema@1").unwrap()
    }

    fn consumer(schema: InterfaceSchema) -> ComponentManifest {
        ComponentManifest {
            id: ComponentId::parse("consumer").unwrap(),
            owner: PluginId::parse("consumer-owner").unwrap(),
            imports: vec![crate::ComponentImport {
                interface: interface(),
                schema,
                required: true,
                authority: Authority::default(),
            }],
            exports: Vec::new(),
            maximum_authority: Authority::default(),
        }
    }

    fn provider(id: &str, priority: i32, schema: InterfaceSchema) -> ComponentManifest {
        ComponentManifest {
            id: ComponentId::parse(id).unwrap(),
            owner: PluginId::parse(format!("{id}-owner")).unwrap(),
            imports: Vec::new(),
            exports: vec![crate::ComponentExport {
                interface: interface(),
                schema,
                priority,
                required_authority: Authority::default(),
            }],
            maximum_authority: Authority::default(),
        }
    }

    #[test]
    fn graph_accepts_directionally_compatible_independent_schemas() {
        let consumer_schema = InterfaceSchema::new(
            table(&[
                ("name", PhenixSchema::String),
                ("detail", PhenixSchema::U64),
            ]),
            table(&[("value", PhenixSchema::String)]),
        );
        let provider_schema = InterfaceSchema::new(
            table(&[("name", PhenixSchema::String)]),
            table(&[
                ("value", PhenixSchema::String),
                ("internal", PhenixSchema::U64),
            ]),
        );
        let graph = ResolvedComponentGraph::compile(
            [plugin("consumer-owner"), plugin("provider-owner")],
            [
                consumer(consumer_schema),
                provider("provider", 10, provider_schema),
            ],
            &Authority::default(),
        )
        .unwrap();

        assert_eq!(
            graph
                .import_handle(&ComponentId::parse("consumer").unwrap(), &interface())
                .unwrap()
                .unwrap()
                .exporter(),
            &ComponentId::parse("provider").unwrap()
        );
    }

    #[test]
    fn graph_skips_incompatible_provider_for_lower_priority_compatible_provider() {
        let consumer_schema = InterfaceSchema::new(
            table(&[("name", PhenixSchema::String)]),
            table(&[("value", PhenixSchema::String)]),
        );
        let incompatible = InterfaceSchema::new(
            table(&[("name", PhenixSchema::String)]),
            table(&[("wrong", PhenixSchema::String)]),
        );
        let compatible = consumer_schema.clone();
        let graph = ResolvedComponentGraph::compile(
            [
                plugin("consumer-owner"),
                plugin("high-owner"),
                plugin("low-owner"),
            ],
            [
                consumer(consumer_schema),
                provider("high", 100, incompatible),
                provider("low", 10, compatible),
            ],
            &Authority::default(),
        )
        .unwrap();

        assert_eq!(
            graph
                .import_handle(&ComponentId::parse("consumer").unwrap(), &interface())
                .unwrap()
                .unwrap()
                .exporter(),
            &ComponentId::parse("low").unwrap()
        );
    }

    #[test]
    fn required_import_reports_structural_incompatibility_before_activation() {
        let consumer_schema = InterfaceSchema::new(
            table(&[("name", PhenixSchema::String)]),
            table(&[("value", PhenixSchema::String)]),
        );
        let provider_schema = InterfaceSchema::new(
            table(&[("name", PhenixSchema::String)]),
            table(&[("wrong", PhenixSchema::String)]),
        );
        let error = ResolvedComponentGraph::compile(
            [plugin("consumer-owner"), plugin("provider-owner")],
            [
                consumer(consumer_schema),
                provider("provider", 10, provider_schema),
            ],
            &Authority::default(),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            ComponentGraphError::IncompatibleRequiredImport { exporter, .. }
                if exporter == ComponentId::parse("provider").unwrap()
        ));
    }
}
