use super::{DomainEvent, DurableProjection};
use crate::{DurableProcessState, DurableResourceOwner, JournalError};

pub(crate) fn apply(
    state: &mut DurableProjection<'_>,
    event: &DomainEvent,
) -> Result<(), JournalError> {
    match event {
        DomainEvent::TerminalCreated { terminal } => {
            if state.terminals.contains_key(&terminal.id) {
                return Err(JournalError::InvalidFormat(format!(
                    "duplicate terminal id: {}",
                    terminal.id.as_str()
                )));
            }
            validate_created_resource(
                "terminal",
                terminal.id.as_str(),
                &terminal.owner,
                &terminal.created_by,
                &terminal.state,
            )?;
            *state.next_terminal =
                (*state.next_terminal).max(id_ordinal(terminal.id.as_str(), "terminal-"));
            state
                .terminals
                .insert(terminal.id.clone(), terminal.clone());
        }
        DomainEvent::JobCreated { job } => {
            if state.jobs.contains_key(&job.id) {
                return Err(JournalError::InvalidFormat(format!(
                    "duplicate job id: {}",
                    job.id.as_str()
                )));
            }
            validate_created_resource(
                "job",
                job.id.as_str(),
                &job.owner,
                &job.created_by,
                &job.state,
            )?;
            *state.next_job = (*state.next_job).max(id_ordinal(job.id.as_str(), "job-"));
            state.jobs.insert(job.id.clone(), job.clone());
        }
        DomainEvent::TerminalStateChanged {
            terminal_id,
            state: next,
        } => {
            let terminal = state.terminals.get_mut(terminal_id).ok_or_else(|| {
                JournalError::InvalidFormat(format!("unknown terminal: {}", terminal_id.as_str()))
            })?;
            validate_state_transition("terminal", terminal_id.as_str(), &terminal.state, next)?;
            terminal.state = next.clone();
        }
        DomainEvent::JobStateChanged {
            job_id,
            state: next,
        } => {
            let job = state.jobs.get_mut(job_id).ok_or_else(|| {
                JournalError::InvalidFormat(format!("unknown job: {}", job_id.as_str()))
            })?;
            validate_state_transition("job", job_id.as_str(), &job.state, next)?;
            job.state = next.clone();
        }
        DomainEvent::JobPromoted {
            job_id,
            workspace_id,
        } => {
            let job = state.jobs.get_mut(job_id).ok_or_else(|| {
                JournalError::InvalidFormat(format!("unknown job: {}", job_id.as_str()))
            })?;
            if job.state != DurableProcessState::Running {
                return Err(JournalError::InvalidFormat(format!(
                    "cannot promote non-running job: {}",
                    job_id.as_str()
                )));
            }
            if !matches!(job.owner, DurableResourceOwner::Execution(_)) {
                return Err(JournalError::InvalidFormat(format!(
                    "job is already workspace-owned: {}",
                    job_id.as_str()
                )));
            }
            job.owner = DurableResourceOwner::Workspace(workspace_id.clone());
        }
        DomainEvent::TerminalOutputRecorded {
            terminal_id,
            output,
        } => {
            let terminal = state.terminals.get_mut(terminal_id).ok_or_else(|| {
                JournalError::InvalidFormat(format!("unknown terminal: {}", terminal_id.as_str()))
            })?;
            terminal.output_refs.push(output.clone());
        }
        DomainEvent::JobOutputRecorded { job_id, output } => {
            let job = state.jobs.get_mut(job_id).ok_or_else(|| {
                JournalError::InvalidFormat(format!("unknown job: {}", job_id.as_str()))
            })?;
            job.output_refs.push(output.clone());
        }
        _ => unreachable!("process resource projection received unrelated event"),
    }
    Ok(())
}

fn validate_created_resource(
    kind: &str,
    id: &str,
    owner: &DurableResourceOwner,
    created_by: &phenix_core::ExecutionId,
    state: &DurableProcessState,
) -> Result<(), JournalError> {
    if state != &DurableProcessState::Running {
        return Err(JournalError::InvalidFormat(format!(
            "{kind} must be created running: {id}"
        )));
    }
    if owner != &DurableResourceOwner::Execution(created_by.clone()) {
        return Err(JournalError::InvalidFormat(format!(
            "{kind} must be execution-owned at creation: {id}"
        )));
    }
    Ok(())
}

fn validate_state_transition(
    kind: &str,
    id: &str,
    current: &DurableProcessState,
    next: &DurableProcessState,
) -> Result<(), JournalError> {
    if current != &DurableProcessState::Running
        || !matches!(
            next,
            DurableProcessState::Exited { .. } | DurableProcessState::Revoked
        )
    {
        return Err(JournalError::InvalidFormat(format!(
            "invalid {kind} state transition for {id}: {current:?} -> {next:?}"
        )));
    }
    Ok(())
}

fn id_ordinal(value: &str, prefix: &str) -> u64 {
    value
        .strip_prefix(prefix)
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
}
