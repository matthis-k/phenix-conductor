use crate::{ExecutionId, ObjectiveCriterionId, ObjectiveId, WorkspaceId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ObjectiveOrigin {
    Root,
    Derived { parent: ObjectiveId },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectiveState {
    Draft,
    Active,
    Completed,
    Failed,
    Invalidated,
    Abandoned,
    Superseded,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObjectiveCriterion {
    pub id: ObjectiveCriterionId,
    pub description: String,
    pub required: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObjectiveRecord {
    pub id: ObjectiveId,
    pub workspace: WorkspaceId,
    pub origin: ObjectiveOrigin,
    pub statement: String,
    pub criteria: Vec<ObjectiveCriterion>,
    pub state: ObjectiveState,
    pub supersedes: Option<ObjectiveId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObjectiveCriterionEvidence {
    pub criterion_id: ObjectiveCriterionId,
    pub evidence_ref: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ObjectiveTransitionCause {
    UserIntent,
    AgentAction { execution_id: ExecutionId },
    ExecutionOutcome { execution_id: ExecutionId },
    EvidenceAssessment { evidence_ref: String },
    Policy { description: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObjectiveTransition {
    pub objective_id: ObjectiveId,
    pub from: ObjectiveState,
    pub to: ObjectiveState,
    pub cause: ObjectiveTransitionCause,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExecutionObjectiveAssignment {
    pub execution_id: ExecutionId,
    pub primary: ObjectiveId,
    #[serde(default)]
    pub supporting: BTreeSet<ObjectiveId>,
}

impl ObjectiveRecord {
    pub fn required_criteria(&self) -> impl Iterator<Item = &ObjectiveCriterion> {
        self.criteria.iter().filter(|criterion| criterion.required)
    }

    #[must_use]
    pub fn parent(&self) -> Option<&ObjectiveId> {
        match &self.origin {
            ObjectiveOrigin::Root => None,
            ObjectiveOrigin::Derived { parent } => Some(parent),
        }
    }

    #[must_use]
    pub fn is_enacted(&self) -> bool {
        self.state != ObjectiveState::Draft
    }
}
