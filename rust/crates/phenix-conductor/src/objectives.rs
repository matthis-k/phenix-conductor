use crate::{
    ConductorError, ConductorRuntime, DomainEvent, JournalError, JournalExecutionPayload,
    RuntimeJournal,
};
use phenix_core::{
    ExecutionId, ExecutionObjectiveAssignment, ObjectiveCriterion, ObjectiveCriterionEvidence,
    ObjectiveId, ObjectiveOrigin, ObjectiveRecord, ObjectiveState, ObjectiveTransition,
    ObjectiveTransitionCause,
};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObjectiveError {
    UnknownObjective(ObjectiveId),
    UnknownCriterion {
        objective_id: ObjectiveId,
        criterion_id: phenix_core::ObjectiveCriterionId,
    },
    UnknownExecution(ExecutionId),
    MissingExecutionObjective(ExecutionId),
    InvalidStatement,
    DuplicateCriterion(phenix_core::ObjectiveCriterionId),
    InvalidParent(ObjectiveId),
    WrongWorkspace(ObjectiveId),
    RootIsImmutable(ObjectiveId),
    EnactedObjectiveIsImmutable(ObjectiveId),
    InvalidTransition {
        objective_id: ObjectiveId,
        from: ObjectiveState,
        to: ObjectiveState,
    },
    MissingRequiredEvidence {
        objective_id: ObjectiveId,
        criteria: Vec<phenix_core::ObjectiveCriterionId>,
    },
    InvalidEvidence,
}

impl Display for ObjectiveError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownObjective(id) => write!(f, "unknown objective: {id}"),
            Self::UnknownCriterion {
                objective_id,
                criterion_id,
            } => write!(
                f,
                "objective {objective_id} has no criterion {criterion_id}"
            ),
            Self::UnknownExecution(id) => write!(f, "unknown execution: {id}"),
            Self::MissingExecutionObjective(id) => {
                write!(f, "execution has no primary objective: {id}")
            }
            Self::InvalidStatement => f.write_str("objective statement must not be empty"),
            Self::DuplicateCriterion(id) => write!(f, "duplicate objective criterion: {id}"),
            Self::InvalidParent(id) => write!(f, "objective parent is not active: {id}"),
            Self::WrongWorkspace(id) => write!(f, "objective belongs to another workspace: {id}"),
            Self::RootIsImmutable(id) => write!(f, "root objective is immutable: {id}"),
            Self::EnactedObjectiveIsImmutable(id) => {
                write!(f, "enacted objective is immutable: {id}")
            }
            Self::InvalidTransition {
                objective_id,
                from,
                to,
            } => write!(
                f,
                "objective {objective_id} cannot transition from {from:?} to {to:?}"
            ),
            Self::MissingRequiredEvidence {
                objective_id,
                criteria,
            } => write!(
                f,
                "objective {objective_id} cannot complete without evidence for {criteria:?}"
            ),
            Self::InvalidEvidence => {
                f.write_str("objective evidence or transition cause must not be empty")
            }
        }
    }
}

impl Error for ObjectiveError {}

#[derive(Default)]
struct ObjectiveProjection {
    active: bool,
    objectives: BTreeMap<ObjectiveId, ObjectiveRecord>,
    evidence: BTreeMap<(ObjectiveId, phenix_core::ObjectiveCriterionId), Vec<String>>,
    assignments: BTreeMap<ExecutionId, ExecutionObjectiveAssignment>,
    known_executions: BTreeSet<ExecutionId>,
    required_assignments: BTreeSet<ExecutionId>,
}

impl ObjectiveProjection {
    fn apply(&mut self, event: &DomainEvent) -> Result<(), JournalError> {
        match event {
            DomainEvent::ExecutionCreated { execution, .. } => {
                if self.active {
                    self.required_assignments.insert(execution.id.clone());
                }
                self.known_executions.insert(execution.id.clone());
            }
            DomainEvent::ObjectiveSemanticsActivated => {
                if self.active {
                    return Err(invalid_event(
                        "objective semantics were activated more than once",
                    ));
                }
                self.active = true;
                self.required_assignments
                    .extend(self.known_executions.iter().cloned());
            }
            DomainEvent::ObjectiveCreated { objective } => {
                self.require_active()?;
                validate_record(objective)?;
                let expected =
                    ObjectiveId::parse(format!("objective-{}", self.objectives.len() + 1))
                        .expect("generated objective id");
                if objective.id != expected || self.objectives.contains_key(&objective.id) {
                    return Err(invalid_event(format!(
                        "objective identity cursor mismatch: expected {expected}, found {}",
                        objective.id
                    )));
                }
                match &objective.origin {
                    ObjectiveOrigin::Root => {
                        if objective.state != ObjectiveState::Active {
                            return Err(invalid_event(format!(
                                "root objective {} must be active when created",
                                objective.id
                            )));
                        }
                    }
                    ObjectiveOrigin::Derived { parent } => {
                        let parent = self.objectives.get(parent).ok_or_else(|| {
                            invalid_event(format!(
                                "derived objective {} references unknown parent {parent}",
                                objective.id
                            ))
                        })?;
                        if parent.workspace != objective.workspace {
                            return Err(invalid_event(format!(
                                "derived objective {} crosses workspace ownership",
                                objective.id
                            )));
                        }
                        if objective.state != ObjectiveState::Draft {
                            return Err(invalid_event(format!(
                                "derived objective {} must start as a draft",
                                objective.id
                            )));
                        }
                    }
                }
                if let Some(superseded) = objective.supersedes.as_ref() {
                    if !matches!(objective.origin, ObjectiveOrigin::Root) {
                        return Err(invalid_event(format!(
                            "derived objective {} cannot supersede another objective",
                            objective.id
                        )));
                    }
                    let previous = self.objectives.get(superseded).ok_or_else(|| {
                        invalid_event(format!(
                            "objective {} supersedes unknown objective {superseded}",
                            objective.id
                        ))
                    })?;
                    if previous.workspace != objective.workspace {
                        return Err(invalid_event(format!(
                            "objective {} supersedes an objective in another workspace",
                            objective.id
                        )));
                    }
                    if !matches!(previous.origin, ObjectiveOrigin::Root) {
                        return Err(invalid_event(format!(
                            "root objective {} cannot supersede derived objective {superseded}",
                            objective.id
                        )));
                    }
                }
                self.objectives
                    .insert(objective.id.clone(), objective.clone());
            }
            DomainEvent::ObjectiveDraftRevised { objective } => {
                self.require_active()?;
                validate_record(objective)?;
                let previous = self.objectives.get(&objective.id).ok_or_else(|| {
                    invalid_event(format!(
                        "draft revision references unknown objective {}",
                        objective.id
                    ))
                })?;
                if matches!(previous.origin, ObjectiveOrigin::Root) {
                    return Err(invalid_event(format!(
                        "root objective {} cannot be revised",
                        objective.id
                    )));
                }
                if previous.state != ObjectiveState::Draft
                    || objective.state != ObjectiveState::Draft
                {
                    return Err(invalid_event(format!(
                        "only a prospective draft objective may be revised: {}",
                        objective.id
                    )));
                }
                if previous.workspace != objective.workspace
                    || previous.origin != objective.origin
                    || previous.supersedes != objective.supersedes
                {
                    return Err(invalid_event(format!(
                        "draft revision changed immutable ownership for objective {}",
                        objective.id
                    )));
                }
                self.objectives
                    .insert(objective.id.clone(), objective.clone());
            }
            DomainEvent::ObjectiveEvidenceRecorded {
                objective_id,
                evidence,
            } => {
                self.require_active()?;
                let objective = self.objectives.get(objective_id).ok_or_else(|| {
                    invalid_event(format!(
                        "evidence references unknown objective {objective_id}"
                    ))
                })?;
                if objective.state != ObjectiveState::Active {
                    return Err(invalid_event(format!(
                        "evidence may only be recorded for active objective {objective_id}"
                    )));
                }
                if !objective
                    .criteria
                    .iter()
                    .any(|criterion| criterion.id == evidence.criterion_id)
                {
                    return Err(invalid_event(format!(
                        "evidence references unknown criterion {} on objective {objective_id}",
                        evidence.criterion_id
                    )));
                }
                if evidence.evidence_ref.trim().is_empty() {
                    return Err(invalid_event("objective evidence reference is empty"));
                }
                self.evidence
                    .entry((objective_id.clone(), evidence.criterion_id.clone()))
                    .or_default()
                    .push(evidence.evidence_ref.clone());
            }
            DomainEvent::ObjectiveStateChanged { transition } => {
                self.require_active()?;
                validate_replayed_transition_cause(&transition.cause, &self.known_executions)?;
                let objective = self
                    .objectives
                    .get_mut(&transition.objective_id)
                    .ok_or_else(|| {
                        invalid_event(format!(
                            "state transition references unknown objective {}",
                            transition.objective_id
                        ))
                    })?;
                if objective.state != transition.from
                    || !allowed_transition(&transition.from, &transition.to)
                {
                    return Err(invalid_event(format!(
                        "invalid objective transition for {}: {:?} -> {:?}",
                        transition.objective_id, transition.from, transition.to
                    )));
                }
                if transition.to == ObjectiveState::Completed {
                    let missing = objective
                        .required_criteria()
                        .filter(|criterion| {
                            !self
                                .evidence
                                .contains_key(&(objective.id.clone(), criterion.id.clone()))
                        })
                        .map(|criterion| criterion.id.to_string())
                        .collect::<Vec<_>>();
                    if !missing.is_empty() {
                        return Err(invalid_event(format!(
                            "objective {} completed without required evidence for {missing:?}",
                            objective.id
                        )));
                    }
                }
                objective.state = transition.to.clone();
            }
            DomainEvent::ExecutionObjectivesAssigned { assignment } => {
                self.require_active()?;
                if !self.known_executions.contains(&assignment.execution_id) {
                    return Err(invalid_event(format!(
                        "objective assignment references unknown execution {}",
                        assignment.execution_id
                    )));
                }
                let primary = self.objectives.get(&assignment.primary).ok_or_else(|| {
                    invalid_event(format!(
                        "execution {} references unknown primary objective {}",
                        assignment.execution_id, assignment.primary
                    ))
                })?;
                if primary.state != ObjectiveState::Active {
                    return Err(invalid_event(format!(
                        "execution {} primary objective {} is not active",
                        assignment.execution_id, assignment.primary
                    )));
                }
                if assignment.supporting.contains(&assignment.primary) {
                    return Err(invalid_event(format!(
                        "execution {} lists its primary objective as supporting",
                        assignment.execution_id
                    )));
                }
                for supporting in &assignment.supporting {
                    let supporting = self.objectives.get(supporting).ok_or_else(|| {
                        invalid_event(format!(
                            "execution {} references unknown supporting objective {supporting}",
                            assignment.execution_id
                        ))
                    })?;
                    if supporting.state != ObjectiveState::Active {
                        return Err(invalid_event(format!(
                            "execution {} supporting objective {} is not active",
                            assignment.execution_id, supporting.id
                        )));
                    }
                }
                if self
                    .assignments
                    .insert(assignment.execution_id.clone(), assignment.clone())
                    .is_some()
                {
                    return Err(invalid_event(format!(
                        "execution {} received more than one objective assignment",
                        assignment.execution_id
                    )));
                }
                self.required_assignments.remove(&assignment.execution_id);
            }
            _ => {}
        }
        Ok(())
    }

    fn require_active(&self) -> Result<(), JournalError> {
        if self.active {
            Ok(())
        } else {
            Err(invalid_event(
                "objective semantic event occurred before objective semantics activation",
            ))
        }
    }
}

pub(crate) fn validate_journal_objectives(journal: &RuntimeJournal) -> Result<(), JournalError> {
    let mut projection = ObjectiveProjection::default();
    for entry in &journal.entries {
        projection.apply(&entry.event)?;
    }
    if projection.active && !projection.required_assignments.is_empty() {
        return Err(invalid_event(format!(
            "executions without primary objectives: {:?}",
            projection.required_assignments
        )));
    }
    Ok(())
}

impl ConductorRuntime {
    fn objective_projection(&self) -> Result<ObjectiveProjection, ConductorError> {
        let mut projection = ObjectiveProjection::default();
        for entry in &self.journal.entries {
            projection.apply(&entry.event)?;
        }
        Ok(projection)
    }

    pub(crate) fn ensure_objective_semantics_active(&mut self) -> Result<(), ConductorError> {
        if self
            .journal
            .entries
            .iter()
            .any(|entry| matches!(entry.event, DomainEvent::ObjectiveSemanticsActivated))
        {
            return Ok(());
        }

        let legacy_executions = self
            .journal
            .entries
            .iter()
            .filter_map(|entry| match &entry.event {
                DomainEvent::ExecutionCreated { execution, .. } => Some(execution.id.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        self.record_domain_event(DomainEvent::ObjectiveSemanticsActivated)?;
        for execution_id in legacy_executions {
            self.backfill_execution_objective(&execution_id)?;
        }
        Ok(())
    }

    fn backfill_execution_objective(
        &mut self,
        execution_id: &ExecutionId,
    ) -> Result<ExecutionObjectiveAssignment, ConductorError> {
        if let Some(existing) = self.execution_objectives(execution_id)? {
            return Ok(existing);
        }
        let execution = self
            .executions
            .get(execution_id)
            .ok_or_else(|| ObjectiveError::UnknownExecution(execution_id.clone()))?
            .summary
            .clone();
        let assignment = if let Some(parent) = execution.parent_execution {
            let parent = self.backfill_execution_objective(&parent)?;
            ExecutionObjectiveAssignment {
                execution_id: execution_id.clone(),
                primary: parent.primary,
                supporting: parent.supporting,
            }
        } else {
            let statement = self.root_execution_input(execution_id)?;
            let objective =
                self.create_root_objective_from_user_intent(statement, Vec::new(), None)?;
            ExecutionObjectiveAssignment {
                execution_id: execution_id.clone(),
                primary: objective.id,
                supporting: BTreeSet::new(),
            }
        };
        self.record_domain_event(DomainEvent::ExecutionObjectivesAssigned {
            assignment: assignment.clone(),
        })?;
        Ok(assignment)
    }

    fn root_execution_input(&self, execution_id: &ExecutionId) -> Result<String, ConductorError> {
        self.journal
            .entries
            .iter()
            .find_map(|entry| match &entry.event {
                DomainEvent::ExecutionCreated { execution, payload }
                    if execution.id == *execution_id =>
                {
                    match payload {
                        JournalExecutionPayload::Invocation { input, .. } => Some(input.clone()),
                        JournalExecutionPayload::Orchestration { input, .. } => Some(
                            serde_json::to_string(input)
                                .expect("orchestration input is JSON serializable"),
                        ),
                    }
                }
                _ => None,
            })
            .ok_or_else(|| ObjectiveError::UnknownExecution(execution_id.clone()).into())
    }

    pub fn objectives(&self) -> Result<Vec<ObjectiveRecord>, ConductorError> {
        Ok(self
            .objective_projection()?
            .objectives
            .into_values()
            .collect())
    }

    pub fn objective(&self, objective_id: &ObjectiveId) -> Result<ObjectiveRecord, ConductorError> {
        self.objective_projection()?
            .objectives
            .get(objective_id)
            .cloned()
            .ok_or_else(|| ObjectiveError::UnknownObjective(objective_id.clone()).into())
    }

    pub fn execution_objectives(
        &self,
        execution_id: &ExecutionId,
    ) -> Result<Option<ExecutionObjectiveAssignment>, ConductorError> {
        Ok(self
            .objective_projection()?
            .assignments
            .get(execution_id)
            .cloned())
    }

    pub fn create_root_objective_from_user_intent(
        &mut self,
        statement: impl Into<String>,
        criteria: Vec<ObjectiveCriterion>,
        supersedes: Option<ObjectiveId>,
    ) -> Result<ObjectiveRecord, ConductorError> {
        self.ensure_objective_semantics_active()?;
        let projection = self.objective_projection()?;
        let statement = statement.into();
        validate_input(&statement, &criteria)?;
        if let Some(previous) = supersedes.as_ref() {
            let previous = projection
                .objectives
                .get(previous)
                .ok_or_else(|| ObjectiveError::UnknownObjective(previous.clone()))?;
            if previous.workspace != self.workspace_id {
                return Err(ObjectiveError::WrongWorkspace(previous.id.clone()).into());
            }
            if !matches!(previous.origin, ObjectiveOrigin::Root) {
                return Err(ObjectiveError::InvalidParent(previous.id.clone()).into());
            }
        }
        let objective = ObjectiveRecord {
            id: next_objective_id(&projection),
            workspace: self.workspace_id.clone(),
            origin: ObjectiveOrigin::Root,
            statement,
            criteria,
            state: ObjectiveState::Active,
            supersedes: supersedes.clone(),
        };
        self.record_domain_event(DomainEvent::ObjectiveCreated {
            objective: objective.clone(),
        })?;
        if let Some(previous) = supersedes {
            let previous = self.objective(&previous)?;
            if previous.state == ObjectiveState::Active {
                self.transition_objective(
                    &previous.id,
                    ObjectiveState::Superseded,
                    ObjectiveTransitionCause::UserIntent,
                )?;
            }
        }
        Ok(objective)
    }

    pub fn create_derived_objective(
        &mut self,
        parent: &ObjectiveId,
        statement: impl Into<String>,
        criteria: Vec<ObjectiveCriterion>,
    ) -> Result<ObjectiveRecord, ConductorError> {
        self.ensure_objective_semantics_active()?;
        let projection = self.objective_projection()?;
        let parent_record = projection
            .objectives
            .get(parent)
            .ok_or_else(|| ObjectiveError::UnknownObjective(parent.clone()))?;
        if parent_record.workspace != self.workspace_id {
            return Err(ObjectiveError::WrongWorkspace(parent.clone()).into());
        }
        if parent_record.state != ObjectiveState::Active {
            return Err(ObjectiveError::InvalidParent(parent.clone()).into());
        }
        let statement = statement.into();
        validate_input(&statement, &criteria)?;
        let objective = ObjectiveRecord {
            id: next_objective_id(&projection),
            workspace: self.workspace_id.clone(),
            origin: ObjectiveOrigin::Derived {
                parent: parent.clone(),
            },
            statement,
            criteria,
            state: ObjectiveState::Draft,
            supersedes: None,
        };
        self.record_domain_event(DomainEvent::ObjectiveCreated {
            objective: objective.clone(),
        })?;
        Ok(objective)
    }

    pub fn revise_objective_draft(
        &mut self,
        objective_id: &ObjectiveId,
        statement: impl Into<String>,
        criteria: Vec<ObjectiveCriterion>,
    ) -> Result<ObjectiveRecord, ConductorError> {
        let mut objective = self.objective(objective_id)?;
        if matches!(objective.origin, ObjectiveOrigin::Root) {
            return Err(ObjectiveError::RootIsImmutable(objective_id.clone()).into());
        }
        if objective.state != ObjectiveState::Draft {
            return Err(ObjectiveError::EnactedObjectiveIsImmutable(objective_id.clone()).into());
        }
        let statement = statement.into();
        validate_input(&statement, &criteria)?;
        objective.statement = statement;
        objective.criteria = criteria;
        self.record_domain_event(DomainEvent::ObjectiveDraftRevised {
            objective: objective.clone(),
        })?;
        Ok(objective)
    }

    pub fn activate_objective(
        &mut self,
        objective_id: &ObjectiveId,
        cause: ObjectiveTransitionCause,
    ) -> Result<ObjectiveRecord, ConductorError> {
        self.transition_objective(objective_id, ObjectiveState::Active, cause)
    }

    pub fn record_objective_evidence(
        &mut self,
        objective_id: &ObjectiveId,
        evidence: ObjectiveCriterionEvidence,
    ) -> Result<(), ConductorError> {
        let objective = self.objective(objective_id)?;
        if objective.state != ObjectiveState::Active {
            return Err(ObjectiveError::InvalidTransition {
                objective_id: objective_id.clone(),
                from: objective.state,
                to: ObjectiveState::Completed,
            }
            .into());
        }
        if evidence.evidence_ref.trim().is_empty() {
            return Err(ObjectiveError::InvalidEvidence.into());
        }
        if !objective
            .criteria
            .iter()
            .any(|criterion| criterion.id == evidence.criterion_id)
        {
            return Err(ObjectiveError::UnknownCriterion {
                objective_id: objective_id.clone(),
                criterion_id: evidence.criterion_id,
            }
            .into());
        }
        self.record_domain_event(DomainEvent::ObjectiveEvidenceRecorded {
            objective_id: objective_id.clone(),
            evidence,
        })?;
        Ok(())
    }

    pub fn complete_objective(
        &mut self,
        objective_id: &ObjectiveId,
        cause: ObjectiveTransitionCause,
    ) -> Result<ObjectiveRecord, ConductorError> {
        let projection = self.objective_projection()?;
        let objective = projection
            .objectives
            .get(objective_id)
            .ok_or_else(|| ObjectiveError::UnknownObjective(objective_id.clone()))?;
        let missing = objective
            .required_criteria()
            .filter(|criterion| {
                !projection
                    .evidence
                    .contains_key(&(objective_id.clone(), criterion.id.clone()))
            })
            .map(|criterion| criterion.id.clone())
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(ObjectiveError::MissingRequiredEvidence {
                objective_id: objective_id.clone(),
                criteria: missing,
            }
            .into());
        }
        self.transition_objective(objective_id, ObjectiveState::Completed, cause)
    }

    pub fn transition_objective(
        &mut self,
        objective_id: &ObjectiveId,
        to: ObjectiveState,
        cause: ObjectiveTransitionCause,
    ) -> Result<ObjectiveRecord, ConductorError> {
        let objective = self.objective(objective_id)?;
        self.validate_transition_cause(&cause)?;
        if !allowed_transition(&objective.state, &to) {
            return Err(ObjectiveError::InvalidTransition {
                objective_id: objective_id.clone(),
                from: objective.state,
                to,
            }
            .into());
        }
        if to == ObjectiveState::Completed {
            let projection = self.objective_projection()?;
            let missing = objective
                .required_criteria()
                .filter(|criterion| {
                    !projection
                        .evidence
                        .contains_key(&(objective_id.clone(), criterion.id.clone()))
                })
                .map(|criterion| criterion.id.clone())
                .collect::<Vec<_>>();
            if !missing.is_empty() {
                return Err(ObjectiveError::MissingRequiredEvidence {
                    objective_id: objective_id.clone(),
                    criteria: missing,
                }
                .into());
            }
        }
        let transition = ObjectiveTransition {
            objective_id: objective_id.clone(),
            from: objective.state,
            to: to.clone(),
            cause,
        };
        self.record_domain_event(DomainEvent::ObjectiveStateChanged { transition })?;
        self.objective(objective_id)
    }

    fn validate_transition_cause(
        &self,
        cause: &ObjectiveTransitionCause,
    ) -> Result<(), ConductorError> {
        match cause {
            ObjectiveTransitionCause::UserIntent => Ok(()),
            ObjectiveTransitionCause::AgentAction { execution_id }
            | ObjectiveTransitionCause::ExecutionOutcome { execution_id } => {
                if self.executions.contains_key(execution_id) {
                    Ok(())
                } else {
                    Err(ObjectiveError::UnknownExecution(execution_id.clone()).into())
                }
            }
            ObjectiveTransitionCause::EvidenceAssessment { evidence_ref } => {
                if evidence_ref.trim().is_empty() {
                    Err(ObjectiveError::InvalidEvidence.into())
                } else {
                    Ok(())
                }
            }
            ObjectiveTransitionCause::Policy { description } => {
                if description.trim().is_empty() {
                    Err(ObjectiveError::InvalidEvidence.into())
                } else {
                    Ok(())
                }
            }
        }
    }

    pub fn assign_execution_objectives(
        &mut self,
        execution_id: &ExecutionId,
        primary: ObjectiveId,
        mut supporting: BTreeSet<ObjectiveId>,
    ) -> Result<ExecutionObjectiveAssignment, ConductorError> {
        if !self.executions.contains_key(execution_id) {
            return Err(ObjectiveError::UnknownExecution(execution_id.clone()).into());
        }
        supporting.remove(&primary);
        let assignment = ExecutionObjectiveAssignment {
            execution_id: execution_id.clone(),
            primary,
            supporting,
        };
        let projection = self.objective_projection()?;
        if projection.assignments.contains_key(execution_id) {
            return Err(ObjectiveError::InvalidTransition {
                objective_id: assignment.primary.clone(),
                from: ObjectiveState::Active,
                to: ObjectiveState::Active,
            }
            .into());
        }
        let primary = projection
            .objectives
            .get(&assignment.primary)
            .ok_or_else(|| ObjectiveError::UnknownObjective(assignment.primary.clone()))?;
        if primary.state != ObjectiveState::Active {
            return Err(ObjectiveError::InvalidParent(primary.id.clone()).into());
        }
        for supporting in &assignment.supporting {
            let supporting = projection
                .objectives
                .get(supporting)
                .ok_or_else(|| ObjectiveError::UnknownObjective(supporting.clone()))?;
            if supporting.state != ObjectiveState::Active {
                return Err(ObjectiveError::InvalidParent(supporting.id.clone()).into());
            }
        }
        self.record_domain_event(DomainEvent::ExecutionObjectivesAssigned {
            assignment: assignment.clone(),
        })?;
        Ok(assignment)
    }
}

fn next_objective_id(projection: &ObjectiveProjection) -> ObjectiveId {
    ObjectiveId::parse(format!("objective-{}", projection.objectives.len() + 1))
        .expect("generated objective id")
}

fn validate_input(statement: &str, criteria: &[ObjectiveCriterion]) -> Result<(), ObjectiveError> {
    if statement.trim().is_empty() {
        return Err(ObjectiveError::InvalidStatement);
    }
    let mut ids = BTreeSet::new();
    for criterion in criteria {
        if !ids.insert(criterion.id.clone()) {
            return Err(ObjectiveError::DuplicateCriterion(criterion.id.clone()));
        }
    }
    Ok(())
}

fn validate_record(objective: &ObjectiveRecord) -> Result<(), JournalError> {
    validate_input(&objective.statement, &objective.criteria)
        .map_err(|error| invalid_event(error.to_string()))
}

fn validate_replayed_transition_cause(
    cause: &ObjectiveTransitionCause,
    known_executions: &BTreeSet<ExecutionId>,
) -> Result<(), JournalError> {
    match cause {
        ObjectiveTransitionCause::UserIntent => Ok(()),
        ObjectiveTransitionCause::AgentAction { execution_id }
        | ObjectiveTransitionCause::ExecutionOutcome { execution_id } => {
            if known_executions.contains(execution_id) {
                Ok(())
            } else {
                Err(invalid_event(format!(
                    "objective transition references unknown execution {execution_id}"
                )))
            }
        }
        ObjectiveTransitionCause::EvidenceAssessment { evidence_ref } => {
            if evidence_ref.trim().is_empty() {
                Err(invalid_event(
                    "objective transition evidence reference is empty",
                ))
            } else {
                Ok(())
            }
        }
        ObjectiveTransitionCause::Policy { description } => {
            if description.trim().is_empty() {
                Err(invalid_event(
                    "objective transition policy description is empty",
                ))
            } else {
                Ok(())
            }
        }
    }
}

fn allowed_transition(from: &ObjectiveState, to: &ObjectiveState) -> bool {
    matches!(
        (from, to),
        (ObjectiveState::Draft, ObjectiveState::Active)
            | (ObjectiveState::Draft, ObjectiveState::Abandoned)
            | (ObjectiveState::Draft, ObjectiveState::Superseded)
            | (ObjectiveState::Active, ObjectiveState::Completed)
            | (ObjectiveState::Active, ObjectiveState::Failed)
            | (ObjectiveState::Active, ObjectiveState::Invalidated)
            | (ObjectiveState::Active, ObjectiveState::Abandoned)
            | (ObjectiveState::Active, ObjectiveState::Superseded)
    )
}

fn invalid_event(message: impl Into<String>) -> JournalError {
    JournalError::InvalidEvent(message.into())
}
