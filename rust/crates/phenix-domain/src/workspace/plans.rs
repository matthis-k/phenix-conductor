use crate::{ExecutionId, ObjectiveId, WorkspaceId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

domain_id_type!(PlanId);
domain_id_type!(PlanStepId);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanState {
    Draft,
    Active,
    Completed,
    Failed,
    Invalidated,
    Abandoned,
    Superseded,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStepState {
    Proposed,
    Committed,
    Active,
    Completed,
    Failed,
    Invalidated,
    Abandoned,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStepRevisability {
    Revisable,
    Fixed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: PlanStepId,
    pub description: String,
    pub state: PlanStepState,
    pub revisability: PlanStepRevisability,
    #[serde(default)]
    pub depends_on: BTreeSet<PlanStepId>,
    #[serde(default)]
    pub objective_refs: BTreeSet<ObjectiveId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlanRecord {
    pub id: PlanId,
    pub workspace: WorkspaceId,
    pub state: PlanState,
    pub revision: u64,
    #[serde(default)]
    pub objective_refs: BTreeSet<ObjectiveId>,
    pub supersedes: Option<PlanId>,
    pub steps: Vec<PlanStep>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExecutionPlanAssignment {
    pub execution_id: ExecutionId,
    pub plan_id: PlanId,
    pub step_id: PlanStepId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlanTransitionCause {
    AgentAction { execution_id: ExecutionId },
    ExecutionOutcome { execution_id: ExecutionId },
    EvidenceAssessment { evidence_ref: String },
    UserAction,
    Policy { description: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlanTransition {
    pub plan_id: PlanId,
    pub from: PlanState,
    pub to: PlanState,
    pub cause: PlanTransitionCause,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlanStepTransition {
    pub plan_id: PlanId,
    pub step_id: PlanStepId,
    pub from: PlanStepState,
    pub to: PlanStepState,
    pub cause: PlanTransitionCause,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_ids_reject_empty_values() {
        assert!(PlanId::parse(" ").is_err());
        assert!(PlanStepId::parse("").is_err());
        assert!(serde_json::from_str::<PlanId>("\" \"").is_err());
        assert!(serde_json::from_str::<PlanStepId>("\"\"").is_err());
    }

    #[test]
    fn plans_do_not_encode_execution_policy() {
        let plan = PlanRecord {
            id: PlanId::parse("plan-1").unwrap(),
            workspace: WorkspaceId::parse("workspace-1").unwrap(),
            state: PlanState::Draft,
            revision: 1,
            objective_refs: BTreeSet::new(),
            supersedes: None,
            steps: vec![PlanStep {
                id: PlanStepId::parse("step-1").unwrap(),
                description: "Implement the durable plan aggregate".to_owned(),
                state: PlanStepState::Proposed,
                revisability: PlanStepRevisability::Revisable,
                depends_on: BTreeSet::new(),
                objective_refs: BTreeSet::new(),
            }],
        };
        let value = serde_json::to_value(plan).unwrap();
        assert!(value.get("model").is_none());
        assert!(value.get("callable").is_none());
        assert!(value.get("authority").is_none());
        assert!(value.get("retry").is_none());
        assert!(value.get("timeout").is_none());
    }
}
