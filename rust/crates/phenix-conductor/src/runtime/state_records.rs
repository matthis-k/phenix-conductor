#[derive(Clone, Debug)]
struct SessionRecord {
    summary: SessionSummary,
}

#[derive(Clone, Debug)]
enum ExecutionPayload {
    Invocation { input: String },
    Orchestration { input: Value },
}

#[derive(Clone, Debug)]
struct ExecutionRecord {
    summary: ExecutionSummary,
    payload: ExecutionPayload,
    authority: ExecutionAuthority,
    config_revision: ConfigRevisionId,
    worker_profile: Option<WorkerProfileId>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct TerminalId(String);

impl TerminalId {
    pub fn parse(value: impl Into<String>) -> Result<Self, phenix_core::InvalidId> {
        let value = value.into();
        if value.trim().is_empty() { Err(phenix_core::InvalidId) } else { Ok(Self(value)) }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct JobId(String);

impl JobId {
    pub fn parse(value: impl Into<String>) -> Result<Self, phenix_core::InvalidId> {
        let value = value.into();
        if value.trim().is_empty() { Err(phenix_core::InvalidId) } else { Ok(Self(value)) }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DurableResourceOwner {
    Execution(ExecutionId),
    Workspace(WorkspaceId),
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DurableProcessState {
    Running,
    Exited { code: Option<i32> },
    Revoked,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TerminalRecord {
    pub id: TerminalId,
    pub owner: DurableResourceOwner,
    pub created_by: ExecutionId,
    pub authority: ExecutionAuthority,
    pub state: DurableProcessState,
    pub output_refs: Vec<phenix_core::ExactReference>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct JobRecord {
    pub id: JobId,
    pub owner: DurableResourceOwner,
    pub created_by: ExecutionId,
    pub authority: ExecutionAuthority,
    pub state: DurableProcessState,
    pub output_refs: Vec<phenix_core::ExactReference>,
}
