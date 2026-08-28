use crate::{
    Authority, CapabilityId, ComponentGraphError, ComponentId, ComponentInterface,
    ComponentInvocationError, DurableSchema, EventBus, EventDispatchReport, EventEnvelope,
    EventError, EventTypeId, GraphGenerationId, KernelConfig, KernelError, KernelEvent,
    KernelPolicyIdentity, LocalPersistence, NamespaceTransaction, PersistenceBackend,
    PluginExecution, PluginId, PluginManifest, ResolvedComponentGraph, ResolvedServiceChain,
    ResourceNamespace, SchemaMigration, ServiceId, ServiceRole, SkillResourceMetadata, TaskHandle,
    TaskRuntime, TransactionOp,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

mod reconciliation;

const PERSISTENCE_SCHEMA: &str = "kernel.persistence.schema";
const PERSISTENCE_READ: &str = "kernel.persistence.read";
const PERSISTENCE_WRITE: &str = "kernel.persistence.write";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PluginState {
    Registered,
    Active,
    Stopped,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServiceParticipantOutcome {
    Handled,
    Delegated,
    Denied,
    Failed,
    Succeeded,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceParticipantProvenance {
    pub plugin: PluginId,
    pub role: ServiceRole,
    pub effective_authority: Authority,
    pub outcome: ServiceParticipantOutcome,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceInvocationProvenance {
    pub graph_generation: Option<GraphGenerationId>,
    pub policy_identity: KernelPolicyIdentity,
    pub service: ServiceId,
    pub planned_chain: ResolvedServiceChain,
    pub caller_authority: Authority,
    pub participants: Vec<ServiceParticipantProvenance>,
    pub terminal_reached: bool,
}

#[derive(Clone, Debug)]
struct PendingParticipantProvenance {
    plugin: PluginId,
    role: ServiceRole,
    effective_authority: Authority,
    outcome: Option<ServiceParticipantOutcome>,
}

#[derive(Clone, Debug)]
struct InvocationTrace {
    graph_generation: Option<GraphGenerationId>,
    planned_chain: ResolvedServiceChain,
    caller_authority: Authority,
    participants: Vec<PendingParticipantProvenance>,
    terminal_reached: bool,
}

impl InvocationTrace {
    fn new(
        chain: &ResolvedServiceChain,
        caller_authority: &Authority,
        graph_generation: Option<&GraphGenerationId>,
    ) -> Self {
        Self {
            graph_generation: graph_generation.cloned(),
            planned_chain: chain.clone(),
            caller_authority: caller_authority.clone(),
            participants: Vec::new(),
            terminal_reached: false,
        }
    }

    fn enter(
        &mut self,
        plugin: PluginId,
        role: ServiceRole,
        effective_authority: Authority,
    ) -> usize {
        if role == ServiceRole::Terminal {
            self.terminal_reached = true;
        }
        let index = self.participants.len();
        self.participants.push(PendingParticipantProvenance {
            plugin,
            role,
            effective_authority,
            outcome: None,
        });
        index
    }

    fn set_outcome(&mut self, index: usize, outcome: ServiceParticipantOutcome) {
        self.participants[index].outcome = Some(outcome);
    }

    fn finish(self) -> ServiceInvocationProvenance {
        ServiceInvocationProvenance {
            graph_generation: self.graph_generation,
            policy_identity: self.planned_chain.policy_identity,
            service: self.planned_chain.service.clone(),
            planned_chain: self.planned_chain,
            caller_authority: self.caller_authority,
            participants: self
                .participants
                .into_iter()
                .map(|participant| ServiceParticipantProvenance {
                    plugin: participant.plugin,
                    role: participant.role,
                    effective_authority: participant.effective_authority,
                    outcome: participant
                        .outcome
                        .unwrap_or(ServiceParticipantOutcome::Failed),
                })
                .collect(),
            terminal_reached: self.terminal_reached,
        }
    }
}

#[derive(Clone)]
struct ContinuationState {
    chain: ResolvedServiceChain,
    terminal_component: Option<ComponentId>,
    next_position: usize,
    used: Arc<AtomicBool>,
    trace: Arc<Mutex<InvocationTrace>>,
}

#[derive(Clone)]
pub(crate) struct ContinuationBinding {
    pub graph_generation: Option<GraphGenerationId>,
    pub policy_identity: KernelPolicyIdentity,
    pub service: ServiceId,
    pub authority: Authority,
    pub next_position: usize,
}

pub struct PluginHost<'a> {
    graph_generation: Option<&'a GraphGenerationId>,
    component_graph: &'a ResolvedComponentGraph,
    config: &'a KernelConfig,
    states: &'a BTreeMap<PluginId, PluginState>,
    instances: &'a BTreeMap<PluginId, Arc<Mutex<Box<dyn PluginInstance>>>>,
    plugin: &'a PluginId,
    authority: &'a Authority,
    call_stack: BTreeSet<PluginId>,
    events: &'a EventBus,
    tasks: &'a TaskRuntime,
    persistence: &'a Mutex<Box<dyn PersistenceBackend>>,
    provenance: &'a Mutex<Vec<ServiceInvocationProvenance>>,
    continuation: Option<ContinuationState>,
    active_services: BTreeSet<ServiceId>,
}

impl<'a> PluginHost<'a> {
    pub fn graph_generation(&self) -> Option<&GraphGenerationId> {
        self.graph_generation
    }

    pub fn plugin(&self) -> &PluginId {
        self.plugin
    }

    pub fn authority(&self) -> &Authority {
        self.authority
    }

    pub fn invoke_import<I: ComponentInterface>(
        &self,
        component: &ComponentId,
        request: &I::Request,
    ) -> Result<I::Response, ComponentInvocationError> {
        let resolved = self
            .component_graph
            .component(component)
            .ok_or_else(|| crate::ComponentGraphError::UnknownComponent(component.clone()))?;
        if &resolved.owning_plugin != self.plugin {
            return Err(KernelError::HostOperationDenied {
                plugin: self.plugin.clone(),
                operation: format!("component import owned by {}", resolved.owning_plugin),
            }
            .into());
        }
        let interface = I::interface_id();
        let handle = self
            .component_graph
            .import_handle(component, &interface)?
            .ok_or_else(|| ComponentInvocationError::UnboundImport {
                component: component.clone(),
                interface: interface.clone(),
            })?;
        let service = ServiceId::parse(interface.as_str().to_owned()).map_err(|message| {
            ComponentInvocationError::InvalidInterface {
                interface: interface.clone(),
                message: message.into(),
            }
        })?;
        let input = serde_json::to_vec(request)
            .map_err(|error| ComponentInvocationError::Encode(error.to_string()))?;
        let delegated_authority = self.authority.attenuate(handle.effective_authority());
        let output = invoke_component_service_with(
            InvocationContext {
                graph_generation: self.graph_generation,
                component_graph: self.component_graph,
                config: self.config,
                states: self.states,
                instances: self.instances,
                events: self.events,
                tasks: self.tasks,
                persistence: self.persistence,
                provenance: self.provenance,
            },
            &service,
            handle.exporter(),
            &input,
            &delegated_authority,
            handle.owning_plugin(),
            ServiceDispatchGuards {
                call_stack: &self.call_stack,
                active_services: &self.active_services,
                terminal_component: Some(handle.exporter()),
            },
        )?;
        serde_json::from_slice(&output)
            .map_err(|error| ComponentInvocationError::Decode(error.to_string()))
    }

    #[doc(hidden)]
    pub fn invoke_service_abi(
        &self,
        service: &ServiceId,
        input: &[u8],
        requested_authority: &Authority,
        binding: Option<&PluginId>,
    ) -> Result<Vec<u8>, KernelError> {
        let delegated_authority = self.authority.attenuate(requested_authority);
        invoke_service_with(
            InvocationContext {
                graph_generation: self.graph_generation,
                component_graph: self.component_graph,
                config: self.config,
                states: self.states,
                instances: self.instances,
                events: self.events,
                tasks: self.tasks,
                persistence: self.persistence,
                provenance: self.provenance,
            },
            service,
            input,
            &delegated_authority,
            binding,
            &self.call_stack,
            &self.active_services,
        )
    }

    pub fn continue_service(
        &self,
        input: &[u8],
        requested_authority: &Authority,
    ) -> Result<Vec<u8>, KernelError> {
        let continuation = self
            .continuation
            .as_ref()
            .ok_or(KernelError::ContinuationUnavailable)?;
        let service = continuation.chain.service.clone();
        if continuation.used.swap(true, Ordering::AcqRel) {
            return Err(KernelError::ContinuationAlreadyUsed(service));
        }
        let delegated_authority = self.authority.attenuate(requested_authority);
        invoke_resolved_chain_with(
            InvocationContext {
                graph_generation: self.graph_generation,
                component_graph: self.component_graph,
                config: self.config,
                states: self.states,
                instances: self.instances,
                events: self.events,
                tasks: self.tasks,
                persistence: self.persistence,
                provenance: self.provenance,
            },
            &continuation.chain,
            continuation.next_position,
            input,
            &delegated_authority,
            ServiceDispatchGuards {
                call_stack: &self.call_stack,
                active_services: &self.active_services,
                terminal_component: continuation.terminal_component.as_ref(),
            },
            &continuation.trace,
        )
    }

    pub(crate) fn continuation_binding(&self) -> Result<ContinuationBinding, KernelError> {
        let continuation = self
            .continuation
            .as_ref()
            .ok_or(KernelError::ContinuationUnavailable)?;
        Ok(ContinuationBinding {
            graph_generation: self.graph_generation.cloned(),
            policy_identity: continuation.chain.policy_identity,
            service: continuation.chain.service.clone(),
            authority: self.authority.clone(),
            next_position: continuation.next_position,
        })
    }

    pub fn spawn_task<T, F>(&self, requested_authority: &Authority, worker: F) -> TaskHandle<T>
    where
        T: Send + 'static,
        F: FnOnce(crate::CancellationToken) -> T + Send + 'static,
    {
        self.tasks.spawn(
            self.graph_generation
                .expect("plugin host task spawn requires a resolved graph generation"),
            self.authority,
            requested_authority,
            worker,
        )
    }

    pub fn dispatch_event(
        &self,
        event_type: EventTypeId,
        version: u32,
        causality_id: u64,
        kernel_policy_revision: u64,
        payload: Vec<u8>,
    ) -> Result<EventDispatchReport, EventError> {
        let event = EventEnvelope {
            event_type,
            version,
            emitter: self.plugin.clone(),
            causality_id,
            kernel_policy_revision,
            payload,
        };
        self.events.dispatch(&event, self.authority)
    }

    pub fn register_durable_schema(&self, schema: &DurableSchema) -> Result<(), KernelError> {
        self.require_persistence_operation(PERSISTENCE_SCHEMA, &schema.namespace)?;
        self.persistence
            .lock()
            .expect("kernel persistence mutex poisoned")
            .register_schema(self.plugin, schema)
            .map_err(|error| self.persistence_error(error.to_string()))
    }

    pub fn migrate_durable_schema(
        &self,
        schema: &DurableSchema,
        migrations: &[SchemaMigration],
    ) -> Result<(), KernelError> {
        self.require_persistence_operation(PERSISTENCE_SCHEMA, &schema.namespace)?;
        self.require_capability(PERSISTENCE_WRITE)?;
        self.persistence
            .lock()
            .expect("kernel persistence mutex poisoned")
            .migrate_schema(self.plugin, schema, migrations)
            .map_err(|error| self.persistence_error(error.to_string()))
    }

    pub fn read_durable(
        &self,
        namespace: &ResourceNamespace,
        key: &str,
    ) -> Result<Option<Vec<u8>>, KernelError> {
        self.require_persistence_operation(PERSISTENCE_READ, namespace)?;
        self.persistence
            .lock()
            .expect("kernel persistence mutex poisoned")
            .read(self.plugin, namespace, key)
            .map_err(|error| self.persistence_error(error.to_string()))
    }

    pub fn transact_durable(
        &self,
        namespace: &ResourceNamespace,
        operations: &[TransactionOp],
    ) -> Result<(), KernelError> {
        self.require_persistence_operation(PERSISTENCE_WRITE, namespace)?;
        self.persistence
            .lock()
            .expect("kernel persistence mutex poisoned")
            .transact(self.plugin, namespace, operations)
            .map_err(|error| self.persistence_error(error.to_string()))
    }

    pub fn transact_durable_many(
        &self,
        transactions: &[NamespaceTransaction],
    ) -> Result<(), KernelError> {
        self.require_capability(PERSISTENCE_WRITE)?;
        if !transactions
            .iter()
            .any(|transaction| &transaction.owner == self.plugin)
        {
            return Err(KernelError::HostOperationDenied {
                plugin: self.plugin.clone(),
                operation: "multi-namespace transaction requires a caller-owned participant".into(),
            });
        }
        for transaction in transactions {
            if self.config.resource_owner(&transaction.namespace) != Some(&transaction.owner) {
                return Err(KernelError::HostOperationDenied {
                    plugin: self.plugin.clone(),
                    operation: format!(
                        "{PERSISTENCE_WRITE}:{}:{}",
                        transaction.owner, transaction.namespace
                    ),
                });
            }
            if &transaction.owner == self.plugin {
                continue;
            }
            let write = CapabilityId::parse(PERSISTENCE_WRITE)
                .expect("kernel persistence write capability is valid");
            let authorized_import = self
                .component_graph
                .components()
                .filter(|component| &component.owning_plugin == self.plugin)
                .flat_map(|component| component.imports.iter())
                .filter_map(|import| import.binding.as_ref())
                .any(|binding| {
                    binding.owning_plugin() == &transaction.owner
                        && binding.effective_authority().permits(&write)
                });
            if !authorized_import {
                return Err(KernelError::HostOperationDenied {
                    plugin: self.plugin.clone(),
                    operation: format!(
                        "{PERSISTENCE_WRITE}:{}:{} without authorized typed import",
                        transaction.owner, transaction.namespace
                    ),
                });
            }
        }
        self.persistence
            .lock()
            .expect("kernel persistence mutex poisoned")
            .transact_many(transactions)
            .map_err(|error| self.persistence_error(error.to_string()))
    }

    fn require_persistence_operation(
        &self,
        capability: &str,
        namespace: &ResourceNamespace,
    ) -> Result<(), KernelError> {
        self.require_capability(capability)?;
        if self.config.resource_owner(namespace) == Some(self.plugin) {
            return Ok(());
        }
        Err(KernelError::HostOperationDenied {
            plugin: self.plugin.clone(),
            operation: format!("{capability}:{}", namespace.as_str()),
        })
    }

    fn require_capability(&self, capability: &str) -> Result<(), KernelError> {
        let capability = CapabilityId::parse(capability).expect("kernel capability is valid");
        if self.authority.permits(&capability) {
            return Ok(());
        }
        Err(KernelError::HostOperationDenied {
            plugin: self.plugin.clone(),
            operation: capability.as_str().to_owned(),
        })
    }

    fn persistence_error(&self, message: String) -> KernelError {
        KernelError::Persistence {
            plugin: self.plugin.clone(),
            message,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LayerResult {
    Handled(Vec<u8>),
    Denied(String),
}

pub trait PluginInstance: Send {
    fn start(&mut self, host: &PluginHost<'_>) -> Result<(), String>;

    fn invoke(
        &mut self,
        _service: &ServiceId,
        _input: &[u8],
        _host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        Err("service invocation is not implemented".into())
    }

    fn invoke_component(
        &mut self,
        _component: &ComponentId,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        self.invoke(service, input, host)
    }

    fn invoke_layer(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<LayerResult, String> {
        self.invoke(service, input, host).map(LayerResult::Handled)
    }

    fn stop(&mut self) -> Result<(), String> {
        Ok(())
    }
}

type EmbeddedFactory = Arc<dyn Fn() -> Box<dyn PluginInstance> + Send + Sync>;
type ExternalFactory =
    Arc<dyn Fn(&PluginManifest) -> Result<Box<dyn PluginInstance>, String> + Send + Sync>;

#[derive(Clone, Copy)]
struct InvocationContext<'a> {
    graph_generation: Option<&'a GraphGenerationId>,
    component_graph: &'a ResolvedComponentGraph,
    config: &'a KernelConfig,
    states: &'a BTreeMap<PluginId, PluginState>,
    instances: &'a BTreeMap<PluginId, Arc<Mutex<Box<dyn PluginInstance>>>>,
    events: &'a EventBus,
    tasks: &'a TaskRuntime,
    persistence: &'a Mutex<Box<dyn PersistenceBackend>>,
    provenance: &'a Mutex<Vec<ServiceInvocationProvenance>>,
}

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

fn invoke_component_service_with(
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

fn invoke_service_with(
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
struct ServiceDispatchGuards<'a> {
    call_stack: &'a BTreeSet<PluginId>,
    active_services: &'a BTreeSet<ServiceId>,
    terminal_component: Option<&'a ComponentId>,
}

fn invoke_resolved_chain_with(
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

pub struct Kernel {
    graph_generation: Option<GraphGenerationId>,
    component_graph: ResolvedComponentGraph,
    active_resources: Vec<SkillResourceMetadata>,
    config: KernelConfig,
    states: BTreeMap<PluginId, PluginState>,
    embedded_factories: BTreeMap<PluginId, EmbeddedFactory>,
    external_factories: BTreeMap<PluginId, ExternalFactory>,
    instances: BTreeMap<PluginId, Arc<Mutex<Box<dyn PluginInstance>>>>,
    events: Arc<EventBus>,
    tasks: TaskRuntime,
    persistence: Mutex<Box<dyn PersistenceBackend>>,
    provenance: Mutex<Vec<ServiceInvocationProvenance>>,
    runtime_active: bool,
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ComponentExport, ComponentImport, ComponentManifest, InterfaceId, PluginManifest,
        ServiceContribution,
    };
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    fn plugin(value: &str) -> PluginId {
        PluginId::parse(value).unwrap()
    }

    fn capability(value: &str) -> CapabilityId {
        CapabilityId::parse(value).unwrap()
    }

    fn service(value: &str) -> ServiceId {
        ServiceId::parse(value).unwrap()
    }

    struct MarkerPlugin(Arc<AtomicBool>);
    impl PluginInstance for MarkerPlugin {
        fn start(&mut self, host: &PluginHost<'_>) -> Result<(), String> {
            assert_eq!(host.plugin().as_str(), "embedded");
            self.0.store(true, Ordering::Release);
            Ok(())
        }
    }

    struct EchoPlugin;

    impl PluginInstance for EchoPlugin {
        fn start(&mut self, _host: &PluginHost<'_>) -> Result<(), String> {
            Ok(())
        }

        fn invoke(
            &mut self,
            _service: &ServiceId,
            input: &[u8],
            host: &PluginHost<'_>,
        ) -> Result<Vec<u8>, String> {
            if host.authority().permits(&capability("fs.write")) {
                return Err("provider regained caller write authority".into());
            }
            Ok(input.to_vec())
        }
    }

    struct PersistencePlugin {
        namespace: ResourceNamespace,
    }

    impl PluginInstance for PersistencePlugin {
        fn start(&mut self, host: &PluginHost<'_>) -> Result<(), String> {
            host.register_durable_schema(&DurableSchema::new(self.namespace.clone(), 1))
                .map_err(|error| error.to_string())?;
            host.transact_durable(
                &self.namespace,
                &[TransactionOp::Put {
                    key: "seed".into(),
                    value: b"ready".to_vec(),
                }],
            )
            .map_err(|error| error.to_string())
        }

        fn invoke(
            &mut self,
            _service: &ServiceId,
            input: &[u8],
            host: &PluginHost<'_>,
        ) -> Result<Vec<u8>, String> {
            match input {
                b"read" => host
                    .read_durable(&self.namespace, "seed")
                    .map_err(|error| error.to_string())?
                    .ok_or_else(|| "missing seed".to_owned()),
                b"write" => {
                    host.transact_durable(
                        &self.namespace,
                        &[TransactionOp::Put {
                            key: "changed".into(),
                            value: b"yes".to_vec(),
                        }],
                    )
                    .map_err(|error| error.to_string())?;
                    Ok(b"written".to_vec())
                }
                _ => Err("unsupported input".into()),
            }
        }
    }

    #[test]
    fn kernel_only_boots_without_agent_domain_services() {
        let mut kernel = Kernel::kernel_only();
        kernel.activate_all().unwrap();
        assert_eq!(kernel.config().manifests().count(), 0);
    }

    #[test]
    fn embedded_and_resource_only_plugins_share_lifecycle_contract() {
        let marker = Arc::new(AtomicBool::new(false));
        let embedded = PluginManifest {
            id: plugin("embedded"),
            version: 1,
            execution: PluginExecution::Embedded,
            dependencies: Vec::new(),
            services: Vec::new(),
            resource_namespaces: Vec::new(),
            maximum_authority: Authority::default(),
        };
        let mut resources = PluginManifest::resource_only(plugin("resources"));
        resources
            .resource_namespaces
            .push(ResourceNamespace::parse("resources.static").unwrap());

        let mut kernel = Kernel::new(KernelConfig::new([resources, embedded]).unwrap());
        let marker_for_factory = Arc::clone(&marker);
        kernel
            .register_embedded_factory(plugin("embedded"), move || {
                Box::new(MarkerPlugin(Arc::clone(&marker_for_factory)))
            })
            .unwrap();

        kernel.activate_all().unwrap();

        assert!(marker.load(Ordering::Acquire));
        assert_eq!(kernel.state(&plugin("embedded")), Some(PluginState::Active));
        assert_eq!(
            kernel.state(&plugin("resources")),
            Some(PluginState::Active)
        );
        assert_eq!(
            kernel
                .config()
                .resource_owner(&ResourceNamespace::parse("resources.static").unwrap()),
            Some(&plugin("resources"))
        );
    }

    #[test]
    fn invocation_uses_caller_authority_attenuated_by_provider_grant() {
        let read = capability("fs.read");
        let write = capability("fs.write");
        let provider = PluginManifest {
            id: plugin("echo"),
            version: 1,
            execution: PluginExecution::Embedded,
            dependencies: Vec::new(),
            services: vec![ServiceContribution {
                role: crate::ServiceRole::Terminal,
                service: service("echo@1"),
                priority: 1,
                required_authority: Authority::new([read.clone()]),
            }],
            resource_namespaces: Vec::new(),
            maximum_authority: Authority::new([read.clone()]),
        };
        let mut kernel = Kernel::new(KernelConfig::new([provider]).unwrap());
        kernel
            .register_embedded_factory(plugin("echo"), || Box::new(EchoPlugin))
            .unwrap();
        kernel.activate_all().unwrap();

        let output = kernel
            .invoke(
                &service("echo@1"),
                b"hello",
                &Authority::new([read, write]),
                None,
            )
            .unwrap();
        assert_eq!(output, b"hello");
    }

    #[test]
    fn persistence_host_rechecks_effective_authority_on_every_call() {
        let schema = capability(PERSISTENCE_SCHEMA);
        let read = capability(PERSISTENCE_READ);
        let write = capability(PERSISTENCE_WRITE);
        let namespace = ResourceNamespace::parse("storage.state").unwrap();
        let provider = PluginManifest {
            id: plugin("storage"),
            version: 1,
            execution: PluginExecution::Embedded,
            dependencies: Vec::new(),
            services: vec![ServiceContribution {
                role: crate::ServiceRole::Terminal,
                service: service("storage@1"),
                priority: 1,
                required_authority: Authority::default(),
            }],
            resource_namespaces: vec![namespace.clone()],
            maximum_authority: Authority::new([schema.clone(), read.clone(), write.clone()]),
        };
        let mut kernel = Kernel::new(KernelConfig::new([provider]).unwrap());
        kernel
            .register_embedded_factory(plugin("storage"), move || {
                Box::new(PersistencePlugin {
                    namespace: namespace.clone(),
                })
            })
            .unwrap();
        kernel.activate_all().unwrap();

        assert_eq!(
            kernel
                .invoke(
                    &service("storage@1"),
                    b"read",
                    &Authority::new([read.clone()]),
                    None,
                )
                .unwrap(),
            b"ready"
        );

        let error = kernel
            .invoke(
                &service("storage@1"),
                b"write",
                &Authority::new([read, write]),
                None,
            )
            .unwrap();
        assert_eq!(error, b"written");

        let denied = kernel
            .invoke(
                &service("storage@1"),
                b"write",
                &Authority::new([capability(PERSISTENCE_READ)]),
                None,
            )
            .unwrap_err();
        assert!(matches!(denied, KernelError::ServiceInvoke { .. }));
        assert!(denied.to_string().contains(PERSISTENCE_WRITE));
    }

    #[test]
    fn multi_namespace_transaction_requires_write_authority_on_foreign_typed_import() {
        let write = capability(PERSISTENCE_WRITE);
        let caller_namespace = ResourceNamespace::parse("caller.state").unwrap();
        let owner_namespace = ResourceNamespace::parse("owner.state").unwrap();
        let interface = InterfaceId::parse("fixture.owner.persistence@1").unwrap();
        let caller = PluginManifest {
            id: plugin("caller"),
            version: 1,
            execution: PluginExecution::Embedded,
            dependencies: Vec::new(),
            services: Vec::new(),
            resource_namespaces: vec![caller_namespace.clone()],
            maximum_authority: Authority::new([write.clone()]),
        };
        let owner = PluginManifest {
            id: plugin("owner"),
            version: 1,
            execution: PluginExecution::Embedded,
            dependencies: Vec::new(),
            services: Vec::new(),
            resource_namespaces: vec![owner_namespace.clone()],
            maximum_authority: Authority::new([write.clone()]),
        };
        let caller_component = ComponentManifest {
            id: ComponentId::parse("caller.component").unwrap(),
            owner: caller.id.clone(),
            imports: vec![ComponentImport {
                interface: interface.clone(),
                required: true,
                authority: Authority::default(),
            }],
            exports: Vec::new(),
            maximum_authority: caller.maximum_authority.clone(),
        };
        let owner_component = ComponentManifest {
            id: ComponentId::parse("owner.component").unwrap(),
            owner: owner.id.clone(),
            imports: Vec::new(),
            exports: vec![ComponentExport {
                interface,
                priority: 1,
                required_authority: Authority::default(),
            }],
            maximum_authority: owner.maximum_authority.clone(),
        };
        let graph = ResolvedComponentGraph::compile(
            [caller.clone(), owner.clone()],
            [caller_component, owner_component],
            &Authority::new([write.clone()]),
        )
        .unwrap();
        let kernel = Kernel::new(KernelConfig::new([caller.clone(), owner.clone()]).unwrap());
        let caller_plugin = caller.id;
        let authority = Authority::new([write]);
        let host = PluginHost {
            graph_generation: kernel.graph_generation(),
            component_graph: &graph,
            config: kernel.config(),
            states: &kernel.states,
            instances: &kernel.instances,
            plugin: &caller_plugin,
            authority: &authority,
            call_stack: BTreeSet::from([caller_plugin.clone()]),
            events: &kernel.events,
            tasks: &kernel.tasks,
            persistence: &kernel.persistence,
            provenance: &kernel.provenance,
            continuation: None,
            active_services: BTreeSet::new(),
        };
        let denied = host
            .transact_durable_many(&[
                NamespaceTransaction {
                    owner: caller_plugin.clone(),
                    namespace: caller_namespace,
                    operations: Vec::new(),
                },
                NamespaceTransaction {
                    owner: owner.id,
                    namespace: owner_namespace,
                    operations: vec![TransactionOp::Put {
                        key: "forbidden".into(),
                        value: b"write".to_vec(),
                    }],
                },
            ])
            .unwrap_err();
        assert!(matches!(denied, KernelError::HostOperationDenied { .. }));
        assert!(denied
            .to_string()
            .contains("without authorized typed import"));
    }

    #[test]
    fn persistence_host_rejects_unowned_namespace_before_backend_access() {
        let namespace = ResourceNamespace::parse("owned.state").unwrap();
        let other_namespace = ResourceNamespace::parse("other.state").unwrap();
        let owner = PluginManifest {
            id: plugin("owner"),
            version: 1,
            execution: PluginExecution::Embedded,
            dependencies: Vec::new(),
            services: Vec::new(),
            resource_namespaces: vec![namespace],
            maximum_authority: Authority::new([capability(PERSISTENCE_SCHEMA)]),
        };
        let kernel = Kernel::new(KernelConfig::new([owner]).unwrap());
        let authority = Authority::new([capability(PERSISTENCE_SCHEMA)]);
        let owner_plugin = plugin("owner");
        let host = PluginHost {
            graph_generation: kernel.graph_generation(),
            component_graph: kernel.component_graph(),
            config: kernel.config(),
            states: &kernel.states,
            instances: &kernel.instances,
            plugin: &owner_plugin,
            authority: &authority,
            call_stack: BTreeSet::from([owner_plugin.clone()]),
            events: &kernel.events,
            tasks: &kernel.tasks,
            persistence: &kernel.persistence,
            provenance: &kernel.provenance,
            continuation: None,
            active_services: BTreeSet::new(),
        };
        assert!(matches!(
            host.register_durable_schema(&DurableSchema::new(other_namespace, 1)),
            Err(KernelError::HostOperationDenied { .. })
        ));
    }
}
