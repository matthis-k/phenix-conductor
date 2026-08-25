use super::state::DurableProjection;
use crate::journal::{DomainEvent, JournalError};
use phenix_core::{ExecutionReadSet, ExecutionState, FilesystemAuthority};

pub(super) fn apply(
    state: &mut DurableProjection<'_>,
    event: &DomainEvent,
) -> Result<(), JournalError> {
    match event {
        DomainEvent::DiagnosticWritePatchCaptured { patch } => {
            let execution = state.executions.get(&patch.execution_id).ok_or_else(|| {
                JournalError::InvalidEvent(format!(
                    "diagnostic patch references unknown execution {}",
                    patch.execution_id
                ))
            })?;
            if execution.authority.filesystem != FilesystemAuthority::ReadOnly {
                return Err(JournalError::InvalidEvent(format!(
                    "diagnostic patch references writable execution {}",
                    patch.execution_id
                )));
            }
            state.diagnostic_write_patches.push(patch.clone());
        }
        DomainEvent::LanguageObservationRecorded { observation } => {
            let execution = state
                .executions
                .get(&observation.execution)
                .ok_or_else(|| {
                    JournalError::InvalidEvent(format!(
                        "language observation references unknown execution {}",
                        observation.execution
                    ))
                })?;
            let session = state
                .sessions
                .get(&execution.summary.session_id)
                .ok_or_else(|| {
                    JournalError::InvalidEvent(format!(
                        "language observation execution {} references unknown session {}",
                        observation.execution, execution.summary.session_id
                    ))
                })?;
            if session.summary.workspace_id != observation.workspace {
                return Err(JournalError::InvalidEvent(format!(
                    "language observation for {} uses workspace {} instead of {}",
                    observation.execution, observation.workspace, session.summary.workspace_id
                )));
            }
        }
        DomainEvent::ContextInjectionRecorded { injection } => {
            if !state.executions.contains_key(&injection.execution_id) {
                return Err(JournalError::InvalidEvent(format!(
                    "context injection references unknown execution {}",
                    injection.execution_id
                )));
            }
        }
        DomainEvent::WorkspaceCheckpointCaptured {
            execution_id,
            workspace_id,
            files: _,
        } => {
            let execution = state.executions.get(execution_id).ok_or_else(|| {
                JournalError::InvalidEvent(format!(
                    "workspace checkpoint references unknown execution {execution_id}"
                ))
            })?;
            if !matches!(
                execution.summary.state,
                ExecutionState::Pending | ExecutionState::Running
            ) {
                return Err(JournalError::InvalidEvent(format!(
                    "workspace checkpoint references inactive execution {execution_id}"
                )));
            }
            if execution.authority.filesystem != FilesystemAuthority::Write {
                return Err(JournalError::InvalidEvent(format!(
                    "workspace checkpoint references non-writer execution {execution_id}"
                )));
            }
            let session = state
                .sessions
                .get(&execution.summary.session_id)
                .ok_or_else(|| {
                    JournalError::InvalidEvent(format!(
                        "workspace checkpoint execution {execution_id} references unknown session {}",
                        execution.summary.session_id
                    ))
                })?;
            if session.summary.workspace_id != *workspace_id {
                return Err(JournalError::InvalidEvent(format!(
                    "workspace checkpoint for {execution_id} uses workspace {workspace_id} instead of {}",
                    session.summary.workspace_id
                )));
            }
        }
        DomainEvent::WorkspaceFileObserved {
            execution_id,
            observation,
        } => {
            let execution = state.executions.get(execution_id).ok_or_else(|| {
                JournalError::InvalidEvent(format!(
                    "workspace observation references unknown execution {execution_id}"
                ))
            })?;
            if execution.summary.state != ExecutionState::Running {
                return Err(JournalError::InvalidEvent(format!(
                    "workspace observation references non-running execution {execution_id}"
                )));
            }
            state
                .read_sets
                .entry(execution_id.clone())
                .or_insert_with(|| ExecutionReadSet::new(execution_id.clone()))
                .observe(observation.clone());
        }
        _ => unreachable!("observation projection received unrelated event"),
    }
    Ok(())
}
