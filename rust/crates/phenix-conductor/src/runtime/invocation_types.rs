#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedInvocation {
    pub execution_id: ExecutionId,
    pub session_id: SessionId,
    pub config_revision: ConfigRevisionId,
    pub callable: Option<CallableId>,
    pub requested_target: ExecutionTarget,
    pub model: ModelTarget,
    pub prompt: String,
    pub context_accounting: ContextProjectionAccounting,
    pub tools: ToolProvision,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreparedInvocation {
    pub resolved: ResolvedInvocation,
    pub tools: PreparedToolSurface,
}

impl PreparedInvocation {
    #[must_use]
    pub fn backend_session_request(&self) -> BackendSessionRequest {
        BackendSessionRequest {
            model: self.resolved.model.clone(),
            tools: self.tools.clone(),
        }
    }

    #[must_use]
    pub fn backend_execution_request(&self) -> BackendExecutionRequest {
        BackendExecutionRequest {
            execution_id: self.resolved.execution_id.clone(),
            prompt: self.resolved.prompt.clone(),
        }
    }

    #[must_use]
    pub fn allowed_tools(&self) -> BTreeSet<CallableId> {
        self.tools
            .callables()
            .iter()
            .map(|descriptor| descriptor.id.clone())
            .collect()
    }
}
