use super::*;
use crate::ResolvedHarness;

impl Kernel {
    /// Replace the active runtime topology with one fully resolved candidate generation.
    /// Candidate implementations are staged before the canonical config/graph swap so a
    /// start failure leaves the previously active generation untouched.
    pub(crate) fn reconcile_resolved_generation(
        &mut self,
        candidate: &ResolvedHarness,
        restart_plugins: &BTreeSet<PluginId>,
    ) -> Result<(), KernelError> {
        let has_active = self
            .states
            .values()
            .any(|state| *state == PluginState::Active);
        let all_active = self
            .states
            .values()
            .all(|state| *state == PluginState::Active);
        if (self.runtime_active && !all_active) || (!self.runtime_active && has_active) {
            return Err(KernelError::PartiallyActiveRuntime);
        }

        let active_runtime = self.runtime_active;
        let candidate_config = candidate.kernel_config().clone();
        let old_manifests: BTreeMap<_, _> = self
            .config
            .manifests()
            .map(|manifest| (manifest.id.clone(), manifest.clone()))
            .collect();
        let candidate_manifests: BTreeMap<_, _> = candidate_config
            .manifests()
            .map(|manifest| (manifest.id.clone(), manifest.clone()))
            .collect();
        let mut next_states = BTreeMap::new();
        let mut next_instances = BTreeMap::new();
        let mut staged = Vec::new();

        for (plugin, manifest) in &candidate_manifests {
            let retain = active_runtime
                && !restart_plugins.contains(plugin)
                && old_manifests.get(plugin) == Some(manifest);
            if retain {
                next_states.insert(plugin.clone(), PluginState::Active);
                if let Some(instance) = self.instances.get(plugin) {
                    next_instances.insert(plugin.clone(), Arc::clone(instance));
                }
            } else {
                next_states.insert(plugin.clone(), PluginState::Registered);
            }
        }

        if active_runtime {
            for plugin in candidate_config.activation_order() {
                if next_states.get(plugin) == Some(&PluginState::Active) {
                    continue;
                }
                let manifest = &candidate_manifests[plugin];
                let instance =
                    match &manifest.execution {
                        PluginExecution::ResourceOnly => None,
                        PluginExecution::Embedded => {
                            Some(self.embedded_factories.get(plugin).ok_or_else(|| {
                                KernelError::EmbeddedFactoryMissing(plugin.clone())
                            })?())
                        }
                        PluginExecution::External { .. } => Some(
                            self.external_factories.get(plugin).ok_or_else(|| {
                                KernelError::ExternalHostUnavailable(plugin.clone())
                            })?(manifest)
                            .map_err(|message| KernelError::PluginStart {
                                plugin: plugin.clone(),
                                message,
                            })?,
                        ),
                    };
                if let Some(mut instance) = instance {
                    let host = PluginHost {
                        graph_generation: Some(candidate.generation()),
                        component_graph: candidate.component_graph(),
                        config: &candidate_config,
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
                        cleanup_staged(&staged, &next_instances);
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
        }

        let retired: Vec<_> = old_manifests
            .iter()
            .filter(|(plugin, old_manifest)| {
                candidate_manifests.get(*plugin) != Some(*old_manifest)
                    || restart_plugins.contains(*plugin)
            })
            .filter_map(|(plugin, _)| {
                self.instances
                    .get(plugin)
                    .map(|instance| (plugin.clone(), Arc::clone(instance)))
            })
            .collect();

        self.config = candidate_config;
        self.states = next_states;
        self.instances = next_instances;
        self.install_resolved_graph(
            candidate.generation().clone(),
            candidate.component_graph().clone(),
            candidate.resources().to_vec(),
        );
        self.runtime_active = active_runtime;

        for (plugin, instance) in retired {
            let _ = instance
                .lock()
                .expect("retired plugin instance mutex poisoned")
                .stop();
            self.events.publish(KernelEvent::PluginStopped(plugin));
        }
        for plugin in staged {
            self.events.publish(KernelEvent::PluginActivated(plugin));
        }
        Ok(())
    }
}

fn cleanup_staged(
    staged: &[PluginId],
    instances: &BTreeMap<PluginId, Arc<Mutex<Box<dyn PluginInstance>>>>,
) {
    for plugin in staged.iter().rev() {
        if let Some(instance) = instances.get(plugin) {
            let _ = instance
                .lock()
                .expect("staged plugin instance mutex poisoned")
                .stop();
        }
    }
}
