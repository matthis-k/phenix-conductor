use super::*;

fn prepare_active_chain(
    runtime: InvocationContext<'_>,
    mut chain: ResolvedServiceChain,
) -> Result<ResolvedServiceChain, KernelError> {
    let configured_layers = std::mem::take(&mut chain.layers);
    for layer in configured_layers {
        if runtime.states.get(&layer.plugin).copied() == Some(PluginState::Active) {
            chain.layers.push(layer);
            continue;
        }
        let required = runtime
            .config
            .layer_policy(&chain.service)
            .iter()
            .find(|policy| policy.plugin == layer.plugin)
            .is_some_and(|policy| policy.required);
        if required {
            return Err(KernelError::RequiredLayerUnavailable {
                service: chain.service.clone(),
                plugin: layer.plugin,
            });
        }
    }
    if runtime.states.get(&chain.terminal.plugin).copied() != Some(PluginState::Active) {
        return Err(KernelError::PluginNotActive(chain.terminal.plugin.clone()));
    }
    Ok(chain)
}

pub(super) fn invoke_component_service_with(
    runtime: InvocationContext<'_>,
    service: &ServiceId,
    terminal_component: &ComponentId,
    input: &[u8],
    caller_authority: &Authority,
    binding: &PluginId,
    guards: ServiceDispatchGuards<'_>,
) -> Result<Vec<u8>, KernelError> {
    if guards.active_services.contains(service) {
        return Err(KernelError::CausalServiceReentry(service.clone()));
    }
    let resolved = runtime
        .component_graph
        .component(terminal_component)
        .ok_or_else(|| ComponentGraphError::UnknownComponent(terminal_component.clone()))?;
    if &resolved.owning_plugin != binding {
        return Err(KernelError::HostOperationDenied {
            plugin: binding.clone(),
            operation: format!(
                "resolved component {terminal_component} belongs to {}, not {binding}",
                resolved.owning_plugin
            ),
        });
    }
    let chain = runtime
        .config
        .resolve_component_chain(service, caller_authority, binding)?;
    let chain = prepare_active_chain(runtime, chain)?;
    let mut next_services = guards.active_services.clone();
    next_services.insert(service.clone());
    let trace = Arc::new(Mutex::new(InvocationTrace::new(
        &chain,
        caller_authority,
        runtime.graph_generation,
    )));
    let result = invoke_resolved_chain_with(
        runtime,
        &chain,
        0,
        input,
        caller_authority,
        ServiceDispatchGuards {
            call_stack: guards.call_stack,
            active_services: &next_services,
            terminal_component: Some(terminal_component),
        },
        &trace,
    );
    let completed = trace
        .lock()
        .expect("service invocation trace mutex poisoned")
        .clone()
        .finish();
    runtime
        .provenance
        .lock()
        .expect("service provenance mutex poisoned")
        .push(completed);
    result
}

pub(super) fn invoke_service_with(
    runtime: InvocationContext<'_>,
    service: &ServiceId,
    input: &[u8],
    caller_authority: &Authority,
    binding: Option<&PluginId>,
    call_stack: &BTreeSet<PluginId>,
    active_services: &BTreeSet<ServiceId>,
) -> Result<Vec<u8>, KernelError> {
    if active_services.contains(service) {
        return Err(KernelError::CausalServiceReentry(service.clone()));
    }
    let chain = runtime
        .config
        .resolve_chain(service, caller_authority, binding)?;
    let chain = prepare_active_chain(runtime, chain)?;
    let mut next_services = active_services.clone();
    next_services.insert(service.clone());
    let trace = Arc::new(Mutex::new(InvocationTrace::new(
        &chain,
        caller_authority,
        runtime.graph_generation,
    )));
    let result = invoke_resolved_chain_with(
        runtime,
        &chain,
        0,
        input,
        caller_authority,
        ServiceDispatchGuards {
            call_stack,
            active_services: &next_services,
            terminal_component: None,
        },
        &trace,
    );
    let completed = trace
        .lock()
        .expect("service invocation trace mutex poisoned")
        .clone()
        .finish();
    runtime
        .provenance
        .lock()
        .expect("service provenance mutex poisoned")
        .push(completed);
    result
}

#[derive(Clone, Copy)]
pub(super) struct ServiceDispatchGuards<'a> {
    pub(super) call_stack: &'a BTreeSet<PluginId>,
    pub(super) active_services: &'a BTreeSet<ServiceId>,
    pub(super) terminal_component: Option<&'a ComponentId>,
}

pub(super) fn invoke_resolved_chain_with(
    runtime: InvocationContext<'_>,
    chain: &ResolvedServiceChain,
    position: usize,
    input: &[u8],
    caller_authority: &Authority,
    guards: ServiceDispatchGuards<'_>,
    trace: &Arc<Mutex<InvocationTrace>>,
) -> Result<Vec<u8>, KernelError> {
    let (provider, is_layer) = if position < chain.layers.len() {
        (&chain.layers[position], true)
    } else {
        (&chain.terminal, false)
    };
    if guards.call_stack.contains(&provider.plugin) {
        return Err(KernelError::HostOperationDenied {
            plugin: provider.plugin.clone(),
            operation: format!("causal plugin re-entry:{}", chain.service),
        });
    }
    if runtime.states.get(&provider.plugin).copied() != Some(PluginState::Active) {
        return Err(KernelError::PluginNotActive(provider.plugin.clone()));
    }
    let provider_manifest = runtime
        .config
        .manifest(&provider.plugin)
        .expect("resolved providers are registered");
    let effective_authority = caller_authority.attenuate(&provider_manifest.maximum_authority);
    let trace_index = trace
        .lock()
        .expect("service invocation trace mutex poisoned")
        .enter(
            provider.plugin.clone(),
            if is_layer {
                ServiceRole::Layer
            } else {
                ServiceRole::Terminal
            },
            effective_authority.clone(),
        );
    let mut next_stack = guards.call_stack.clone();
    next_stack.insert(provider.plugin.clone());
    let continuation = is_layer.then(|| ContinuationState {
        chain: chain.clone(),
        terminal_component: guards.terminal_component.cloned(),
        next_position: position + 1,
        used: Arc::new(AtomicBool::new(false)),
        trace: Arc::clone(trace),
    });
    let continuation_used = continuation.as_ref().map(|state| Arc::clone(&state.used));
    let host = PluginHost {
        graph_generation: runtime.graph_generation,
        component_graph: runtime.component_graph,
        config: runtime.config,
        states: runtime.states,
        instances: runtime.instances,
        plugin: &provider.plugin,
        authority: &effective_authority,
        call_stack: next_stack,
        events: runtime.events,
        tasks: runtime.tasks,
        persistence: runtime.persistence,
        provenance: runtime.provenance,
        continuation,
        active_services: guards.active_services.clone(),
    };
    let instance = runtime
        .instances
        .get(&provider.plugin)
        .ok_or_else(|| KernelError::WrongExecutionKind(provider.plugin.clone()))?;
    let mut instance = instance.lock().expect("plugin instance mutex poisoned");
    if is_layer {
        match instance.invoke_layer(&chain.service, input, &host) {
            Ok(LayerResult::Handled(output)) => {
                let delegated = continuation_used
                    .as_ref()
                    .is_some_and(|used| used.load(Ordering::Acquire));
                trace
                    .lock()
                    .expect("service invocation trace mutex poisoned")
                    .set_outcome(
                        trace_index,
                        if delegated {
                            ServiceParticipantOutcome::Delegated
                        } else {
                            ServiceParticipantOutcome::Handled
                        },
                    );
                Ok(output)
            }
            Ok(LayerResult::Denied(message)) => {
                trace
                    .lock()
                    .expect("service invocation trace mutex poisoned")
                    .set_outcome(trace_index, ServiceParticipantOutcome::Denied);
                Err(KernelError::ServiceDenied {
                    plugin: provider.plugin.clone(),
                    service: chain.service.clone(),
                    message,
                })
            }
            Err(message) => {
                trace
                    .lock()
                    .expect("service invocation trace mutex poisoned")
                    .set_outcome(trace_index, ServiceParticipantOutcome::Failed);
                Err(KernelError::ServiceInvoke {
                    plugin: provider.plugin.clone(),
                    service: chain.service.clone(),
                    message,
                })
            }
        }
    } else {
        let result = match guards.terminal_component {
            Some(component) => instance.invoke_component(component, &chain.service, input, &host),
            None => instance.invoke(&chain.service, input, &host),
        };
        match result {
            Ok(output) => {
                trace
                    .lock()
                    .expect("service invocation trace mutex poisoned")
                    .set_outcome(trace_index, ServiceParticipantOutcome::Succeeded);
                Ok(output)
            }
            Err(message) => {
                trace
                    .lock()
                    .expect("service invocation trace mutex poisoned")
                    .set_outcome(trace_index, ServiceParticipantOutcome::Failed);
                Err(KernelError::ServiceInvoke {
                    plugin: provider.plugin.clone(),
                    service: chain.service.clone(),
                    message,
                })
            }
        }
    }
}
