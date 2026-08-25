fn termination_columns(cause: &ExecutionTerminationCause) -> (&'static str, &ExecutionId) {
    match cause {
        ExecutionTerminationCause::ExplicitCancellation {
            requested_execution,
        } => ("explicit_cancellation", requested_execution),
        ExecutionTerminationCause::AncestorFailure { failed_ancestor } => {
            ("ancestor_failure", failed_ancestor)
        }
    }
}

fn execution_kind_token(kind: &ExecutionKind) -> &'static str {
    match kind {
        ExecutionKind::Root => "root",
        ExecutionKind::Agent => "agent",
        ExecutionKind::Orchestration => "orchestration",
    }
}

fn execution_state_token(state: &ExecutionState) -> &'static str {
    match state {
        ExecutionState::Pending => "pending",
        ExecutionState::Running => "running",
        ExecutionState::Completed => "completed",
        ExecutionState::Failed => "failed",
        ExecutionState::Cancelled => "cancelled",
        ExecutionState::Interrupted => "interrupted",
    }
}

fn session_state_token(state: &SessionState) -> &'static str {
    match state {
        SessionState::Active => "active",
        SessionState::Closed => "closed",
    }
}

fn file_kind_token(kind: &FileKind) -> &'static str {
    match kind {
        FileKind::Regular => "regular",
        FileKind::Directory => "directory",
        FileKind::Symlink => "symlink",
        FileKind::Other => "other",
    }
}
