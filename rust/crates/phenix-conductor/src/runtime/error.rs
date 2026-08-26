#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ConfigRevisionFingerprint(String);

impl Display for ConfigRevisionFingerprint {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConductorError {
    UnknownSession(SessionId),
    UnknownConfigRevision(ConfigRevisionId),
    UnboundConfigRevision(ConfigRevisionId),
    ConfigRevisionAlreadyBound(ConfigRevisionId),
    ConfigRevisionFingerprintMismatch {
        revision: ConfigRevisionId,
        expected: ConfigRevisionFingerprint,
        actual: ConfigRevisionFingerprint,
    },
    IncompatibleSessionRebase {
        session_id: SessionId,
        revision: ConfigRevisionId,
        reason: String,
    },
    ClosedSession(SessionId),
    SessionHasActiveExecutions(SessionId),
    UnknownExecution(ExecutionId),
    WorkspaceMismatch {
        expected: WorkspaceId,
        actual: WorkspaceId,
    },
    EmptyInput,
    InvalidExecutionData {
        execution_id: ExecutionId,
        message: String,
    },
    InvalidLifecycle(ExecutionId),
    InvalidFailureDecision {
        parent_execution: ExecutionId,
        failed_child: ExecutionId,
    },
    FailureDecisionDenied {
        parent_execution: ExecutionId,
        decider_execution: ExecutionId,
    },
    DelegationDenied {
        parent_execution: ExecutionId,
        callable: CallableId,
    },
    NonModelExecution(ExecutionId),
    NonProviderExecution(ExecutionId),
    PolicyDenied {
        execution_id: ExecutionId,
        denial: PolicyDenial,
    },
    WorkerProfile(WorkerProfileError),
    CallableRegistry(CallableRegistryError),
    LifecycleHook(LifecycleHookError),
    ExecutionProvider(ExecutionProviderError),
    Journal(JournalError),
    Routing(RoutingRegistryError),
    Context(ContextError),
    Decision(DecisionError),
    Objective(ObjectiveError),
    Plan(PlanError),
    Backend(BackendError),
}

impl Display for ConductorError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownSession(id) => write!(f, "unknown session: {id}"),
            Self::UnknownConfigRevision(id) => write!(f, "unknown configuration revision: {id}"),
            Self::UnboundConfigRevision(id) => write!(f, "configuration revision is not bound in this process: {id}"),
            Self::ConfigRevisionAlreadyBound(id) => write!(f, "configuration revision is already bound: {id}"),
            Self::ConfigRevisionFingerprintMismatch {
                revision,
                expected,
                actual,
            } => write!(
                f,
                "configuration revision fingerprint mismatch for {revision}: expected {expected}, found {actual}"
            ),
            Self::IncompatibleSessionRebase {
                session_id,
                revision,
                reason,
            } => write!(
                f,
                "session {session_id} cannot rebase to configuration revision {revision}: {reason}"
            ),
            Self::ClosedSession(id) => write!(f, "session is closed: {id}"),
            Self::SessionHasActiveExecutions(id) => {
                write!(f, "session has active executions and cannot close: {id}")
            }
            Self::UnknownExecution(id) => write!(f, "unknown execution: {id}"),
            Self::WorkspaceMismatch { expected, actual } => write!(
                f,
                "workspace binding mismatch: persisted {expected}, discovered {actual}"
            ),
            Self::EmptyInput => f.write_str("input must not be empty"),
            Self::InvalidExecutionData {
                execution_id,
                message,
            } => write!(f, "execution {execution_id} has invalid typed data: {message}"),
            Self::InvalidLifecycle(id) => write!(f, "execution is not runnable: {id}"),
            Self::InvalidFailureDecision {
                parent_execution,
                failed_child,
            } => write!(
                f,
                "invalid failure decision for child {failed_child} of orchestration {parent_execution}"
            ),
            Self::FailureDecisionDenied {
                parent_execution,
                decider_execution,
            } => write!(
                f,
                "execution {decider_execution} may not decide failures for orchestration {parent_execution}"
            ),
            Self::DelegationDenied {
                parent_execution,
                callable,
            } => write!(
                f,
                "execution {parent_execution} may not delegate callable {callable}"
            ),
            Self::NonModelExecution(id) => {
                write!(f, "execution is not model-provider backed: {id}")
            }
            Self::NonProviderExecution(id) => {
                write!(f, "execution is not non-model-provider backed: {id}")
            }
            Self::PolicyDenied { denial, .. } => Display::fmt(denial, f),
            Self::WorkerProfile(error) => Display::fmt(error, f),
            Self::CallableRegistry(error) => Display::fmt(error, f),
            Self::LifecycleHook(error) => Display::fmt(error, f),
            Self::ExecutionProvider(error) => Display::fmt(error, f),
            Self::Journal(error) => Display::fmt(error, f),
            Self::Routing(error) => Display::fmt(error, f),
            Self::Context(error) => Display::fmt(error, f),
            Self::Decision(error) => Display::fmt(error, f),
            Self::Objective(error) => Display::fmt(error, f),
            Self::Plan(error) => Display::fmt(error, f),
            Self::Backend(error) => Display::fmt(error, f),
        }
    }
}

impl Error for ConductorError {}

impl From<BackendError> for ConductorError {
    fn from(value: BackendError) -> Self {
        Self::Backend(value)
    }
}

impl From<WorkerProfileError> for ConductorError {
    fn from(value: WorkerProfileError) -> Self {
        Self::WorkerProfile(value)
    }
}

impl From<CallableRegistryError> for ConductorError {
    fn from(value: CallableRegistryError) -> Self {
        Self::CallableRegistry(value)
    }
}

impl From<LifecycleHookError> for ConductorError {
    fn from(value: LifecycleHookError) -> Self {
        Self::LifecycleHook(value)
    }
}

impl From<ExecutionProviderError> for ConductorError {
    fn from(value: ExecutionProviderError) -> Self {
        Self::ExecutionProvider(value)
    }
}

impl From<JournalError> for ConductorError {
    fn from(value: JournalError) -> Self {
        Self::Journal(value)
    }
}

impl From<RoutingRegistryError> for ConductorError {
    fn from(value: RoutingRegistryError) -> Self {
        Self::Routing(value)
    }
}

impl From<ContextError> for ConductorError {
    fn from(value: ContextError) -> Self {
        Self::Context(value)
    }
}

impl From<DecisionError> for ConductorError {
    fn from(value: DecisionError) -> Self {
        Self::Decision(value)
    }
}

impl From<ObjectiveError> for ConductorError {
    fn from(value: ObjectiveError) -> Self {
        Self::Objective(value)
    }
}

impl From<PlanError> for ConductorError {
    fn from(value: PlanError) -> Self {
        Self::Plan(value)
    }
}
