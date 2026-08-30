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

mod dispatch;
mod host;
mod kernel;
mod reconciliation;
#[cfg(test)]
mod tests;

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
    service: ServiceId,
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
            service: chain.service.clone(),
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
            service: self.service,
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
