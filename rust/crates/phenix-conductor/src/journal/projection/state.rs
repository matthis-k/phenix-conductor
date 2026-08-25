use super::super::ResolvedRoute;
use crate::{ConfigRevisionSlot, ExecutionRecord, SessionRecord};
use phenix_core::{
    AttemptGroup, AttemptGroupId, ConfigRevisionId, DiagnosticWritePatch, ExecutionEvent,
    ExecutionId, ExecutionReadSet, OrchestrationFailureDecisionRecord, OrchestrationNodeId,
    SessionId,
};
use std::collections::BTreeMap;

pub(crate) struct DurableProjection<'a> {
    pub config_revisions: &'a mut BTreeMap<ConfigRevisionId, ConfigRevisionSlot>,
    pub current_config_revision: &'a mut ConfigRevisionId,
    pub sessions: &'a mut BTreeMap<SessionId, SessionRecord>,
    pub executions: &'a mut BTreeMap<ExecutionId, ExecutionRecord>,
    pub root_ingress: &'a mut BTreeMap<ExecutionId, u64>,
    pub next_root_ingress: &'a mut BTreeMap<SessionId, u64>,
    pub attempt_groups: &'a mut BTreeMap<AttemptGroupId, AttemptGroup>,
    pub orchestration_decisions: &'a mut BTreeMap<ExecutionId, OrchestrationFailureDecisionRecord>,
    pub orchestration_interfaces: &'a mut BTreeMap<ExecutionId, ExecutionId>,
    pub orchestration_nodes: &'a mut BTreeMap<ExecutionId, OrchestrationNodeId>,
    pub orchestration_node_inputs:
        &'a mut BTreeMap<(ExecutionId, OrchestrationNodeId), serde_json::Value>,
    pub orchestration_synthesis: &'a mut BTreeMap<ExecutionId, ExecutionId>,
    pub execution_outputs: &'a mut BTreeMap<ExecutionId, serde_json::Value>,
    pub diagnostic_write_patches: &'a mut Vec<DiagnosticWritePatch>,
    pub resolved_routes: &'a mut BTreeMap<ExecutionId, ResolvedRoute>,
    pub read_sets: &'a mut BTreeMap<ExecutionId, ExecutionReadSet>,
    pub events: &'a mut Vec<ExecutionEvent>,
    pub next_config_revision: &'a mut u64,
    pub next_session: &'a mut u64,
    pub next_execution: &'a mut u64,
    pub next_attempt_group: &'a mut u64,
    pub next_event: &'a mut u64,
    pub next_tool_call: &'a mut u64,
}
