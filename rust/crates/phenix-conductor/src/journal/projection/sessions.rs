use super::state::DurableProjection;
use crate::journal::{DomainEvent, JournalError};
use crate::{ConfigRevisionSlot, SessionRecord};
use phenix_core::{ConfigRevisionId, ExecutionState, SessionId, SessionState};
use std::collections::btree_map::Entry;

pub(super) fn apply(
    state: &mut DurableProjection<'_>,
    event: &DomainEvent,
) -> Result<(), JournalError> {
    match event {
        DomainEvent::ConfigurationRevisionActivated {
            revision,
            fingerprint,
        } => {
            let expected =
                ConfigRevisionId::parse(format!("config-{}", *state.next_config_revision + 1))
                    .expect("generated config revision id");
            if revision != &expected || state.config_revisions.contains_key(revision) {
                return Err(JournalError::InvalidEvent(format!(
                    "configuration revision activation expected {expected}, found {revision}"
                )));
            }
            state.config_revisions.insert(
                revision.clone(),
                ConfigRevisionSlot {
                    fingerprint: fingerprint.clone(),
                    configuration: None,
                    ordinal: *state.next_config_revision + 1,
                },
            );
            *state.current_config_revision = revision.clone();
            *state.next_config_revision += 1;
        }
        DomainEvent::SessionCreated { session } => {
            if !state
                .config_revisions
                .contains_key(&session.config_revision)
            {
                return Err(JournalError::InvalidEvent(format!(
                    "session {} references unknown config revision {}",
                    session.id, session.config_revision
                )));
            }
            if session.state != SessionState::Active {
                return Err(JournalError::InvalidEvent(format!(
                    "new session {} must start active",
                    session.id
                )));
            }
            let expected_id = SessionId::parse(format!("session-{}", *state.next_session + 1))
                .expect("generated session id");
            if session.id != expected_id {
                return Err(JournalError::InvalidEvent(format!(
                    "session identity cursor mismatch: expected {expected_id}, found {}",
                    session.id
                )));
            }
            if let Some(parent) = &session.parent_session {
                let parent = state.sessions.get(parent).ok_or_else(|| {
                    JournalError::InvalidEvent(format!(
                        "session {} references unknown parent {parent}",
                        session.id
                    ))
                })?;
                if parent.summary.workspace_id != session.workspace_id {
                    return Err(JournalError::InvalidEvent(format!(
                        "session {} workspace {} does not match parent workspace {}",
                        session.id, session.workspace_id, parent.summary.workspace_id
                    )));
                }
            } else if let Some(existing) = state.sessions.values().next() {
                if existing.summary.workspace_id != session.workspace_id {
                    return Err(JournalError::InvalidEvent(format!(
                        "root session {} workspace {} does not match runtime workspace {}",
                        session.id, session.workspace_id, existing.summary.workspace_id
                    )));
                }
            }
            match state.sessions.entry(session.id.clone()) {
                Entry::Vacant(entry) => {
                    entry.insert(SessionRecord {
                        summary: session.clone(),
                    });
                }
                Entry::Occupied(_) => {
                    return Err(JournalError::InvalidEvent(format!(
                        "duplicate session id: {}",
                        session.id
                    )));
                }
            }
            *state.next_session += 1;
        }
        DomainEvent::SessionConfigRebased {
            session_id,
            config_revision,
        } => {
            let target_ordinal = state
                .config_revisions
                .get(config_revision)
                .ok_or_else(|| {
                    JournalError::InvalidEvent(format!(
                        "session {session_id} rebase references unknown config revision {config_revision}"
                    ))
                })?
                .ordinal;
            let session = state.sessions.get(session_id).ok_or_else(|| {
                JournalError::InvalidEvent(format!(
                    "rebase references unknown session {session_id}"
                ))
            })?;
            if session.summary.state == SessionState::Closed {
                return Err(JournalError::InvalidEvent(format!(
                    "closed session {session_id} cannot be rebased"
                )));
            }
            let current_ordinal = state.config_revisions[&session.summary.config_revision].ordinal;
            if target_ordinal <= current_ordinal {
                return Err(JournalError::InvalidEvent(format!(
                    "session {session_id} cannot rebase from {} to non-newer revision {config_revision}",
                    session.summary.config_revision
                )));
            }
            let session = state
                .sessions
                .get_mut(session_id)
                .expect("validated session exists");
            session.summary.config_revision = config_revision.clone();
        }
        DomainEvent::SessionRenamed { session_id, name } => {
            let session = state.sessions.get_mut(session_id).ok_or_else(|| {
                JournalError::InvalidEvent(format!(
                    "rename references unknown session {session_id}"
                ))
            })?;
            if session.summary.state == SessionState::Closed {
                return Err(JournalError::InvalidEvent(format!(
                    "closed session {session_id} cannot be renamed"
                )));
            }
            session.summary.name = Some(name.clone());
        }
        DomainEvent::SessionTargetChanged { session_id, target } => {
            let session = state.sessions.get_mut(session_id).ok_or_else(|| {
                JournalError::InvalidEvent(format!(
                    "target change references unknown session {session_id}"
                ))
            })?;
            if session.summary.state == SessionState::Closed {
                return Err(JournalError::InvalidEvent(format!(
                    "closed session {session_id} cannot change target"
                )));
            }
            session.summary.default_target = target.clone();
        }
        DomainEvent::SessionClosed { session_id } => {
            let session = state.sessions.get_mut(session_id).ok_or_else(|| {
                JournalError::InvalidEvent(format!("close references unknown session {session_id}"))
            })?;
            if session.summary.state == SessionState::Closed {
                return Err(JournalError::InvalidEvent(format!(
                    "session {session_id} was closed more than once"
                )));
            }
            if state.executions.values().any(|execution| {
                execution.summary.session_id == *session_id
                    && !is_terminal(&execution.summary.state)
            }) {
                return Err(JournalError::InvalidEvent(format!(
                    "session {session_id} cannot close with active executions"
                )));
            }
            session.summary.state = SessionState::Closed;
        }
        _ => unreachable!("session projection received unrelated event"),
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
