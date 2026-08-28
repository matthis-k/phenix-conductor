use crate::ExecutionId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FailureAttemptSummary {
    pub execution_id: ExecutionId,
    pub attempt: u32,
    pub approach: String,
    pub failure_at: String,
    pub reason: String,
    pub completed_work: Vec<String>,
}
