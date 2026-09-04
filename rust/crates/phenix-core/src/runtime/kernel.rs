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
            prepared_embedded_instances: BTreeMap::new(),
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

    /// Preload one already-constructed embedded implementation for the next activation.
    /// Stateful plugins with real construction inputs use this path instead of pretending to have
    /// a reusable zero-argument factory.
    pub fn preload_embedded_instance(
        &mut self,
        plugin: PluginId,
        instance: Box<dyn PluginInstance>,
    ) {
        self.prepared_embedded_instances.insert(plugin, instance);
    }

    pub(super) fn take_embedded_instance(
        &mut self,
        plugin: &PluginId,
    ) -> Result<Box<dyn PluginInstance>, KernelError> {
        if let Some(instance) = self.prepared_embedded_instances.remove(plugin) {
            return Ok(instance);
        }
        self.embedded_factories
            .get(plugin)
            .map(|factory| factory())
            .ok_or_else(|| KernelError::EmbeddedFactoryMissing(plugin.clone()))
    }

    pub fn activate_all(&mut self) -> Result<(), KernelError> {
        if self.runtime_active
            && self
                .states
                .values()
                .all(|state| *state == PluginState::Active)
        {
            return Ok(());
        }
        let config = self.config.clone();
        let mut next_states = if self.runtime_active {
            self.states.clone()
        } else {
            config
                .manifests()
                .map(|manifest| (manifest.id.clone(), PluginState::Registered))
                .collect()
        };
        let mut next_instances: BTreeMap<PluginId, Arc<Mutex<Box<dyn PluginInstance>>>> =
            if self.runtime_active {
                self.instances.clone()
            } else {
                BTreeMap::new()
            };
        let mut staged = Vec::new();

        for plugin in config.activation_order() {
            if next_states.get(plugin) == Some(&PluginState::Active) {
                continue;
            }
            let manifest = config
                .manifest(plugin)
                .expect("activation order only contains configured plugins");
            let instance = (|| -> Result<Option<Box<dyn PluginInstance>>, KernelError> {
                match &manifest.execution {
                    PluginExecution::ResourceOnly => Ok(None),
                    PluginExecution::Embedded => self.take_embedded_instance(plugin).map(Some),
                    PluginExecution::Runtime { runtime, artifact } => {
                        let binding = config.runtime_binding(plugin).cloned().ok_or_else(|| {
                            KernelError::RuntimeProviderUnavailable(runtime.clone())
                        })?;
                        let provider =
                            next_instances
                                .get(&binding.provider)
                                .cloned()
                                .ok_or_else(|| {
                                    KernelError::PluginNotActive(binding.provider.clone())
                                })?;
                        let mut provider = provider.lock().expect("plugin instance mutex poisoned");
                        let contract = provider.runtime_provider().ok_or_else(|| {
                            KernelError::RuntimeProviderContractUnavailable {
                                runtime: runtime.clone(),
                                provider: binding.provider.clone(),
                            }
                        })?;
                        contract
                            .prepare(RuntimePluginCandidate {
                                manifest,
                                artifact,
                                guest_authority: &manifest.maximum_authority,
                            })
                            .map(Some)
                            .map_err(|message| KernelError::RuntimePrepare {
                                plugin: plugin.clone(),
                                runtime: runtime.clone(),
                                message,
                            })
                    }
                }
            })();
            let instance = match instance {
                Ok(instance) => instance,
                Err(error) => {
                    reconciliation::cleanup_staged(
                        &staged,
                        reconciliation::StopView {
                            generation: self.graph_generation.as_ref(),
                            graph: &self.component_graph,
                            config: &config,
                            states: &next_states,
                            instances: &next_instances,
                            events: &self.events,
                            tasks: &self.tasks,
                            persistence: &self.persistence,
                            provenance: &self.provenance,
                        },
                    );
                    return Err(error);
                }
            };
            if let Some(mut instance) = instance {
                let host = PluginHost {
                    graph_generation: self.graph_generation.as_ref(),
                    component_graph: &self.component_graph,
                    config: &config,
                    states: &next_states,
                    instances: &next_instances,
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
                if let Err(message) = instance.start(&host) {
                    reconciliation::cleanup_staged(
                        &staged,
                        reconciliation::StopView {
                            generation: self.graph_generation.as_ref(),
                            graph: &self.component_graph,
                            config: &config,
                            states: &next_states,
                            instances: &next_instances,
                            events: &self.events,
                            tasks: &self.tasks,
                            persistence: &self.persistence,
                            provenance: &self.provenance,
                        },
                    );
                    return Err(KernelError::PluginStart {
                        plugin: plugin.clone(),
                        message,
                    });
                }
                next_instances.insert(plugin.clone(), Arc::new(Mutex::new(instance)));
            }
            next_states.insert(plugin.clone(), PluginState::Active);
            staged.push(plugin.clone());
        }

        let subscriptions = match self.graph_generation.as_ref() {
            Some(generation) => stage_listener_subscriptions(
                &self.component_graph,
                generation,
                &config,
                &next_instances,
            ),
            None if self.component_graph.listeners().next().is_none() => Ok(Vec::new()),
            None => Err(KernelError::ResolvedGenerationMissing),
        };
        let subscriptions = match subscriptions {
            Ok(subscriptions) => subscriptions,
            Err(error) => {
                reconciliation::cleanup_staged(
                    &staged,
                    reconciliation::StopView {
                        generation: self.graph_generation.as_ref(),
                        graph: &self.component_graph,
                        config: &config,
                        states: &next_states,
                        instances: &next_instances,
                        events: &self.events,
                        tasks: &self.tasks,
                        persistence: &self.persistence,
                        provenance: &self.provenance,
                    },
                );
                return Err(error);
            }
        };

        self.events.replace_subscriptions(subscriptions)?;
        self.states = next_states;
        self.instances = next_instances;
        self.runtime_active = true;
        for plugin in staged {
            self.events.publish(KernelEvent::PluginActivated(plugin));
        }
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
            None,
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
        let manifest = self
            .config
            .manifest(plugin)
            .ok_or_else(|| KernelError::UnknownPlugin(plugin.clone()))?;
        self.tasks.cancel_plugin(plugin);
        if let Some(instance) = self.instances.get(plugin) {
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
                .lock()
                .expect("plugin instance mutex poisoned")
                .stop(&host)
                .map_err(|message| KernelError::PluginStop {
                    plugin: plugin.clone(),
                    message,
                })?;
        }
        self.instances.remove(plugin);
        let state = self
            .states
            .get_mut(plugin)
            .expect("plugin manifest and lifecycle state stay aligned");
        *state = PluginState::Stopped;
        self.events
            .publish(KernelEvent::PluginStopped(plugin.clone()));
        Ok(())
    }
}
