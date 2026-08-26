#![forbid(unsafe_code)]

mod callables;
#[cfg(test)]
mod config_revision_tests;
mod context;
mod context_budget;
mod context_compaction;
mod context_projection;
#[cfg(test)]
mod decision_tests;
mod decisions;
mod execution_provider;
mod failure_decisions;
mod journal;
mod lifecycle_hooks;
mod objectives;
mod persistence;
mod plans;
mod policy;
mod routing;
mod sandbox;
mod server;

pub use callables::{CallableRegistry, CallableRegistryError, ToolOutcome};
pub use context::{ContextError, ContextRegistry, SkillRegistry};
pub use context_budget::{
    ContextBudgetCategory, ContextBudgetError, ContextBudgetManager, ContextBudgetPolicy,
    ContextManagementDecision, ContextManagementTrigger, ContextPressure, ExecutionContextBudget,
    ResolvedModelContextCapacity,
};
pub use context_compaction::{
    ContextCheckpoint, ContextCheckpointGeneration, ContextCompactionConfiguration,
    ContextCompactionOutput, ContextCompactionRequest, ContextHistoryRange,
};
pub use context_projection::{
    ContextArtifactView, ContextManager, ContextProjectionAccounting, ContextProjectionInspection,
    ContextPruneInspection, ContextPruneReason, ExecutionContextProjection,
};
pub use decisions::DecisionError;
pub use execution_provider::{
    ExecutionProvider, ExecutionProviderBinding, ExecutionProviderError, ExecutionProviderEvent,
    ExecutionProviderHost, ExecutionProviderKind, ExecutionProviderRequest, ResolvedExactReference,
};
pub use failure_decisions::OrchestrationFailureDecisionRequest;
pub use journal::{
    DomainEvent, JournalEntry, JournalError, JournalExecutionPayload, ResolvedRoute, RuntimeJournal,
};
pub use lifecycle_hooks::{
    HookAction, HookFailurePolicy, LifecycleEvent, LifecycleHookDefinition, LifecycleHookError,
    LifecycleHookId,
};
pub use objectives::ObjectiveError;
pub use persistence::{PersistenceError, SqliteStore};
pub use plans::PlanError;
pub use policy::{
    CallableOperation, CallablePermissionGuard, InvocationGuard, InvocationPolicy,
    InvocationPolicyContext, InvocationSubject, PolicyDenial,
};
pub use routing::{RoutingRegistry, RoutingRegistryError};
pub use server::{ConductorServer, ConductorService, ServerError};

use journal::{apply_domain_event, DurableProjection};
use phenix_backend::{
    Backend, BackendCapabilities, BackendError, BackendEvent, BackendExecutionRequest, BackendHost,
    BackendSessionRequest, PreparedToolSurface, ToolInvocation, ToolProvision, ToolResult,
};
use phenix_core::{
    AgentDefinition, AttemptGroup, AttemptGroupId, CallableDescriptor, CallableId, CallableKind,
    ConfigRevisionId, ContextDescriptor, ContextResourceId, ContextResourceKind, ContextRevision,
    ContextScope, DebugConversationMessage, DebugConversationRole, DebugOrchestration,
    DebugResolvedRoute, DebugWorkspaceCheckpoint, DiagnosticWritePatch, ExecutionAuthority,
    ExecutionEvent, ExecutionEventKind, ExecutionId, ExecutionKind, ExecutionReadSet,
    ExecutionState, ExecutionSummary, ExecutionTarget, ExecutionTerminationCause,
    ExecutionWorkspaceValidity, FileObservation, FileObservationId, FileObservationInput,
    FileVersion, LanguageObservation, LanguageObservationId, LanguageObservationInput,
    LanguageOperation, ModelTarget, OrchestrationDefinition, OrchestrationFailureDecisionRecord,
    OrchestrationNodeId, RoutingProfile, RoutingProfileDescriptor, SessionDebugBundle, SessionId,
    SessionState, SessionSummary, SkillDescriptor, SkillId, ToolCallId, WorkspaceDescriptor,
    WorkspaceId, WorkspaceLeaseRequest,
};
use phenix_protocol::RuntimeSnapshot;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::path::PathBuf;

include!("runtime/error.rs");

include!("runtime/state_records.rs");

include!("runtime/configuration_types.rs");

#[derive(Clone, Debug)]
pub(crate) struct ConfigRevisionSlot {
    pub fingerprint: ConfigRevisionFingerprint,
    pub configuration: Option<CompiledConfiguration>,
    pub ordinal: u64,
}

include!("runtime/invocation_types.rs");

#[derive(Debug)]
pub struct ConductorRuntime {
    config_revision: ConfigRevisionId,
    config_revisions: BTreeMap<ConfigRevisionId, ConfigRevisionSlot>,
    workspace_id: WorkspaceId,
    sessions: BTreeMap<SessionId, SessionRecord>,
    executions: BTreeMap<ExecutionId, ExecutionRecord>,
    root_ingress: BTreeMap<ExecutionId, u64>,
    next_root_ingress: BTreeMap<SessionId, u64>,
    attempt_groups: BTreeMap<AttemptGroupId, AttemptGroup>,
    orchestration_decisions: BTreeMap<ExecutionId, OrchestrationFailureDecisionRecord>,
    orchestration_interfaces: BTreeMap<ExecutionId, ExecutionId>,
    orchestration_nodes: BTreeMap<ExecutionId, OrchestrationNodeId>,
    orchestration_node_inputs: BTreeMap<(ExecutionId, OrchestrationNodeId), Value>,
    orchestration_synthesis: BTreeMap<ExecutionId, ExecutionId>,
    execution_outputs: BTreeMap<ExecutionId, Value>,
    diagnostic_write_patches: Vec<DiagnosticWritePatch>,
    resolved_routes: BTreeMap<ExecutionId, ResolvedRoute>,
    read_sets: BTreeMap<ExecutionId, ExecutionReadSet>,
    events: Vec<ExecutionEvent>,
    journal: RuntimeJournal,
    skill_activations: BTreeMap<ExecutionId, BTreeSet<SkillId>>,
    active_lifecycle_hooks: BTreeSet<(ExecutionId, LifecycleHookId)>,
    sandbox_states: BTreeMap<ExecutionId, std::sync::Arc<sandbox::ExecutionSandboxState>>,
    policy: InvocationPolicy,
    event_sinks: BTreeMap<u64, std::sync::mpsc::Sender<ExecutionEvent>>,
    next_event_subscription: u64,
    next_config_revision: u64,
    next_session: u64,
    next_execution: u64,
    next_attempt_group: u64,
    next_event: u64,
    next_tool_call: u64,
}

impl Default for ConductorRuntime {
    fn default() -> Self {
        Self::new()
    }
}

include!("runtime/configuration.rs");
include!("runtime/debug.rs");
include!("runtime/events.rs");
include!("runtime/executions.rs");
include!("runtime/invocation.rs");
include!("runtime/lifecycle.rs");
include!("runtime/lifecycle_hook_runtime.rs");
include!("runtime/orchestration.rs");
include!("runtime/runtime.rs");
include!("runtime/sessions.rs");
include!("runtime/support.rs");
include!("runtime/tooling.rs");
include!("runtime/workspace.rs");

struct RuntimeHost<'a> {
    runtime: &'a mut ConductorRuntime,
    execution_id: ExecutionId,
    allowed_tools: BTreeSet<CallableId>,
}

impl BackendHost for RuntimeHost<'_> {
    fn emit(&mut self, event: BackendEvent) -> Result<(), BackendError> {
        let event = match event {
            BackendEvent::ContentDelta(text) => ExecutionEventKind::AssistantContentDelta { text },
            BackendEvent::ReasoningDelta(text) => ExecutionEventKind::ReasoningDelta { text },
        };
        self.runtime
            .push_event(&self.execution_id, event)
            .map(|_| ())
            .map_err(conductor_protocol_error)
    }

    fn invoke_tool(&mut self, invocation: ToolInvocation) -> Result<ToolResult, BackendError> {
        self.runtime
            .invoke_tool(&self.execution_id, &self.allowed_tools, invocation)
    }
}

struct ProviderRuntimeHost<'a> {
    runtime: &'a mut ConductorRuntime,
    execution_id: ExecutionId,
}

impl ExecutionProviderHost for ProviderRuntimeHost<'_> {
    fn emit(&mut self, event: ExecutionProviderEvent) -> Result<(), ExecutionProviderError> {
        let event = match event {
            ExecutionProviderEvent::ContentDelta(text) => {
                ExecutionEventKind::AssistantContentDelta { text }
            }
            ExecutionProviderEvent::ReasoningDelta(text) => {
                ExecutionEventKind::ReasoningDelta { text }
            }
        };
        self.runtime
            .push_event(&self.execution_id, event)
            .map(|_| ())
            .map_err(|error| ExecutionProviderError::Protocol(error.to_string()))
    }
}

include!("runtime/helpers/debug.rs");
include!("runtime/helpers/events.rs");
include!("runtime/helpers/orchestration.rs");
include!("runtime/helpers/support.rs");

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_core::{
        BackendId, CallablePolicy, CapabilitySet, FilesystemAuthority, InferenceOptions, ModelId,
        NetworkAuthority, ProviderId, RepositoryAuthority, RoutingProfileId, WorkspaceId,
        CAPABILITY_FILESYSTEM_READ, CAPABILITY_FILESYSTEM_WRITE,
    };
    use serde_json::json;

    fn fixed(name: &str) -> ExecutionTarget {
        ExecutionTarget::Fixed(ModelTarget {
            backend: BackendId::parse("mock").unwrap(),
            provider: ProviderId::parse("mock").unwrap(),
            model: ModelId::parse(name).unwrap(),
            inference: InferenceOptions::default(),
        })
    }

    fn agent(id: &str) -> CallableDescriptor {
        CallableDescriptor {
            id: CallableId::parse(id).unwrap(),
            kind: CallableKind::Agent,
            description: "test agent".to_owned(),
            input_schema: json!({"type": "object"}),
            output_schema: json!({"type": "object"}),
            capabilities: CapabilitySet::default(),
            policy: CallablePolicy::default(),
        }
    }

    fn tool(id: &str, capabilities: &[&str]) -> CallableDescriptor {
        CallableDescriptor {
            id: CallableId::parse(id).unwrap(),
            kind: CallableKind::Tool,
            description: "test tool".to_owned(),
            input_schema: json!({"type": "object"}),
            output_schema: json!({"type": "object"}),
            capabilities: CapabilitySet(
                capabilities
                    .iter()
                    .map(|capability| (*capability).to_owned())
                    .collect(),
            ),
            policy: CallablePolicy::default(),
        }
    }

    fn authority(
        filesystem: FilesystemAuthority,
        network: NetworkAuthority,
        repository: RepositoryAuthority,
        ipc: &[&str],
        secrets: &[&str],
        callables: &[&str],
    ) -> ExecutionAuthority {
        ExecutionAuthority {
            filesystem,
            network,
            repository,
            ipc: ipc.iter().map(|value| (*value).to_owned()).collect(),
            secrets: secrets.iter().map(|value| (*value).to_owned()).collect(),
            callables: callables
                .iter()
                .map(|value| CallableId::parse(*value).unwrap())
                .collect(),
        }
    }

    include!("runtime/tests/configuration.rs");
    include!("runtime/tests/executions.rs");
    include!("runtime/tests/invocation.rs");
    include!("runtime/tests/lifecycle.rs");
    include!("runtime/tests/orchestration.rs");
    include!("runtime/tests/runtime.rs");
    include!("runtime/tests/sessions.rs");
    include!("runtime/tests/tooling.rs");
}
