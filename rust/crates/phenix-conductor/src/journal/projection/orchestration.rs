use super::state::DurableProjection;
use crate::journal::{DomainEvent, JournalError};
use phenix_core::{ExecutionId, ExecutionKind, ExecutionState, OrchestrationFailureDecision};
use std::collections::btree_map::Entry;

pub(super) fn apply(
    state: &mut DurableProjection<'_>,
    event: &DomainEvent,
) -> Result<(), JournalError> {
    match event {
        DomainEvent::OrchestrationFailureInterfaceStarted {
            parent_execution,
            failed_child,
            interface_execution,
        } => {
            if state.orchestration_interfaces.contains_key(failed_child) {
                return Err(JournalError::InvalidEvent(format!(
                    "failed child {failed_child} received more than one interface execution"
                )));
            }
            let parent = state.executions.get(parent_execution).ok_or_else(|| {
                JournalError::InvalidEvent(format!(
                    "failure interface references unknown parent {parent_execution}"
                ))
            })?;
            let failed = state.executions.get(failed_child).ok_or_else(|| {
                JournalError::InvalidEvent(format!(
                    "failure interface references unknown failed child {failed_child}"
                ))
            })?;
            let interface = state.executions.get(interface_execution).ok_or_else(|| {
                JournalError::InvalidEvent(format!(
                    "failure interface references unknown execution {interface_execution}"
                ))
            })?;
            let expected_latest =
                ExecutionId::parse(format!("execution-{}", *state.next_execution))
                    .expect("generated execution id");
            if parent.summary.kind != ExecutionKind::Orchestration
                || parent.summary.state != ExecutionState::Running
                || failed.summary.parent_execution.as_ref() != Some(parent_execution)
                || failed.summary.kind != ExecutionKind::Agent
                || failed.summary.state != ExecutionState::Failed
                || interface.summary.parent_execution.as_ref() != Some(parent_execution)
                || interface.summary.kind != ExecutionKind::Agent
                || interface.summary.state != ExecutionState::Pending
                || *interface_execution != expected_latest
                || state.orchestration_nodes.contains_key(interface_execution)
            {
                return Err(JournalError::InvalidEvent(format!(
                    "invalid failure interface binding for child {failed_child}"
                )));
            }
            state
                .orchestration_interfaces
                .insert(failed_child.clone(), interface_execution.clone());
        }
        DomainEvent::OrchestrationDecisionMade { decision } => {
            if state
                .orchestration_decisions
                .contains_key(&decision.failed_child)
            {
                return Err(JournalError::InvalidEvent(format!(
                    "child {} received more than one orchestration failure decision",
                    decision.failed_child
                )));
            }
            let parent = state
                .executions
                .get(&decision.parent_execution)
                .ok_or_else(|| {
                    JournalError::InvalidEvent(format!(
                        "orchestration decision references unknown parent {}",
                        decision.parent_execution
                    ))
                })?;
            let failed = state
                .executions
                .get(&decision.failed_child)
                .ok_or_else(|| {
                    JournalError::InvalidEvent(format!(
                        "orchestration decision references unknown failed child {}",
                        decision.failed_child
                    ))
                })?;
            if parent.summary.kind != ExecutionKind::Orchestration
                || parent.summary.state != ExecutionState::Running
                || failed.summary.parent_execution.as_ref() != Some(&decision.parent_execution)
                || failed.summary.kind != ExecutionKind::Agent
                || failed.summary.state != ExecutionState::Failed
            {
                return Err(JournalError::InvalidEvent(format!(
                    "invalid orchestration decision relation for child {}",
                    decision.failed_child
                )));
            }
            match decision.decider_execution.as_ref() {
                Some(decider_id) => {
                    if state.orchestration_interfaces.get(&decision.failed_child)
                        != Some(decider_id)
                    {
                        return Err(JournalError::InvalidEvent(format!(
                            "orchestration decision decider {decider_id} is not bound to failed child {}",
                            decision.failed_child
                        )));
                    }
                    let decider = state.executions.get(decider_id).ok_or_else(|| {
                        JournalError::InvalidEvent(format!(
                            "orchestration decision references unknown decider {decider_id}"
                        ))
                    })?;
                    if decider.summary.parent_execution.as_ref() != Some(&decision.parent_execution)
                        || decider.summary.kind != ExecutionKind::Agent
                        || !matches!(
                            decider.summary.state,
                            ExecutionState::Running | ExecutionState::Completed
                        )
                    {
                        return Err(JournalError::InvalidEvent(format!(
                            "orchestration decision decider {decider_id} is not an active interface agent"
                        )));
                    }
                }
                None if !matches!(decision.decision, OrchestrationFailureDecision::Fail) => {
                    return Err(JournalError::InvalidEvent(
                        "only fail decisions may omit a decider execution".to_owned(),
                    ));
                }
                None => {}
            }
            if let Some(recovery_id) = decision.decision.recovery_execution() {
                let expected_latest =
                    ExecutionId::parse(format!("execution-{}", *state.next_execution))
                        .expect("generated execution id");
                let recovery = state.executions.get(recovery_id).ok_or_else(|| {
                    JournalError::InvalidEvent(format!(
                        "orchestration decision references unknown recovery execution {recovery_id}"
                    ))
                })?;
                if *recovery_id != expected_latest
                    || recovery.summary.parent_execution.as_ref()
                        != Some(&decision.parent_execution)
                    || recovery.summary.kind != ExecutionKind::Agent
                    || recovery.summary.state != ExecutionState::Pending
                    || state.orchestration_nodes.contains_key(recovery_id)
                    || state
                        .orchestration_interfaces
                        .values()
                        .any(|id| id == recovery_id)
                    || state
                        .orchestration_decisions
                        .values()
                        .any(|existing| existing.decision.recovery_execution() == Some(recovery_id))
                {
                    return Err(JournalError::InvalidEvent(format!(
                        "orchestration recovery {recovery_id} is not a fresh pending recovery child"
                    )));
                }
            }
            match &decision.decision {
                OrchestrationFailureDecision::Retry { execution_id } => {
                    let group = state
                        .attempt_groups
                        .values()
                        .find(|group| {
                            group.contains_execution(&decision.failed_child)
                                && group.contains_execution(execution_id)
                        })
                        .ok_or_else(|| {
                            JournalError::InvalidEvent(format!(
                                "retry decision for {} is not backed by one attempt group",
                                decision.failed_child
                            ))
                        })?;
                    if group.latest_execution() != Some(execution_id) {
                        return Err(JournalError::InvalidEvent(format!(
                            "retry decision recovery {execution_id} is not the latest attempt"
                        )));
                    }
                }
                OrchestrationFailureDecision::ChooseAnotherChild { execution_id } => {
                    let recovery = state
                        .executions
                        .get(execution_id)
                        .expect("recovery reference validated above");
                    if recovery.summary.callable == failed.summary.callable {
                        return Err(JournalError::InvalidEvent(format!(
                            "replacement decision for {} reuses the failed callable",
                            decision.failed_child
                        )));
                    }
                }
                OrchestrationFailureDecision::Continue | OrchestrationFailureDecision::Fail => {}
            }
            state
                .orchestration_decisions
                .insert(decision.failed_child.clone(), decision.clone());
        }
        DomainEvent::OrchestrationNodeStarted {
            execution_id,
            node_id,
            child_execution_id,
        } => {
            let orchestration = state.executions.get(execution_id).ok_or_else(|| {
                JournalError::InvalidEvent(format!(
                    "orchestration node references unknown execution {execution_id}"
                ))
            })?;
            if orchestration.summary.kind != ExecutionKind::Orchestration {
                return Err(JournalError::InvalidEvent(format!(
                    "orchestration node references non-orchestration execution {execution_id}"
                )));
            }
            let child = state.executions.get(child_execution_id).ok_or_else(|| {
                JournalError::InvalidEvent(format!(
                    "orchestration node {node_id} references unknown child {child_execution_id}"
                ))
            })?;
            if child.summary.parent_execution.as_ref() != Some(execution_id) {
                return Err(JournalError::InvalidEvent(format!(
                    "orchestration node {node_id} child {child_execution_id} has the wrong parent"
                )));
            }
            if state
                .orchestration_nodes
                .iter()
                .any(|(child_id, existing_node)| {
                    existing_node == node_id
                        && state
                            .executions
                            .get(child_id)
                            .and_then(|execution| execution.summary.parent_execution.as_ref())
                            == Some(execution_id)
                })
            {
                return Err(JournalError::InvalidEvent(format!(
                    "orchestration {execution_id} started node {node_id} more than once"
                )));
            }
            match state.orchestration_nodes.entry(child_execution_id.clone()) {
                Entry::Vacant(entry) => {
                    entry.insert(node_id.clone());
                }
                Entry::Occupied(_) => {
                    return Err(JournalError::InvalidEvent(format!(
                        "child execution {child_execution_id} was assigned to more than one orchestration node"
                    )));
                }
            }
        }
        DomainEvent::OrchestrationNodeInputBound {
            execution_id,
            node_id,
            input,
        } => {
            let child_exists = state
                .orchestration_nodes
                .iter()
                .any(|(child_id, bound_node)| {
                    bound_node == node_id
                        && state
                            .executions
                            .get(child_id)
                            .and_then(|execution| execution.summary.parent_execution.as_ref())
                            == Some(execution_id)
                });
            if !child_exists {
                return Err(JournalError::InvalidEvent(format!(
                    "orchestration {execution_id} input binding references unstarted node {node_id}"
                )));
            }
            if state
                .orchestration_node_inputs
                .insert((execution_id.clone(), node_id.clone()), input.clone())
                .is_some()
            {
                return Err(JournalError::InvalidEvent(format!(
                    "orchestration {execution_id} node {node_id} input was bound more than once"
                )));
            }
        }
        DomainEvent::OrchestrationSynthesisStarted {
            execution_id,
            interface_execution_id,
        } => {
            let orchestration = state.executions.get(execution_id).ok_or_else(|| {
                JournalError::InvalidEvent(format!(
                    "synthesis references unknown orchestration {execution_id}"
                ))
            })?;
            let interface = state
                .executions
                .get(interface_execution_id)
                .ok_or_else(|| {
                    JournalError::InvalidEvent(format!(
                        "synthesis references unknown interface execution {interface_execution_id}"
                    ))
                })?;
            if orchestration.summary.kind != ExecutionKind::Orchestration
                || interface.summary.parent_execution.as_ref() != Some(execution_id)
            {
                return Err(JournalError::InvalidEvent(format!(
                    "invalid synthesis binding {execution_id} -> {interface_execution_id}"
                )));
            }
            if state
                .orchestration_synthesis
                .insert(execution_id.clone(), interface_execution_id.clone())
                .is_some()
            {
                return Err(JournalError::InvalidEvent(format!(
                    "orchestration {execution_id} started synthesis more than once"
                )));
            }
        }
        DomainEvent::ExecutionOutputRecorded {
            execution_id,
            output,
        } => {
            if !state.executions.contains_key(execution_id) {
                return Err(JournalError::InvalidEvent(format!(
                    "output references unknown execution {execution_id}"
                )));
            }
            if state
                .execution_outputs
                .insert(execution_id.clone(), output.clone())
                .is_some()
            {
                return Err(JournalError::InvalidEvent(format!(
                    "execution {execution_id} output was recorded more than once"
                )));
            }
        }
        _ => unreachable!("orchestration projection received unrelated event"),
    }
    Ok(())
}
