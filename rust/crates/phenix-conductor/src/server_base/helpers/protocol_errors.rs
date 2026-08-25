use crate::WorkerProfileError;

fn protocol_error(code: ErrorCode, message: impl Into<String>) -> ProtocolError {
    ProtocolError {
        code,
        message: message.into(),
        session_id: None,
        execution_id: None,
    }
}

fn map_backend_error(error: BackendError) -> ProtocolError {
    match error {
        BackendError::Unsupported(message) => {
            protocol_error(ErrorCode::UnsupportedCapability, message)
        }
        BackendError::Transport(message) => protocol_error(ErrorCode::BackendTransport, message),
        BackendError::Protocol(message) => protocol_error(ErrorCode::BackendProtocol, message),
        BackendError::ContextOverflow(message) => protocol_error(
            ErrorCode::BackendProtocol,
            format!("context overflow after bounded recovery: {message}"),
        ),
    }
}

fn map_conductor_error(error: ConductorError) -> ProtocolError {
    match error {
        ConductorError::UnknownSession(id) => {
            let mut error = protocol_error(ErrorCode::UnknownId, format!("unknown session: {id}"));
            error.session_id = Some(id);
            error
        }
        ConductorError::UnknownConfigRevision(id) => protocol_error(
            ErrorCode::UnknownId,
            format!("unknown configuration revision: {id}"),
        ),
        ConductorError::UnboundConfigRevision(id) => protocol_error(
            ErrorCode::InvalidRequest,
            format!("configuration revision is not bound in this process: {id}"),
        ),
        ConductorError::ConfigRevisionAlreadyBound(id) => protocol_error(
            ErrorCode::InvalidRequest,
            format!("configuration revision is already bound: {id}"),
        ),
        ConductorError::ConfigRevisionFingerprintMismatch {
            revision,
            expected,
            actual,
        } => protocol_error(
            ErrorCode::InvalidRequest,
            format!(
                "configuration revision fingerprint mismatch for {revision}: expected {expected}, found {actual}"
            ),
        ),
        ConductorError::IncompatibleSessionRebase {
            session_id,
            revision,
            reason,
        } => {
            let mut error = protocol_error(
                ErrorCode::InvalidRequest,
                format!(
                    "session {session_id} cannot rebase to configuration revision {revision}: {reason}"
                ),
            );
            error.session_id = Some(session_id);
            error
        }
        ConductorError::ClosedSession(id) => {
            let mut error = protocol_error(
                ErrorCode::InvalidRequest,
                format!("session is closed: {id}"),
            );
            error.session_id = Some(id);
            error
        }
        ConductorError::SessionHasActiveExecutions(id) => {
            let mut error = protocol_error(
                ErrorCode::InvalidRequest,
                format!("session has active executions and cannot close: {id}"),
            );
            error.session_id = Some(id);
            error
        }
        ConductorError::UnknownExecution(id) => {
            let mut error =
                protocol_error(ErrorCode::UnknownId, format!("unknown execution: {id}"));
            error.execution_id = Some(id);
            error
        }
        ConductorError::WorkspaceMismatch { expected, actual } => protocol_error(
            ErrorCode::InvalidRequest,
            format!("workspace binding mismatch: persisted {expected}, discovered {actual}"),
        ),
        ConductorError::EmptyInput => {
            protocol_error(ErrorCode::InvalidRequest, "input must not be empty")
        }
        ConductorError::InvalidExecutionData {
            execution_id,
            message,
        } => {
            let mut error = protocol_error(ErrorCode::InvalidRequest, message);
            error.execution_id = Some(execution_id);
            error
        }
        ConductorError::InvalidLifecycle(id) => {
            let mut error = protocol_error(
                ErrorCode::InvalidRequest,
                format!("invalid execution lifecycle: {id}"),
            );
            error.execution_id = Some(id);
            error
        }
        ConductorError::InvalidFailureDecision {
            parent_execution,
            failed_child,
        } => {
            let mut error = protocol_error(
                ErrorCode::InvalidRequest,
                format!(
                    "invalid failure decision for child {failed_child} of orchestration {parent_execution}"
                ),
            );
            error.execution_id = Some(parent_execution);
            error
        }
        ConductorError::FailureDecisionDenied {
            parent_execution,
            decider_execution,
        } => {
            let mut error = protocol_error(
                ErrorCode::PolicyDenied,
                format!(
                    "execution {decider_execution} may not decide failures for orchestration {parent_execution}"
                ),
            );
            error.execution_id = Some(decider_execution);
            error
        }
        ConductorError::DelegationDenied {
            parent_execution,
            callable,
        } => {
            let mut error = protocol_error(
                ErrorCode::PolicyDenied,
                format!("execution {parent_execution} may not delegate callable {callable}"),
            );
            error.execution_id = Some(parent_execution);
            error
        }
        ConductorError::NonModelExecution(id) => {
            let mut error = protocol_error(
                ErrorCode::UnsupportedCapability,
                format!("execution is not model-backed: {id}"),
            );
            error.execution_id = Some(id);
            error
        }
        ConductorError::NonProviderExecution(id) => {
            let mut error = protocol_error(
                ErrorCode::UnsupportedCapability,
                format!("execution is not provider-backed: {id}"),
            );
            error.execution_id = Some(id);
            error
        }
        ConductorError::PolicyDenied {
            execution_id,
            denial,
        } => {
            let mut error = protocol_error(ErrorCode::PolicyDenied, denial.message);
            error.execution_id = Some(execution_id);
            error
        }
        ConductorError::WorkerProfile(error) => match error {
            WorkerProfileError::Unknown(_) => {
                protocol_error(ErrorCode::UnknownId, error.to_string())
            }
            WorkerProfileError::InvalidId
            | WorkerProfileError::Duplicate(_)
            | WorkerProfileError::InvalidAgent { .. } => {
                protocol_error(ErrorCode::InvalidRequest, error.to_string())
            }
        },
        ConductorError::CallableRegistry(error) => {
            protocol_error(ErrorCode::InvalidRequest, error.to_string())
        }
        ConductorError::ExecutionProvider(error) => map_execution_provider_error(error),
        ConductorError::Journal(error) => {
            protocol_error(ErrorCode::BackendProtocol, error.to_string())
        }
        ConductorError::Routing(error) => {
            protocol_error(ErrorCode::RoutingFailure, error.to_string())
        }
        ConductorError::Context(error) => {
            protocol_error(ErrorCode::InvalidRequest, error.to_string())
        }
        ConductorError::Objective(error) => match error {
            ObjectiveError::UnknownObjective(_)
            | ObjectiveError::UnknownCriterion { .. }
            | ObjectiveError::UnknownExecution(_) => {
                protocol_error(ErrorCode::UnknownId, error.to_string())
            }
            ObjectiveError::MissingExecutionObjective(_)
            | ObjectiveError::InvalidStatement
            | ObjectiveError::DuplicateCriterion(_)
            | ObjectiveError::InvalidParent(_)
            | ObjectiveError::WrongWorkspace(_)
            | ObjectiveError::RootIsImmutable(_)
            | ObjectiveError::EnactedObjectiveIsImmutable(_)
            | ObjectiveError::InvalidTransition { .. }
            | ObjectiveError::MissingRequiredEvidence { .. }
            | ObjectiveError::InvalidEvidence => {
                protocol_error(ErrorCode::InvalidRequest, error.to_string())
            }
        },
        ConductorError::Plan(error) => match error {
            PlanError::UnknownPlan(_)
            | PlanError::UnknownStep { .. }
            | PlanError::UnknownExecution(_)
            | PlanError::UnknownObjective(_) => {
                protocol_error(ErrorCode::UnknownId, error.to_string())
            }
            PlanError::WrongWorkspace(_)
            | PlanError::EmptyPlan
            | PlanError::InvalidStep(_)
            | PlanError::DuplicateStep(_)
            | PlanError::InvalidDependency { .. }
            | PlanError::DependencyCycle
            | PlanError::WrongObjectiveWorkspace(_)
            | PlanError::EnactedPlanIsImmutable(_)
            | PlanError::DraftRevisionConflict { .. }
            | PlanError::InvalidTransition { .. }
            | PlanError::InvalidStepTransition { .. }
            | PlanError::IncompleteDependencies { .. }
            | PlanError::ExecutionAlreadyAssigned(_)
            | PlanError::InvalidCause
            | PlanError::InvalidSuccessor(_) => {
                protocol_error(ErrorCode::InvalidRequest, error.to_string())
            }
        },
        ConductorError::Backend(error) => map_backend_error(error),
    }
}