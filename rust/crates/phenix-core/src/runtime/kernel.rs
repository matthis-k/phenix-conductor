use super::{
    dispatch::{invoke_component_service_with, invoke_service_with, ServiceDispatchGuards},
    *,
};

impl Kernel {
    pub fn new(config: KernelConfig) -> Self {
        Self::with_persistence(
            config,
            LocalPersistence::open_in_memory().expect("baseline local persistence opens"),
        )
    }

    pub fn with_persistence(
        config: KernelConfig,
        persistence: impl PersistenceBackend + 'static,
    ) -> Self {
        let states = config
            .manifests()
            .map(|manifest| (manifest.id.clone(), PluginState::Registered))
            .collect();
        Self {
            graph_generation: None,
            component_graph: ResolvedComponentGraph::empty(),
            active_resources: Vec::new(),
            config,
            states,
            embedded_factories: BTreeMap::new(),
            external_factories: BTreeMap::new(),
            instances: BTreeMap::new(),
            events: Arc::new(EventBus::default()),
            tasks: TaskRuntime::default(),
            persistence: Mutex::new(Box::new(persistence)),
            provenance: Mutex::new(Vec::new()),
            runtime_active: false,
        }
    }

    pub fn kernel_only() -> Self {
        Self::new(KernelConfig::empty())
    }

    pub fn config(&self) -> &KernelConfig {
        &self.config
    }

    pub fn graph_generation(&self) -> Option<&GraphGenerationId> {
        self.graph_generation.as_ref()
    }

    pub(crate) fn install_resolved_graph(
        &mut self,
        generation: GraphGenerationId,
        graph: ResolvedComponentGraph,
        resources: Vec<SkillResourceMetadata>,
    ) {
        self.graph_generation = Some(generation);
        self.component_graph = graph;
        self.active_resources = resources;
    }

    pub fn component_graph(&self) -> &ResolvedComponentGraph {
        &self.component_graph
    }

    pub fn active_resources(&self) -> &[SkillResourceMetadata] {
        &self.active_resources
    }

    pub fn events(&self) -> Arc<EventBus> {
        Arc::clone(&self.events)
    }

    pub fn tasks(&self) -> &TaskRuntime {
        &self.tasks
    }

    pub fn service_invocation_provenance(&self) -> Vec<ServiceInvocationProvenance> {
        self.provenance
            .lock()
            .expect("service provenance mutex poisoned")
            .clone()
    }

    pub fn state(&self, plugin: &PluginId) -> Option<PluginState> {
        self.states.get(plugin).copied()
    }

    pub fn register_embedded_factory<F>(
        &mut self,
        plugin: PluginId,
        factory: F,
    ) -> Result<(), KernelError>
    where
        F: Fn() -> Box<dyn PluginInstance> + Send + Sync + 'static,
    {
        let manifest = self
            .config
            .manifest(&plugin)
            .ok_or_else(|| KernelError::UnknownPlugin(plugin.clone()))?;
        if !matches!(manifest.execution, PluginExecution::Embedded) {
            return Err(KernelError::WrongExecutionKind(plugin));
        }
        self.preload_embedded_factory(plugin, factory);
        Ok(())
    }

    /// Preload an embedded implementation for a plugin that may enter a future graph generation.
    /// This grants no authority and does not mutate the active composition.
    pub fn preload_embedded_factory<F>(&mut self, plugin: PluginId, factory: F)
    where
        F: Fn() -> Box<dyn PluginInstance> + Send + Sync + 'static,
    {
        self.embedded_factories.insert(plugin, Arc::new(factory));
    }

    pub fn register_external_factory<F>(
        &mut self,
        plugin: PluginId,
        factory: F,
    ) -> Result<(), KernelError>
    where
        F: Fn(&PluginManifest) -> Result<Box<dyn PluginInstance>, String> + Send + Sync + 'static,
    {
        let manifest = self
            .config
            .manifest(&plugin)
            .ok_or_else(|| KernelError::UnknownPlugin(plugin.clone()))?;
        if !matches!(manifest.execution, PluginExecution::External { .. }) {
            return Err(KernelError::WrongExecutionKind(plugin));
        }
        self.preload_external_factory(plugin, factory);
        Ok(())
    }

    /// Preload an external-host implementation for a plugin that may enter a future graph generation.
    pub fn preload_external_factory<F>(&mut self, plugin: PluginId, factory: F)
    where
        F: Fn(&PluginManifest) -> Result<Box<dyn PluginInstance>, String> + Send + Sync + 'static,
    {
        self.external_factories.insert(plugin, Arc::new(factory));
    }

    pub fn activate_all(&mut self) -> Result<(), KernelError> {
        for plugin in self.config.activation_order().to_vec() {
            self.activate(&plugin)?;
        }
        self.runtime_active = true;
        Ok(())
    }

    fn activate(&mut self, plugin: &PluginId) -> Result<(), KernelError> {
        if self.state(plugin) == Some(PluginState::Active) {
            return Ok(());
        }
        let manifest = self
            .config
            .manifest(plugin)
            .ok_or_else(|| KernelError::UnknownPlugin(plugin.clone()))?
            .clone();

        match manifest.execution {
            PluginExecution::ResourceOnly => {}
            PluginExecution::Embedded => {
                let factory = self
                    .embedded_factories
                    .get(plugin)
                    .ok_or_else(|| KernelError::EmbeddedFactoryMissing(plugin.clone()))?;
                let instance = factory();
                self.start_instance(plugin, &manifest, instance)?;
            }
            PluginExecution::External { .. } => {
                let factory = self
                    .external_factories
                    .get(plugin)
                    .ok_or_else(|| KernelError::ExternalHostUnavailable(plugin.clone()))?;
                let instance = factory(&manifest).map_err(|message| KernelError::PluginStart {
                    plugin: plugin.clone(),
                    message,
                })?;
                self.start_instance(plugin, &manifest, instance)?;
            }
        }

        self.states.insert(plugin.clone(), PluginState::Active);
        self.events
            .publish(KernelEvent::PluginActivated(plugin.clone()));
        Ok(())
    }

    fn start_instance(
        &mut self,
        plugin: &PluginId,
        manifest: &PluginManifest,
        mut instance: Box<dyn PluginInstance>,
    ) -> Result<(), KernelError> {
        let host = PluginHost {
            graph_generation: self.graph_generation.as_ref(),
            component_graph: &self.component_graph,
            config: &self.config,
            states: &self.states,
            instances: &self.instances,
            plugin,
            authority: &manifest.maximum_authority,
            call_stack: BTreeSet::from([plugin.clone()]),
            events: &self.events,
            tasks: &self.tasks,
            persistence: &self.persistence,
            provenance: &self.provenance,
            continuation: None,
            active_services: BTreeSet::new(),
        };
        instance
            .start(&host)
            .map_err(|message| KernelError::PluginStart {
                plugin: plugin.clone(),
                message,
            })?;
        self.instances
            .insert(plugin.clone(), Arc::new(Mutex::new(instance)));
        Ok(())
    }

    pub fn invoke_component(
        &mut self,
        component: &ComponentId,
        service: &ServiceId,
        input: &[u8],
        caller_authority: &Authority,
        binding: &PluginId,
    ) -> Result<Vec<u8>, KernelError> {
        invoke_component_service_with(
            InvocationContext {
                graph_generation: self.graph_generation.as_ref(),
                component_graph: &self.component_graph,
                config: &self.config,
                states: &self.states,
                instances: &self.instances,
                events: &self.events,
                tasks: &self.tasks,
                persistence: &self.persistence,
                provenance: &self.provenance,
            },
            service,
            component,
            input,
            caller_authority,
            binding,
            ServiceDispatchGuards {
                call_stack: &BTreeSet::new(),
                active_services: &BTreeSet::new(),
                terminal_component: Some(component),
            },
        )
    }

    pub fn invoke(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        caller_authority: &Authority,
        binding: Option<&PluginId>,
    ) -> Result<Vec<u8>, KernelError> {
        invoke_service_with(
            InvocationContext {
                graph_generation: self.graph_generation.as_ref(),
                component_graph: &self.component_graph,
                config: &self.config,
                states: &self.states,
                instances: &self.instances,
                events: &self.events,
                tasks: &self.tasks,
                persistence: &self.persistence,
                provenance: &self.provenance,
            },
            service,
            input,
            caller_authority,
            binding,
            &BTreeSet::new(),
            &BTreeSet::new(),
        )
    }

    pub fn stop(&mut self, plugin: &PluginId) -> Result<(), KernelError> {
        if let Some(instance) = self.instances.get(plugin) {
            instance
                .lock()
                .expect("plugin instance mutex poisoned")
                .stop()
                .map_err(|message| KernelError::PluginStop {
                    plugin: plugin.clone(),
                    message,
                })?;
        }
        self.instances.remove(plugin);
        let state = self
            .states
            .get_mut(plugin)
            .ok_or_else(|| KernelError::UnknownPlugin(plugin.clone()))?;
        *state = PluginState::Stopped;
        self.events
            .publish(KernelEvent::PluginStopped(plugin.clone()));
        Ok(())
    }
}
