use crate::{DecisionId, ExactReference, ExecutionId, ObjectiveId, WorkspaceId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionState {
    Draft,
    Recorded,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionApplicability {
    Applicable,
    Questionable,
    Invalidated,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DecisionCreator {
    User,
    Execution { execution_id: ExecutionId },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DecisionRelation {
    Supersedes { decision_id: DecisionId },
    Reverts { decision_id: DecisionId },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DecisionDraftInput {
    pub question: String,
    pub chosen_option: String,
    pub rationale: String,
    #[serde(default)]
    pub alternatives: Vec<String>,
    #[serde(default)]
    pub alternatives_not_considered_reason: Option<String>,
    #[serde(default)]
    pub evidence: Vec<ExactReference>,
    pub creator: DecisionCreator,
    #[serde(default)]
    pub objectives: BTreeSet<ObjectiveId>,
    #[serde(default)]
    pub dependencies: BTreeSet<DecisionId>,
    pub relation: Option<DecisionRelation>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub id: DecisionId,
    pub workspace: WorkspaceId,
    pub revision: u64,
    pub state: DecisionState,
    pub question: String,
    pub chosen_option: String,
    pub rationale: String,
    pub alternatives: Vec<String>,
    pub alternatives_not_considered_reason: Option<String>,
    pub evidence: Vec<ExactReference>,
    pub creator: DecisionCreator,
    pub objectives: BTreeSet<ObjectiveId>,
    pub dependencies: BTreeSet<DecisionId>,
    pub relation: Option<DecisionRelation>,
    pub applicability: DecisionApplicability,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecisionHistoryScope {
    ObjectiveLineage(ObjectiveId),
    Workspace,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionHistoryQuery {
    pub text: String,
    pub scope: DecisionHistoryScope,
    pub limit: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionHistoryMatch {
    pub decision_id: DecisionId,
    pub question: String,
    pub chosen_option: String,
    pub rationale: String,
    pub applicability: DecisionApplicability,
}
