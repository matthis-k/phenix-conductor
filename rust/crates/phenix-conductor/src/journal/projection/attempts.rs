use super::state::DurableProjection;
use crate::journal::{DomainEvent, JournalError};
use phenix_core::{AttemptGroupId, ExecutionKind, ExecutionState};

pub(super) fn apply(
    state: &mut DurableProjection<'_>,
    event: &DomainEvent,
) -> Result<(), JournalError> {
    match event {
        DomainEvent::AttemptGroupCreated { group } => {
            let expected_id =
                AttemptGroupId::parse(format!("attempt-group-{}", *state.next_attempt_group + 1))
                    .expect("generated attempt group id");
            if group.id != expected_id {
                return Err(JournalError::InvalidEvent(format!(
                    "attempt group identity cursor mismatch: expected {expected_id}, found {}",
                    group.id
                )));
            }
            if group.attempts.len() != 1 || group.failures.len() != 1 {
                return Err(JournalError::InvalidEvent(format!(
                    "new attempt group {} must contain exactly one failed first attempt",
                    group.id
                )));
            }
            let first_execution_id = &group.attempts[0];
            let first_failure = &group.failures[0];
            if first_failure.execution_id != *first_execution_id || first_failure.attempt != 1 {
                return Err(JournalError::InvalidEvent(format!(
                    "attempt group {} has an invalid first failure",
                    group.id
                )));
            }
            let execution = state.executions.get(first_execution_id).ok_or_else(|| {
                JournalError::InvalidEvent(format!(
                    "attempt group {} references unknown execution {first_execution_id}",
                    group.id
                ))
            })?;
            if execution.summary.kind != ExecutionKind::Agent
                || execution.summary.state != ExecutionState::Failed
                || execution.summary.parent_execution.as_ref() != Some(&group.parent_execution)
                || execution.summary.callable.as_ref() != Some(&group.callable)
            {
                return Err(JournalError::InvalidEvent(format!(
                    "attempt group {} does not match its first failed agent execution",
                    group.id
                )));
            }
            if state
                .attempt_groups
                .values()
                .any(|existing| existing.contains_execution(first_execution_id))
            {
                return Err(JournalError::InvalidEvent(format!(
                    "execution {first_execution_id} belongs to more than one attempt group"
                )));
            }
            if state
                .attempt_groups
                .insert(group.id.clone(), group.clone())
                .is_some()
            {
                return Err(JournalError::InvalidEvent(format!(
                    "duplicate attempt group id: {}",
                    group.id
                )));
            }
            *state.next_attempt_group += 1;
        }
        DomainEvent::AttemptFailureRecorded { group_id, failure } => {
            let group = state.attempt_groups.get(group_id).ok_or_else(|| {
                JournalError::InvalidEvent(format!(
                    "attempt failure references unknown group {group_id}"
                ))
            })?;
            if group.latest_execution() != Some(&failure.execution_id) {
                return Err(JournalError::InvalidEvent(format!(
                    "attempt failure for {} is not the latest execution in group {group_id}",
                    failure.execution_id
                )));
            }
            let expected_attempt = group
                .attempt_for_execution(&failure.execution_id)
                .expect("latest execution belongs to its attempt group");
            if failure.attempt != expected_attempt {
                return Err(JournalError::InvalidEvent(format!(
                    "attempt failure for {} uses number {}, expected {expected_attempt}",
                    failure.execution_id, failure.attempt
                )));
            }
            if group
                .failures
                .iter()
                .any(|existing| existing.execution_id == failure.execution_id)
            {
                return Err(JournalError::InvalidEvent(format!(
                    "attempt failure for {} was recorded more than once",
                    failure.execution_id
                )));
            }
            let execution = state.executions.get(&failure.execution_id).ok_or_else(|| {
                JournalError::InvalidEvent(format!(
                    "attempt failure references unknown execution {}",
                    failure.execution_id
                ))
            })?;
            if execution.summary.state != ExecutionState::Failed {
                return Err(JournalError::InvalidEvent(format!(
                    "attempt failure references non-failed execution {}",
                    failure.execution_id
                )));
            }
            state
                .attempt_groups
                .get_mut(group_id)
                .expect("validated attempt group exists")
                .record_failure(failure.clone());
        }
        DomainEvent::AttemptRetryStarted {
            group_id,
            execution_id,
        } => {
            let group = state.attempt_groups.get(group_id).ok_or_else(|| {
                JournalError::InvalidEvent(format!(
                    "attempt retry references unknown group {group_id}"
                ))
            })?;
            let previous = group.latest_execution().ok_or_else(|| {
                JournalError::InvalidEvent(format!("attempt group {group_id} has no attempts"))
            })?;
            if !group
                .failures
                .iter()
                .any(|failure| &failure.execution_id == previous)
            {
                return Err(JournalError::InvalidEvent(format!(
                    "attempt group {group_id} retried before its latest execution failed"
                )));
            }
            if state
                .attempt_groups
                .values()
                .any(|existing| existing.contains_execution(execution_id))
            {
                return Err(JournalError::InvalidEvent(format!(
                    "execution {execution_id} belongs to more than one attempt group"
                )));
            }
            let execution = state.executions.get(execution_id).ok_or_else(|| {
                JournalError::InvalidEvent(format!(
                    "attempt retry references unknown execution {execution_id}"
                ))
            })?;
            if execution.summary.kind != ExecutionKind::Agent
                || execution.summary.state != ExecutionState::Pending
                || execution.summary.parent_execution.as_ref() != Some(&group.parent_execution)
                || execution.summary.callable.as_ref() != Some(&group.callable)
            {
                return Err(JournalError::InvalidEvent(format!(
                    "retry execution {execution_id} does not match attempt group {group_id}"
                )));
            }
            state
                .attempt_groups
                .get_mut(group_id)
                .expect("validated attempt group exists")
                .record_retry(execution_id.clone());
        }
        _ => unreachable!("attempt projection received unrelated event"),
    }
    Ok(())
}
