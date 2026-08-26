use crate::{ConductorError, ConductorRuntime, DomainEvent, JournalError, RuntimeJournal};
use phenix_core::{
    DecisionApplicability, DecisionCreator, DecisionDraftInput, DecisionHistoryQuery,
    DecisionHistoryScope, DecisionId, DecisionRecord, DecisionRelation, DecisionState,
    ExactReference, ExecutionId, ObjectiveId, ObjectiveOrigin,
};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecisionError {
    UnknownDecision(DecisionId),
    InvalidText(&'static str),
    UnknownDependency(DecisionId),
    DependencyCycle(DecisionId),
    UnknownObjective(ObjectiveId),
    UnknownCreatorExecution(phenix_core::ExecutionId),
    InvalidRelation(DecisionId),
    DecisionReferenceNotRecorded(DecisionId),
    RecordedDecisionIsImmutable(DecisionId),
    RevisionConflict {
        decision_id: DecisionId,
        expected: u64,
        actual: u64,
    },
    DecisionAlreadyRecorded(DecisionId),
    DecisionNotRecorded(DecisionId),
}

impl Display for DecisionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownDecision(id) => write!(f, "unknown decision: {id}"),
            Self::InvalidText(field) => write!(f, "decision {field} must not be empty"),
            Self::UnknownDependency(id) => write!(f, "unknown decision dependency: {id}"),
            Self::DependencyCycle(id) => write!(f, "decision dependency cycle reaches {id}"),
            Self::UnknownObjective(id) => write!(f, "unknown decision objective: {id}"),
            Self::UnknownCreatorExecution(id) => {
                write!(f, "unknown decision creator execution: {id}")
            }
            Self::InvalidRelation(id) => write!(f, "invalid decision relation target: {id}"),
            Self::DecisionReferenceNotRecorded(id) => {
                write!(f, "decision reference is not recorded: {id}")
            }
            Self::RecordedDecisionIsImmutable(id) => {
                write!(f, "recorded decision is immutable: {id}")
            }
            Self::RevisionConflict {
                decision_id,
                expected,
                actual,
            } => write!(
                f,
                "decision {decision_id} revision conflict: expected {expected}, found {actual}"
            ),
            Self::DecisionAlreadyRecorded(id) => write!(f, "decision is already recorded: {id}"),
            Self::DecisionNotRecorded(id) => write!(f, "decision is not recorded: {id}"),
        }
    }
}
impl Error for DecisionError {}

#[derive(Default)]
struct DecisionProjection {
    decisions: BTreeMap<DecisionId, DecisionRecord>,
}

impl DecisionProjection {
    fn apply(&mut self, event: &DomainEvent) -> Result<(), JournalError> {
        match event {
            DomainEvent::DecisionDraftCreated { decision } => {
                validate_record_shape(decision)?;
                let expected = DecisionId::parse(format!("decision-{}", self.decisions.len() + 1))
                    .expect("generated decision id");
                if decision.id != expected
                    || decision.revision != 1
                    || decision.state != DecisionState::Draft
                {
                    return Err(invalid_event(format!(
                        "decision creation identity mismatch: expected {expected} revision 1 draft"
                    )));
                }
                validate_graph_references(decision, &self.decisions)?;
                self.decisions.insert(decision.id.clone(), decision.clone());
                validate_acyclic(&self.decisions)?;
            }
            DomainEvent::DecisionDraftRevised {
                decision,
                expected_revision,
            } => {
                validate_record_shape(decision)?;
                let previous = self.decisions.get(&decision.id).ok_or_else(|| {
                    invalid_event(format!(
                        "decision draft revision references unknown decision {}",
                        decision.id
                    ))
                })?;
                if previous.state != DecisionState::Draft || decision.state != DecisionState::Draft
                {
                    return Err(invalid_event(format!(
                        "recorded decision {} was revised",
                        decision.id
                    )));
                }
                if previous.revision != *expected_revision
                    || decision.revision != expected_revision + 1
                {
                    return Err(invalid_event(format!(
                        "decision {} revision sequence is invalid",
                        decision.id
                    )));
                }
                if previous.workspace != decision.workspace
                    || previous.applicability != decision.applicability
                {
                    return Err(invalid_event(format!("decision {} changed immutable ownership or applicability in a draft revision", decision.id)));
                }
                validate_graph_references(decision, &self.decisions)?;
                self.decisions.insert(decision.id.clone(), decision.clone());
                validate_acyclic(&self.decisions)?;
            }
            DomainEvent::DecisionRecorded { decision_id } => {
                let decision = self.decisions.get(decision_id).ok_or_else(|| {
                    invalid_event(format!(
                        "recording references unknown decision {decision_id}"
                    ))
                })?;
                if decision.state != DecisionState::Draft {
                    return Err(invalid_event(format!(
                        "decision {decision_id} was recorded more than once"
                    )));
                }
                if let Some(reference) =
                    first_unrecorded_decision_reference(decision, &self.decisions)
                {
                    return Err(invalid_event(format!(
                        "decision {decision_id} records unstable decision reference {reference}"
                    )));
                }
                self.decisions
                    .get_mut(decision_id)
                    .expect("validated decision exists")
                    .state = DecisionState::Recorded;
            }
            DomainEvent::DecisionApplicabilityAssessed {
                decision_id,
                applicability,
            } => {
                let decision = self.decisions.get_mut(decision_id).ok_or_else(|| {
                    invalid_event(format!(
                        "applicability references unknown decision {decision_id}"
                    ))
                })?;
                if decision.state != DecisionState::Recorded {
                    return Err(invalid_event(format!(
                        "draft decision {decision_id} received applicability assessment"
                    )));
                }
                decision.applicability = applicability.clone();
            }
            _ => {}
        }
        Ok(())
    }
}

pub(crate) fn validate_journal_decisions(journal: &RuntimeJournal) -> Result<(), JournalError> {
    let mut projection = DecisionProjection::default();
    for entry in &journal.entries {
        projection.apply(&entry.event)?;
    }
    Ok(())
}

impl ConductorRuntime {
    fn decision_projection(&self) -> Result<DecisionProjection, ConductorError> {
        let mut projection = DecisionProjection::default();
        for entry in &self.journal.entries {
            projection.apply(&entry.event)?;
        }
        Ok(projection)
    }

    pub fn decisions(&self) -> Result<Vec<DecisionRecord>, ConductorError> {
        Ok(self
            .decision_projection()?
            .decisions
            .into_values()
            .collect())
    }

    pub fn decision(&self, decision_id: &DecisionId) -> Result<DecisionRecord, ConductorError> {
        self.decision_projection()?
            .decisions
            .get(decision_id)
            .cloned()
            .ok_or_else(|| DecisionError::UnknownDecision(decision_id.clone()).into())
    }

    pub fn create_decision_draft(
        &mut self,
        input: DecisionDraftInput,
    ) -> Result<DecisionRecord, ConductorError> {
        self.validate_decision_input(&input, None)?;
        let projection = self.decision_projection()?;
        let decision = DecisionRecord {
            id: DecisionId::parse(format!("decision-{}", projection.decisions.len() + 1))
                .expect("generated decision id"),
            workspace: self.workspace_id.clone(),
            revision: 1,
            state: DecisionState::Draft,
            question: input.question,
            chosen_option: input.chosen_option,
            rationale: input.rationale,
            alternatives: input.alternatives,
            alternatives_not_considered_reason: input.alternatives_not_considered_reason,
            evidence: input.evidence,
            creator: input.creator,
            objectives: input.objectives,
            dependencies: input.dependencies,
            relation: input.relation,
            applicability: DecisionApplicability::Applicable,
        };
        let mut candidate = projection.decisions;
        candidate.insert(decision.id.clone(), decision.clone());
        validate_acyclic(&candidate).map_err(ConductorError::from)?;
        self.record_domain_event(DomainEvent::DecisionDraftCreated {
            decision: decision.clone(),
        })?;
        Ok(decision)
    }

    pub fn revise_decision_draft(
        &mut self,
        decision_id: &DecisionId,
        expected_revision: u64,
        input: DecisionDraftInput,
    ) -> Result<DecisionRecord, ConductorError> {
        let previous = self.decision(decision_id)?;
        if previous.state != DecisionState::Draft {
            return Err(DecisionError::RecordedDecisionIsImmutable(decision_id.clone()).into());
        }
        if previous.revision != expected_revision {
            return Err(DecisionError::RevisionConflict {
                decision_id: decision_id.clone(),
                expected: expected_revision,
                actual: previous.revision,
            }
            .into());
        }
        self.validate_decision_input(&input, Some(decision_id))?;
        let decision = DecisionRecord {
            id: decision_id.clone(),
            workspace: previous.workspace,
            revision: expected_revision + 1,
            state: DecisionState::Draft,
            question: input.question,
            chosen_option: input.chosen_option,
            rationale: input.rationale,
            alternatives: input.alternatives,
            alternatives_not_considered_reason: input.alternatives_not_considered_reason,
            evidence: input.evidence,
            creator: input.creator,
            objectives: input.objectives,
            dependencies: input.dependencies,
            relation: input.relation,
            applicability: previous.applicability,
        };
        let mut candidate = self.decision_projection()?.decisions;
        candidate.insert(decision.id.clone(), decision.clone());
        validate_acyclic(&candidate).map_err(ConductorError::from)?;
        self.record_domain_event(DomainEvent::DecisionDraftRevised {
            decision: decision.clone(),
            expected_revision,
        })?;
        Ok(decision)
    }

    pub fn record_decision(
        &mut self,
        decision_id: &DecisionId,
    ) -> Result<DecisionRecord, ConductorError> {
        let decision = self.decision(decision_id)?;
        if decision.state == DecisionState::Recorded {
            return Err(DecisionError::DecisionAlreadyRecorded(decision_id.clone()).into());
        }
        let projection = self.decision_projection()?;
        if let Some(reference) =
            first_unrecorded_decision_reference(&decision, &projection.decisions)
        {
            return Err(DecisionError::DecisionReferenceNotRecorded(reference).into());
        }
        self.record_domain_event(DomainEvent::DecisionRecorded {
            decision_id: decision_id.clone(),
        })?;
        self.decision(decision_id)
    }

    pub fn assess_decision_applicability(
        &mut self,
        decision_id: &DecisionId,
        applicability: DecisionApplicability,
    ) -> Result<DecisionRecord, ConductorError> {
        let decision = self.decision(decision_id)?;
        if decision.state != DecisionState::Recorded {
            return Err(DecisionError::DecisionNotRecorded(decision_id.clone()).into());
        }
        self.record_domain_event(DomainEvent::DecisionApplicabilityAssessed {
            decision_id: decision_id.clone(),
            applicability,
        })?;
        self.decision(decision_id)
    }

    pub fn decision_history_query_for_execution(
        &self,
        execution_id: &ExecutionId,
        text: impl Into<String>,
        limit: usize,
    ) -> Result<DecisionHistoryQuery, ConductorError> {
        let assignment = self.execution_objectives(execution_id)?.ok_or_else(|| {
            crate::ObjectiveError::MissingExecutionObjective(execution_id.clone())
        })?;
        Ok(DecisionHistoryQuery {
            text: text.into(),
            scope: DecisionHistoryScope::ObjectiveLineage(assignment.primary),
            limit,
        })
    }

    pub(crate) fn decisions_for_objective_lineage(
        &self,
        objective_id: &ObjectiveId,
    ) -> Result<Vec<DecisionRecord>, ConductorError> {
        let objectives = self.objectives()?;
        let by_id = objectives
            .into_iter()
            .map(|objective| (objective.id.clone(), objective))
            .collect::<BTreeMap<_, _>>();
        let mut lineage = BTreeSet::new();
        let mut current = Some(objective_id.clone());
        while let Some(id) = current {
            if !lineage.insert(id.clone()) {
                break;
            }
            current = by_id
                .get(&id)
                .and_then(|objective| match &objective.origin {
                    ObjectiveOrigin::Root => None,
                    ObjectiveOrigin::Derived { parent } => Some(parent.clone()),
                });
        }
        Ok(self
            .decisions()?
            .into_iter()
            .filter(|decision| {
                decision.state == DecisionState::Recorded
                    && decision
                        .objectives
                        .iter()
                        .any(|objective| lineage.contains(objective))
            })
            .collect())
    }

    fn validate_decision_input(
        &self,
        input: &DecisionDraftInput,
        editing: Option<&DecisionId>,
    ) -> Result<(), ConductorError> {
        for (value, field) in [
            (&input.question, "question"),
            (&input.chosen_option, "chosen option"),
            (&input.rationale, "rationale"),
        ] {
            if value.trim().is_empty() {
                return Err(DecisionError::InvalidText(field).into());
            }
        }
        if input
            .alternatives
            .iter()
            .any(|alternative| alternative.trim().is_empty())
        {
            return Err(DecisionError::InvalidText("alternative").into());
        }
        match (
            input.alternatives.is_empty(),
            input.alternatives_not_considered_reason.as_deref(),
        ) {
            (true, Some(reason)) if !reason.trim().is_empty() => {}
            (true, _) => {
                return Err(
                    DecisionError::InvalidText("why no alternatives were considered").into(),
                );
            }
            (false, Some(_)) => {
                return Err(DecisionError::InvalidText(
                    "alternatives-not-considered reason with alternatives",
                )
                .into());
            }
            (false, None) => {}
        }
        let projection = self.decision_projection()?;
        for dependency in &input.dependencies {
            if Some(dependency) == editing {
                return Err(DecisionError::DependencyCycle(dependency.clone()).into());
            }
            if !projection.decisions.contains_key(dependency) {
                return Err(DecisionError::UnknownDependency(dependency.clone()).into());
            }
        }
        if let Some(relation) = &input.relation {
            let target = relation_target(relation);
            if Some(target) == editing || !projection.decisions.contains_key(target) {
                return Err(DecisionError::InvalidRelation(target.clone()).into());
            }
        }
        for objective in &input.objectives {
            self.objective(objective)
                .map_err(|_| DecisionError::UnknownObjective(objective.clone()))?;
        }
        if let DecisionCreator::Execution { execution_id } = &input.creator {
            if !self.executions.contains_key(execution_id) {
                return Err(DecisionError::UnknownCreatorExecution(execution_id.clone()).into());
            }
        }
        Ok(())
    }
}

fn first_unrecorded_decision_reference(
    decision: &DecisionRecord,
    decisions: &BTreeMap<DecisionId, DecisionRecord>,
) -> Option<DecisionId> {
    let is_unstable = |id: &DecisionId| {
        decisions
            .get(id)
            .is_none_or(|target| target.state != DecisionState::Recorded)
    };
    for dependency in &decision.dependencies {
        if is_unstable(dependency) {
            return Some(dependency.clone());
        }
    }
    if let Some(relation) = &decision.relation {
        let target = relation_target(relation);
        if is_unstable(target) {
            return Some(target.clone());
        }
    }
    for evidence in &decision.evidence {
        if let ExactReference::Decision(target) = evidence {
            if is_unstable(target) {
                return Some(target.clone());
            }
        }
    }
    None
}

fn relation_target(relation: &DecisionRelation) -> &DecisionId {
    match relation {
        DecisionRelation::Supersedes { decision_id }
        | DecisionRelation::Reverts { decision_id } => decision_id,
    }
}

fn validate_record_shape(decision: &DecisionRecord) -> Result<(), JournalError> {
    if decision.question.trim().is_empty()
        || decision.chosen_option.trim().is_empty()
        || decision.rationale.trim().is_empty()
    {
        return Err(invalid_event(format!(
            "decision {} contains empty required text",
            decision.id
        )));
    }
    if decision
        .alternatives
        .iter()
        .any(|alternative| alternative.trim().is_empty())
    {
        return Err(invalid_event(format!(
            "decision {} contains an empty alternative",
            decision.id
        )));
    }
    match (
        decision.alternatives.is_empty(),
        decision.alternatives_not_considered_reason.as_deref(),
    ) {
        (true, Some(reason)) if !reason.trim().is_empty() => {}
        (true, _) => {
            return Err(invalid_event(format!(
                "decision {} omits why no alternatives were considered",
                decision.id
            )));
        }
        (false, Some(_)) => {
            return Err(invalid_event(format!(
                "decision {} records a no-alternatives reason alongside alternatives",
                decision.id
            )));
        }
        (false, None) => {}
    }
    Ok(())
}

fn validate_graph_references(
    decision: &DecisionRecord,
    decisions: &BTreeMap<DecisionId, DecisionRecord>,
) -> Result<(), JournalError> {
    for dependency in &decision.dependencies {
        if dependency == &decision.id || !decisions.contains_key(dependency) {
            return Err(invalid_event(format!(
                "decision {} has invalid dependency {dependency}",
                decision.id
            )));
        }
    }
    if let Some(relation) = &decision.relation {
        let target = relation_target(relation);
        if target == &decision.id || !decisions.contains_key(target) {
            return Err(invalid_event(format!(
                "decision {} has invalid relation target {target}",
                decision.id
            )));
        }
    }
    Ok(())
}

fn validate_acyclic(decisions: &BTreeMap<DecisionId, DecisionRecord>) -> Result<(), JournalError> {
    fn visit(
        id: &DecisionId,
        decisions: &BTreeMap<DecisionId, DecisionRecord>,
        visiting: &mut BTreeSet<DecisionId>,
        visited: &mut BTreeSet<DecisionId>,
    ) -> Result<(), JournalError> {
        if visited.contains(id) {
            return Ok(());
        }
        if !visiting.insert(id.clone()) {
            return Err(invalid_event(format!(
                "decision dependency cycle reaches {id}"
            )));
        }
        if let Some(decision) = decisions.get(id) {
            for dependency in &decision.dependencies {
                visit(dependency, decisions, visiting, visited)?;
            }
        }
        visiting.remove(id);
        visited.insert(id.clone());
        Ok(())
    }
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for id in decisions.keys() {
        visit(id, decisions, &mut visiting, &mut visited)?;
    }
    Ok(())
}

fn invalid_event(message: impl Into<String>) -> JournalError {
    JournalError::InvalidEvent(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_core::{
        BackendId, DecisionDraftInput, DecisionRelation, ExactReference, ExecutionTarget,
        InferenceOptions, ModelId, ModelTarget, ProviderId,
    };

    fn target() -> ExecutionTarget {
        ExecutionTarget::Fixed(ModelTarget {
            backend: BackendId::parse("mock").unwrap(),
            provider: ProviderId::parse("mock").unwrap(),
            model: ModelId::parse("mock").unwrap(),
            inference: InferenceOptions::default(),
        })
    }

    fn input(question: &str, objective: ObjectiveId) -> DecisionDraftInput {
        DecisionDraftInput {
            question: question.to_owned(),
            chosen_option: "choose typed state".to_owned(),
            rationale: "it preserves replay semantics".to_owned(),
            alternatives: vec!["store prose only".to_owned()],
            alternatives_not_considered_reason: None,
            evidence: vec![ExactReference::Objective(objective.clone())],
            creator: DecisionCreator::User,
            objectives: BTreeSet::from([objective]),
            dependencies: BTreeSet::new(),
            relation: None,
        }
    }

    #[test]
    fn recorded_decision_is_immutable_and_successor_preserves_history() {
        let mut runtime = ConductorRuntime::new();
        let session = runtime.create_session(None, None, target()).unwrap();
        let execution = runtime
            .submit(&session.id, "make a durable decision")
            .unwrap();
        let objective = runtime
            .execution_objectives(&execution.id)
            .unwrap()
            .unwrap()
            .primary;
        let first = runtime
            .create_decision_draft(input("Which representation?", objective.clone()))
            .unwrap();
        let first = runtime.record_decision(&first.id).unwrap();
        assert!(matches!(
            runtime.revise_decision_draft(
                &first.id,
                first.revision,
                input("change it", objective.clone())
            ),
            Err(ConductorError::Decision(
                DecisionError::RecordedDecisionIsImmutable(_)
            ))
        ));
        let mut successor_input = input("Should the decision change?", objective);
        successor_input.relation = Some(DecisionRelation::Supersedes {
            decision_id: first.id.clone(),
        });
        let successor = runtime.create_decision_draft(successor_input).unwrap();
        runtime.record_decision(&successor.id).unwrap();
        assert_eq!(runtime.decision(&first.id).unwrap(), first);
    }

    #[test]
    fn dependency_cycle_is_rejected() {
        let mut runtime = ConductorRuntime::new();
        let session = runtime.create_session(None, None, target()).unwrap();
        let execution = runtime.submit(&session.id, "make decisions").unwrap();
        let objective = runtime
            .execution_objectives(&execution.id)
            .unwrap()
            .unwrap()
            .primary;
        let first = runtime
            .create_decision_draft(input("first", objective.clone()))
            .unwrap();
        let mut second_input = input("second", objective.clone());
        second_input.dependencies.insert(first.id.clone());
        let second = runtime.create_decision_draft(second_input).unwrap();
        let mut revised = input("first revised", objective);
        revised.dependencies.insert(second.id.clone());
        assert!(matches!(
            runtime.revise_decision_draft(&first.id, 1, revised),
            Err(ConductorError::Journal(JournalError::InvalidEvent(_)))
        ));
    }
}
