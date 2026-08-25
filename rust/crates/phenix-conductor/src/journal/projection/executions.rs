use super::replay::materialize_execution_payload;
use super::state::DurableProjection;
use crate::journal::{DomainEvent, JournalError};
use crate::{ExecutionPayload, ExecutionRecord};
use phenix_core::{ExecutionEventKind, ExecutionId, ExecutionState, ExecutionTarget, ToolCallId};
use std::collections::btree_map::Entry;

pub(super) fn apply(
    state: &mut DurableProjection<'_>,
    event: &DomainEvent,
) -> Result<(), JournalError> {
    match event {
        DomainEvent::ExecutionCreated { execution, payload } => {
            let session = state.sessions.get(&execution.session_id).ok_or_else(|| {
                JournalError::InvalidEvent(format!(
                    "execution {} references unknown session {}",
                    execution.id, execution.session_id
                ))
            })?;
            if session.summary.state == phenix_core::SessionState::Closed {
                return Err(JournalError::InvalidEvent(format!(
                    "execution {} references closed session {}",
                    execution.id, execution.session_id
                )));
            }
            let expected_id =
                ExecutionId::parse(format!("execution-{}", *state.next_execution + 1))
                    .expect("generated execution id");
            if execution.id != expected_id {
                return Err(JournalError::InvalidEvent(format!(
                    "execution identity cursor mismatch: expected {expected_id}, found {}",
                    execution.id
                )));
            }
            let mut config_revision = session.summary.config_revision.clone();
            if let Some(parent_id) = &execution.parent_execution {
                let parent = state.executions.get(parent_id).ok_or_else(|| {
                    JournalError::InvalidEvent(format!(
                        "execution {} references unknown parent {parent_id}",
                        execution.id
                    ))
                })?;
                config_revision = parent.config_revision.clone();
                if let Some(callable) = execution.callable.as_ref() {
                    if !parent.authority.callables.contains(callable) {
                        return Err(JournalError::InvalidEvent(format!(
                            "execution {} callable {callable} is not delegated by parent {parent_id}",
                            execution.id
                        )));
                    }
                }
                if !parent.authority.permits(payload.authority()) {
                    return Err(JournalError::InvalidEvent(format!(
                        "execution {} authority exceeds parent {parent_id}",
                        execution.id
                    )));
                }
            }
            let materialized_payload = materialize_execution_payload(state, execution, payload);
            match state.executions.entry(execution.id.clone()) {
                Entry::Vacant(entry) => {
                    entry.insert(ExecutionRecord {
                        summary: execution.clone(),
                        payload: materialized_payload,
                        authority: payload.authority().clone(),
                        config_revision,
                        worker_profile: None,
                    });
                }
                Entry::Occupied(_) => {
                    return Err(JournalError::InvalidEvent(format!(
                        "duplicate execution id: {}",
                        execution.id
                    )));
                }
            }
            *state.next_execution += 1;
        }
        DomainEvent::WorkerProfileBound {
            execution_id,
            profile_id,
        } => {
            let execution = state.executions.get_mut(execution_id).ok_or_else(|| {
                JournalError::InvalidEvent(format!(
                    "worker profile binding references unknown execution {execution_id}"
                ))
            })?;
            if execution.worker_profile.is_some() {
                return Err(JournalError::InvalidEvent(format!(
                    "execution {execution_id} has more than one worker profile binding"
                )));
            }
            execution.worker_profile = Some(profile_id.clone());
        }
        DomainEvent::RootSubmissionAccepted {
            session_id,
            execution_id,
            ingress_order,
        } => {
            let execution = state.executions.get(execution_id).ok_or_else(|| {
                JournalError::InvalidEvent(format!(
                    "root ingress references unknown execution {execution_id}"
                ))
            })?;
            if execution.summary.session_id != *session_id
                || execution.summary.parent_execution.is_some()
                || execution.summary.state != ExecutionState::Pending
            {
                return Err(JournalError::InvalidEvent(format!(
                    "root ingress does not match pending root execution {execution_id}"
                )));
            }
            let expected = state
                .next_root_ingress
                .get(session_id)
                .copied()
                .unwrap_or(0)
                .saturating_add(1);
            if *ingress_order != expected {
                return Err(JournalError::InvalidEvent(format!(
                    "session {session_id} ingress order expected {expected}, found {ingress_order}"
                )));
            }
            if state
                .root_ingress
                .insert(execution_id.clone(), *ingress_order)
                .is_some()
            {
                return Err(JournalError::InvalidEvent(format!(
                    "root execution {execution_id} was accepted more than once"
                )));
            }
            state
                .next_root_ingress
                .insert(session_id.clone(), *ingress_order);
        }
        DomainEvent::ExecutionStateChanged {
            execution_id,
            state: next,
        } => {
            let execution = state.executions.get_mut(execution_id).ok_or_else(|| {
                JournalError::InvalidEvent(format!(
                    "state change references unknown execution {execution_id}"
                ))
            })?;
            if is_terminal(&execution.summary.state) {
                return Err(JournalError::InvalidEvent(format!(
                    "terminal execution {execution_id} cannot change state"
                )));
            }
            execution.summary.state = next.clone();
        }
        DomainEvent::InvocationResolved {
            execution_id,
            route,
        } => {
            let execution = state.executions.get(execution_id).ok_or_else(|| {
                JournalError::InvalidEvent(format!(
                    "resolved route references unknown execution {execution_id}"
                ))
            })?;
            if !matches!(&execution.payload, ExecutionPayload::Invocation { .. }) {
                return Err(JournalError::InvalidEvent(format!(
                    "resolved route references non-invocation execution {execution_id}"
                )));
            }
            if route.config_revision != execution.config_revision {
                return Err(JournalError::InvalidEvent(format!(
                    "resolved route for {execution_id} uses config revision {} instead of pinned {}",
                    route.config_revision, execution.config_revision
                )));
            }
            if route.requested_target != execution.summary.target {
                return Err(JournalError::InvalidEvent(format!(
                    "resolved route for {execution_id} does not match execution target"
                )));
            }
            if let ExecutionTarget::Fixed(expected) = &route.requested_target {
                if &route.model != expected {
                    return Err(JournalError::InvalidEvent(format!(
                        "resolved fixed route for {execution_id} does not match its requested model"
                    )));
                }
            }
            match state.resolved_routes.entry(execution_id.clone()) {
                Entry::Vacant(entry) => {
                    entry.insert(route.clone());
                }
                Entry::Occupied(_) => {
                    return Err(JournalError::InvalidEvent(format!(
                        "execution {execution_id} was resolved more than once"
                    )));
                }
            }
        }
        DomainEvent::FrontendEvent { event } => {
            let expected = *state.next_event + 1;
            if event.sequence != expected {
                return Err(JournalError::InvalidEvent(format!(
                    "frontend event sequence mismatch: expected {expected}, found {}",
                    event.sequence
                )));
            }
            let execution = state.executions.get(&event.execution_id).ok_or_else(|| {
                JournalError::InvalidEvent(format!(
                    "frontend event {} references unknown execution {}",
                    event.sequence, event.execution_id
                ))
            })?;
            if execution.summary.session_id != event.session_id {
                return Err(JournalError::InvalidEvent(format!(
                    "frontend event {} session does not match execution {}",
                    event.sequence, event.execution_id
                )));
            }
            if let ExecutionEventKind::ToolCallStarted { tool_call_id, .. } = &event.kind {
                let expected_id =
                    ToolCallId::parse(format!("tool-call-{}", *state.next_tool_call + 1))
                        .expect("generated tool call id");
                if *tool_call_id != expected_id {
                    return Err(JournalError::InvalidEvent(format!(
                        "tool-call identity cursor mismatch: expected {expected_id}, found {tool_call_id}"
                    )));
                }
                *state.next_tool_call += 1;
            }
            state.events.push(event.clone());
            *state.next_event = event.sequence;
        }
        _ => unreachable!("execution projection received unrelated event"),
    }
    Ok(())
}

fn is_terminal(state: &ExecutionState) -> bool {
    matches!(
        state,
        ExecutionState::Completed
            | ExecutionState::Failed
            | ExecutionState::Cancelled
            | ExecutionState::Interrupted
    )
}
