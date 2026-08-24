use crate::{ConductorError, ConductorRuntime, DomainEvent, JournalError, RuntimeJournal};
use phenix_core::{
    ExecutionId, ExecutionPlanAssignment, ObjectiveId, PlanId, PlanRecord, PlanState, PlanStep,
    PlanStepId, PlanStepState, PlanStepTransition, PlanTransition, PlanTransitionCause,
    WorkspaceId,
};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlanError {
    UnknownPlan(PlanId),
    UnknownStep {
        plan_id: PlanId,
        step_id: PlanStepId,
    },
    UnknownExecution(ExecutionId),
    WrongWorkspace(PlanId),
    EmptyPlan,
    InvalidStep(PlanStepId),
    DuplicateStep(PlanStepId),
    InvalidDependency {
        step_id: PlanStepId,
        dependency: PlanStepId,
    },
    DependencyCycle,
    UnknownObjective(ObjectiveId),
    WrongObjectiveWorkspace(ObjectiveId),
    EnactedPlanIsImmutable(PlanId),
    DraftRevisionConflict {
        plan_id: PlanId,
        expected: u64,
        actual: u64,
    },
    InvalidTransition {
        plan_id: PlanId,
        from: PlanState,
        to: PlanState,
    },
    InvalidStepTransition {
        plan_id: PlanId,
        step_id: PlanStepId,
        from: PlanStepState,
        to: PlanStepState,
    },
    IncompleteDependencies {
        plan_id: PlanId,
        step_id: PlanStepId,
    },
    ExecutionAlreadyAssigned(ExecutionId),
    InvalidCause,
    InvalidSuccessor(PlanId),
}

impl Display for PlanError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownPlan(id) => write!(f, "unknown plan: {id}"),
            Self::UnknownStep { plan_id, step_id } => {
                write!(f, "plan {plan_id} has no step {step_id}")
            }
            Self::UnknownExecution(id) => write!(f, "unknown execution: {id}"),
            Self::WrongWorkspace(id) => write!(f, "plan belongs to another workspace: {id}"),
            Self::EmptyPlan => f.write_str("plan must contain at least one step"),
            Self::InvalidStep(id) => write!(f, "invalid plan step: {id}"),
            Self::DuplicateStep(id) => write!(f, "duplicate plan step: {id}"),
            Self::InvalidDependency {
                step_id,
                dependency,
            } => {
                write!(f, "plan step {step_id} has invalid dependency {dependency}")
            }
            Self::DependencyCycle => f.write_str("plan step dependencies contain a cycle"),
            Self::UnknownObjective(id) => write!(f, "unknown objective: {id}"),
            Self::WrongObjectiveWorkspace(id) => {
                write!(f, "objective belongs to another workspace: {id}")
            }
            Self::EnactedPlanIsImmutable(id) => write!(f, "enacted plan is immutable: {id}"),
            Self::DraftRevisionConflict {
                plan_id,
                expected,
                actual,
            } => write!(
                f,
                "plan {plan_id} draft revision conflict: expected {expected}, current {actual}"
            ),
            Self::InvalidTransition { plan_id, from, to } => {
                write!(
                    f,
                    "plan {plan_id} cannot transition from {from:?} to {to:?}"
                )
            }
            Self::InvalidStepTransition {
                plan_id,
                step_id,
                from,
                to,
            } => write!(
                f,
                "plan {plan_id} step {step_id} cannot transition from {from:?} to {to:?}"
            ),
            Self::IncompleteDependencies { plan_id, step_id } => {
                write!(
                    f,
                    "plan {plan_id} step {step_id} has incomplete dependencies"
                )
            }
            Self::ExecutionAlreadyAssigned(id) => {
                write!(f, "execution already has a plan-step assignment: {id}")
            }
            Self::InvalidCause => f.write_str("plan transition cause is invalid"),
            Self::InvalidSuccessor(id) => {
                write!(f, "plan cannot be superseded in its current state: {id}")
            }
        }
    }
}

impl Error for PlanError {}

#[derive(Default)]
struct PlanProjection {
    plans: BTreeMap<PlanId, PlanRecord>,
    assignments: BTreeMap<ExecutionId, ExecutionPlanAssignment>,
    known_executions: BTreeSet<ExecutionId>,
    objectives: BTreeMap<ObjectiveId, WorkspaceId>,
}

impl PlanProjection {
    fn apply(&mut self, event: &DomainEvent) -> Result<(), JournalError> {
        match event {
            DomainEvent::ExecutionCreated { execution, .. } => {
                self.known_executions.insert(execution.id.clone());
            }
            DomainEvent::ObjectiveCreated { objective } => {
                self.objectives
                    .insert(objective.id.clone(), objective.workspace.clone());
            }
            DomainEvent::PlanCreated { plan } => {
                validate_record(plan, &self.objectives)?;
                let expected = PlanId::parse(format!("plan-{}", self.plans.len() + 1))
                    .expect("generated plan id");
                if plan.id != expected || self.plans.contains_key(&plan.id) {
                    return Err(invalid_event(format!(
                        "plan identity cursor mismatch: expected {expected}, found {}",
                        plan.id
                    )));
                }
                if plan.state != PlanState::Draft || plan.revision != 1 {
                    return Err(invalid_event(format!(
                        "plan {} must be created as draft revision 1",
                        plan.id
                    )));
                }
                if let Some(previous) = plan.supersedes.as_ref() {
                    let previous = self.plans.get(previous).ok_or_else(|| {
                        invalid_event(format!(
                            "plan {} supersedes unknown plan {previous}",
                            plan.id
                        ))
                    })?;
                    if previous.workspace != plan.workspace
                        || !valid_successor_predecessor_state(&previous.state)
                    {
                        return Err(invalid_event(format!(
                            "plan {} has invalid predecessor {}",
                            plan.id, previous.id
                        )));
                    }
                }
                self.plans.insert(plan.id.clone(), plan.clone());
            }
            DomainEvent::PlanDraftRevised {
                plan,
                expected_revision,
            } => {
                validate_record(plan, &self.objectives)?;
                let previous = self.plans.get(&plan.id).ok_or_else(|| {
                    invalid_event(format!(
                        "draft revision references unknown plan {}",
                        plan.id
                    ))
                })?;
                if previous.state != PlanState::Draft || plan.state != PlanState::Draft {
                    return Err(invalid_event(format!(
                        "only a prospective plan draft may be revised: {}",
                        plan.id
                    )));
                }
                if previous.revision != *expected_revision
                    || plan.revision != expected_revision.saturating_add(1)
                {
                    return Err(invalid_event(format!(
                        "stale plan draft revision for {}: expected {}, current {}",
                        plan.id, expected_revision, previous.revision
                    )));
                }
                if previous.workspace != plan.workspace || previous.supersedes != plan.supersedes {
                    return Err(invalid_event(format!(
                        "plan {} draft revision changed immutable ownership",
                        plan.id
                    )));
                }
                self.plans.insert(plan.id.clone(), plan.clone());
            }
            DomainEvent::ExecutionPlanAssigned { assignment } => {
                if !self.known_executions.contains(&assignment.execution_id) {
                    return Err(invalid_event(format!(
                        "plan assignment references unknown execution {}",
                        assignment.execution_id
                    )));
                }
                if self.assignments.contains_key(&assignment.execution_id) {
                    return Err(invalid_event(format!(
                        "execution {} received more than one plan assignment",
                        assignment.execution_id
                    )));
                }
                let plan = self.plans.get_mut(&assignment.plan_id).ok_or_else(|| {
                    invalid_event(format!(
                        "plan assignment references unknown plan {}",
                        assignment.plan_id
                    ))
                })?;
                if !matches!(plan.state, PlanState::Draft | PlanState::Active) {
                    return Err(invalid_event(format!(
                        "execution cannot enact terminal plan {}",
                        plan.id
                    )));
                }
                if plan.state == PlanState::Draft {
                    plan.state = PlanState::Active;
                    for step in &mut plan.steps {
                        if step.state != PlanStepState::Proposed {
                            return Err(invalid_event(format!(
                                "draft plan {} contains non-proposed step {}",
                                plan.id, step.id
                            )));
                        }
                        step.state = PlanStepState::Committed;
                    }
                }
                let position = plan
                    .steps
                    .iter()
                    .position(|step| step.id == assignment.step_id)
                    .ok_or_else(|| {
                        invalid_event(format!(
                            "plan {} has no assigned step {}",
                            plan.id, assignment.step_id
                        ))
                    })?;
                if !matches!(
                    plan.steps[position].state,
                    PlanStepState::Committed | PlanStepState::Active
                ) {
                    return Err(invalid_event(format!(
                        "plan {} step {} is not enactable",
                        plan.id, assignment.step_id
                    )));
                }
                let dependencies = plan.steps[position].depends_on.clone();
                if dependencies.iter().any(|dependency| {
                    plan.steps
                        .iter()
                        .find(|step| step.id == *dependency)
                        .is_none_or(|step| step.state != PlanStepState::Completed)
                }) {
                    return Err(invalid_event(format!(
                        "plan {} step {} has incomplete dependencies",
                        plan.id, assignment.step_id
                    )));
                }
                plan.steps[position].state = PlanStepState::Active;
                self.assignments
                    .insert(assignment.execution_id.clone(), assignment.clone());
            }
            DomainEvent::PlanStepStateChanged { transition } => {
                validate_cause(&transition.cause, &self.known_executions)?;
                let plan = self.plans.get_mut(&transition.plan_id).ok_or_else(|| {
                    invalid_event(format!(
                        "step transition references unknown plan {}",
                        transition.plan_id
                    ))
                })?;
                if plan.state != PlanState::Active {
                    return Err(invalid_event(format!(
                        "terminal plan {} cannot change step state",
                        transition.plan_id
                    )));
                }
                let step = plan
                    .steps
                    .iter_mut()
                    .find(|step| step.id == transition.step_id)
                    .ok_or_else(|| {
                        invalid_event(format!(
                            "plan {} has no step {}",
                            transition.plan_id, transition.step_id
                        ))
                    })?;
                if step.state != transition.from
                    || !allowed_step_transition(&transition.from, &transition.to)
                {
                    return Err(invalid_event(format!(
                        "invalid step transition for {}/{}: {:?} -> {:?}",
                        transition.plan_id, transition.step_id, transition.from, transition.to
                    )));
                }
                step.state = transition.to.clone();
            }
            DomainEvent::PlanStateChanged { transition } => {
                validate_cause(&transition.cause, &self.known_executions)?;
                let has_successor = transition.to != PlanState::Superseded
                    || self.plans.values().any(|candidate| {
                        candidate.supersedes.as_ref() == Some(&transition.plan_id)
                    });
                let plan = self.plans.get_mut(&transition.plan_id).ok_or_else(|| {
                    invalid_event(format!(
                        "plan transition references unknown plan {}",
                        transition.plan_id
                    ))
                })?;
                if plan.state != transition.from
                    || !allowed_plan_transition(&transition.from, &transition.to)
                {
                    return Err(invalid_event(format!(
                        "invalid plan transition for {}: {:?} -> {:?}",
                        transition.plan_id, transition.from, transition.to
                    )));
                }
                if transition.to == PlanState::Completed
                    && plan
                        .steps
                        .iter()
                        .any(|step| step.state != PlanStepState::Completed)
                {
                    return Err(invalid_event(format!(
                        "plan {} completed with incomplete steps",
                        transition.plan_id
                    )));
                }
                if transition.to == PlanState::Superseded && !has_successor {
                    return Err(invalid_event(format!(
                        "plan {} was superseded without a successor",
                        transition.plan_id
                    )));
                }
                plan.state = transition.to.clone();
            }
            _ => {}
        }
        Ok(())
    }
}

pub(crate) fn validate_journal_plans(journal: &RuntimeJournal) -> Result<(), JournalError> {
    let mut projection = PlanProjection::default();
    for entry in &journal.entries {
        projection.apply(&entry.event)?;
    }
    Ok(())
}

impl ConductorRuntime {
    fn plan_projection(&self) -> Result<PlanProjection, ConductorError> {
        let mut projection = PlanProjection::default();
        for entry in &self.journal.entries {
            projection.apply(&entry.event)?;
        }
        Ok(projection)
    }

    pub fn plans(&self) -> Result<Vec<PlanRecord>, ConductorError> {
        Ok(self.plan_projection()?.plans.into_values().collect())
    }

    pub fn plan(&self, plan_id: &PlanId) -> Result<PlanRecord, ConductorError> {
        self.plan_projection()?
            .plans
            .get(plan_id)
            .cloned()
            .ok_or_else(|| PlanError::UnknownPlan(plan_id.clone()).into())
    }

    pub fn execution_plan(
        &self,
        execution_id: &ExecutionId,
    ) -> Result<Option<ExecutionPlanAssignment>, ConductorError> {
        Ok(self
            .plan_projection()?
            .assignments
            .get(execution_id)
            .cloned())
    }

    pub fn create_plan(
        &mut self,
        objective_refs: BTreeSet<ObjectiveId>,
        steps: Vec<PlanStep>,
    ) -> Result<PlanRecord, ConductorError> {
        self.create_plan_record(objective_refs, steps, None)
    }

    fn create_plan_record(
        &mut self,
        objective_refs: BTreeSet<ObjectiveId>,
        steps: Vec<PlanStep>,
        supersedes: Option<PlanId>,
    ) -> Result<PlanRecord, ConductorError> {
        self.ensure_objective_semantics_active()?;
        self.validate_plan_objectives(&objective_refs, &steps)?;
        let projection = self.plan_projection()?;
        if let Some(previous) = supersedes.as_ref() {
            let previous = projection
                .plans
                .get(previous)
                .ok_or_else(|| PlanError::UnknownPlan(previous.clone()))?;
            if previous.workspace != self.workspace_id {
                return Err(PlanError::WrongWorkspace(previous.id.clone()).into());
            }
            if !valid_successor_predecessor_state(&previous.state) {
                return Err(PlanError::InvalidSuccessor(previous.id.clone()).into());
            }
        }
        let plan = PlanRecord {
            id: next_plan_id(&projection),
            workspace: self.workspace_id.clone(),
            state: PlanState::Draft,
            revision: 1,
            objective_refs,
            supersedes,
            steps,
        };
        validate_record_runtime(&plan)?;
        self.record_domain_event(DomainEvent::PlanCreated { plan: plan.clone() })?;
        Ok(plan)
    }

    pub fn revise_plan_draft(
        &mut self,
        plan_id: &PlanId,
        expected_revision: u64,
        objective_refs: BTreeSet<ObjectiveId>,
        steps: Vec<PlanStep>,
    ) -> Result<PlanRecord, ConductorError> {
        let current = self.plan(plan_id)?;
        if current.state != PlanState::Draft {
            return Err(PlanError::EnactedPlanIsImmutable(plan_id.clone()).into());
        }
        if current.revision != expected_revision {
            return Err(PlanError::DraftRevisionConflict {
                plan_id: plan_id.clone(),
                expected: expected_revision,
                actual: current.revision,
            }
            .into());
        }
        self.validate_plan_objectives(&objective_refs, &steps)?;
        let plan = PlanRecord {
            id: current.id,
            workspace: current.workspace,
            state: PlanState::Draft,
            revision: expected_revision.checked_add(1).ok_or_else(|| {
                PlanError::DraftRevisionConflict {
                    plan_id: plan_id.clone(),
                    expected: expected_revision,
                    actual: expected_revision,
                }
            })?,
            objective_refs,
            supersedes: current.supersedes,
            steps,
        };
        validate_record_runtime(&plan)?;
        self.record_domain_event(DomainEvent::PlanDraftRevised {
            plan: plan.clone(),
            expected_revision,
        })?;
        Ok(plan)
    }

    pub fn assign_execution_to_plan_step(
        &mut self,
        execution_id: &ExecutionId,
        plan_id: &PlanId,
        step_id: &PlanStepId,
    ) -> Result<ExecutionPlanAssignment, ConductorError> {
        let projection = self.plan_projection()?;
        if !projection.known_executions.contains(execution_id) {
            return Err(PlanError::UnknownExecution(execution_id.clone()).into());
        }
        if projection.assignments.contains_key(execution_id) {
            return Err(PlanError::ExecutionAlreadyAssigned(execution_id.clone()).into());
        }
        let plan = projection
            .plans
            .get(plan_id)
            .ok_or_else(|| PlanError::UnknownPlan(plan_id.clone()))?;
        if plan.workspace != self.workspace_id {
            return Err(PlanError::WrongWorkspace(plan_id.clone()).into());
        }
        let step = plan
            .steps
            .iter()
            .find(|step| &step.id == step_id)
            .ok_or_else(|| PlanError::UnknownStep {
                plan_id: plan_id.clone(),
                step_id: step_id.clone(),
            })?;
        if step.depends_on.iter().any(|dependency| {
            plan.steps
                .iter()
                .find(|candidate| candidate.id == *dependency)
                .is_none_or(|candidate| candidate.state != PlanStepState::Completed)
        }) {
            return Err(PlanError::IncompleteDependencies {
                plan_id: plan_id.clone(),
                step_id: step_id.clone(),
            }
            .into());
        }
        let assignment = ExecutionPlanAssignment {
            execution_id: execution_id.clone(),
            plan_id: plan_id.clone(),
            step_id: step_id.clone(),
        };
        self.record_domain_event(DomainEvent::ExecutionPlanAssigned {
            assignment: assignment.clone(),
        })?;
        Ok(assignment)
    }

    pub fn transition_plan_step(
        &mut self,
        plan_id: &PlanId,
        step_id: &PlanStepId,
        to: PlanStepState,
        cause: PlanTransitionCause,
    ) -> Result<PlanRecord, ConductorError> {
        let projection = self.plan_projection()?;
        let plan = projection
            .plans
            .get(plan_id)
            .ok_or_else(|| PlanError::UnknownPlan(plan_id.clone()))?;
        if plan.state != PlanState::Active {
            let step = plan
                .steps
                .iter()
                .find(|step| &step.id == step_id)
                .ok_or_else(|| PlanError::UnknownStep {
                    plan_id: plan_id.clone(),
                    step_id: step_id.clone(),
                })?;
            return Err(PlanError::InvalidStepTransition {
                plan_id: plan_id.clone(),
                step_id: step_id.clone(),
                from: step.state.clone(),
                to,
            }
            .into());
        }
        let step = plan
            .steps
            .iter()
            .find(|step| &step.id == step_id)
            .ok_or_else(|| PlanError::UnknownStep {
                plan_id: plan_id.clone(),
                step_id: step_id.clone(),
            })?;
        if !allowed_step_transition(&step.state, &to) {
            return Err(PlanError::InvalidStepTransition {
                plan_id: plan_id.clone(),
                step_id: step_id.clone(),
                from: step.state.clone(),
                to,
            }
            .into());
        }
        validate_cause_runtime(&cause, &projection.known_executions)?;
        self.record_domain_event(DomainEvent::PlanStepStateChanged {
            transition: PlanStepTransition {
                plan_id: plan_id.clone(),
                step_id: step_id.clone(),
                from: step.state.clone(),
                to,
                cause,
            },
        })?;
        self.plan(plan_id)
    }

    pub fn transition_plan(
        &mut self,
        plan_id: &PlanId,
        to: PlanState,
        cause: PlanTransitionCause,
    ) -> Result<PlanRecord, ConductorError> {
        let projection = self.plan_projection()?;
        let plan = projection
            .plans
            .get(plan_id)
            .ok_or_else(|| PlanError::UnknownPlan(plan_id.clone()))?;
        if !allowed_plan_transition(&plan.state, &to) {
            return Err(PlanError::InvalidTransition {
                plan_id: plan_id.clone(),
                from: plan.state.clone(),
                to,
            }
            .into());
        }
        if to == PlanState::Completed
            && plan
                .steps
                .iter()
                .any(|step| step.state != PlanStepState::Completed)
        {
            return Err(PlanError::InvalidTransition {
                plan_id: plan_id.clone(),
                from: plan.state.clone(),
                to,
            }
            .into());
        }
        if to == PlanState::Superseded
            && !projection
                .plans
                .values()
                .any(|candidate| candidate.supersedes.as_ref() == Some(plan_id))
        {
            return Err(PlanError::InvalidSuccessor(plan_id.clone()).into());
        }
        validate_cause_runtime(&cause, &projection.known_executions)?;
        self.record_domain_event(DomainEvent::PlanStateChanged {
            transition: PlanTransition {
                plan_id: plan_id.clone(),
                from: plan.state.clone(),
                to,
                cause,
            },
        })?;
        self.plan(plan_id)
    }

    pub fn create_successor_plan(
        &mut self,
        previous: &PlanId,
        objective_refs: BTreeSet<ObjectiveId>,
        steps: Vec<PlanStep>,
        cause: PlanTransitionCause,
    ) -> Result<PlanRecord, ConductorError> {
        let projection = self.plan_projection()?;
        let current = projection
            .plans
            .get(previous)
            .ok_or_else(|| PlanError::UnknownPlan(previous.clone()))?;
        if !valid_successor_predecessor_state(&current.state) {
            return Err(PlanError::InvalidSuccessor(previous.clone()).into());
        }
        validate_cause_runtime(&cause, &projection.known_executions)?;
        let successor = self.create_plan_record(objective_refs, steps, Some(previous.clone()))?;
        self.transition_plan(previous, PlanState::Superseded, cause)?;
        Ok(successor)
    }

    fn validate_plan_objectives(
        &self,
        objective_refs: &BTreeSet<ObjectiveId>,
        steps: &[PlanStep],
    ) -> Result<(), ConductorError> {
        let refs = objective_refs
            .iter()
            .chain(steps.iter().flat_map(|step| step.objective_refs.iter()));
        for objective_id in refs {
            let objective = self
                .objective(objective_id)
                .map_err(|_| PlanError::UnknownObjective(objective_id.clone()))?;
            if objective.workspace != self.workspace_id {
                return Err(PlanError::WrongObjectiveWorkspace(objective_id.clone()).into());
            }
        }
        Ok(())
    }
}

fn validate_record_runtime(plan: &PlanRecord) -> Result<(), PlanError> {
    if plan.steps.is_empty() {
        return Err(PlanError::EmptyPlan);
    }
    let mut ids = BTreeSet::new();
    for step in &plan.steps {
        if step.description.trim().is_empty() || step.state != PlanStepState::Proposed {
            return Err(PlanError::InvalidStep(step.id.clone()));
        }
        if !ids.insert(step.id.clone()) {
            return Err(PlanError::DuplicateStep(step.id.clone()));
        }
    }
    validate_dependencies_runtime(&plan.steps)
}

fn validate_dependencies_runtime(steps: &[PlanStep]) -> Result<(), PlanError> {
    let ids = steps
        .iter()
        .map(|step| step.id.clone())
        .collect::<BTreeSet<_>>();
    for step in steps {
        for dependency in &step.depends_on {
            if dependency == &step.id || !ids.contains(dependency) {
                return Err(PlanError::InvalidDependency {
                    step_id: step.id.clone(),
                    dependency: dependency.clone(),
                });
            }
        }
    }
    if has_cycle(steps) {
        return Err(PlanError::DependencyCycle);
    }
    Ok(())
}

fn validate_record(
    plan: &PlanRecord,
    objectives: &BTreeMap<ObjectiveId, WorkspaceId>,
) -> Result<(), JournalError> {
    validate_record_runtime(plan).map_err(|error| invalid_event(error.to_string()))?;
    for objective_id in plan.objective_refs.iter().chain(
        plan.steps
            .iter()
            .flat_map(|step| step.objective_refs.iter()),
    ) {
        let workspace = objectives.get(objective_id).ok_or_else(|| {
            invalid_event(format!(
                "plan {} references unknown objective {objective_id}",
                plan.id
            ))
        })?;
        if workspace != &plan.workspace {
            return Err(invalid_event(format!(
                "plan {} references objective {objective_id} in another workspace",
                plan.id
            )));
        }
    }
    Ok(())
}

fn has_cycle(steps: &[PlanStep]) -> bool {
    let graph = steps
        .iter()
        .map(|step| (step.id.clone(), step.depends_on.clone()))
        .collect::<BTreeMap<_, _>>();
    fn visit(
        id: &PlanStepId,
        graph: &BTreeMap<PlanStepId, BTreeSet<PlanStepId>>,
        visiting: &mut BTreeSet<PlanStepId>,
        done: &mut BTreeSet<PlanStepId>,
    ) -> bool {
        if done.contains(id) {
            return false;
        }
        if !visiting.insert(id.clone()) {
            return true;
        }
        if graph
            .get(id)
            .is_some_and(|deps| deps.iter().any(|dep| visit(dep, graph, visiting, done)))
        {
            return true;
        }
        visiting.remove(id);
        done.insert(id.clone());
        false
    }
    let mut visiting = BTreeSet::new();
    let mut done = BTreeSet::new();
    graph
        .keys()
        .any(|id| visit(id, &graph, &mut visiting, &mut done))
}

fn validate_cause(
    cause: &PlanTransitionCause,
    known_executions: &BTreeSet<ExecutionId>,
) -> Result<(), JournalError> {
    match cause {
        PlanTransitionCause::AgentAction { execution_id }
        | PlanTransitionCause::ExecutionOutcome { execution_id } => {
            if !known_executions.contains(execution_id) {
                return Err(invalid_event(format!(
                    "plan transition cause references unknown execution {execution_id}"
                )));
            }
        }
        PlanTransitionCause::EvidenceAssessment { evidence_ref }
            if evidence_ref.trim().is_empty() =>
        {
            return Err(invalid_event("plan evidence cause is empty"));
        }
        PlanTransitionCause::Policy { description } if description.trim().is_empty() => {
            return Err(invalid_event("plan policy cause is empty"));
        }
        _ => {}
    }
    Ok(())
}

fn validate_cause_runtime(
    cause: &PlanTransitionCause,
    known_executions: &BTreeSet<ExecutionId>,
) -> Result<(), PlanError> {
    match cause {
        PlanTransitionCause::AgentAction { execution_id }
        | PlanTransitionCause::ExecutionOutcome { execution_id }
            if !known_executions.contains(execution_id) =>
        {
            Err(PlanError::UnknownExecution(execution_id.clone()))
        }
        PlanTransitionCause::EvidenceAssessment { evidence_ref }
            if evidence_ref.trim().is_empty() =>
        {
            Err(PlanError::InvalidCause)
        }
        PlanTransitionCause::Policy { description } if description.trim().is_empty() => {
            Err(PlanError::InvalidCause)
        }
        _ => Ok(()),
    }
}

fn valid_successor_predecessor_state(state: &PlanState) -> bool {
    matches!(
        state,
        PlanState::Active | PlanState::Failed | PlanState::Invalidated | PlanState::Abandoned
    )
}

fn allowed_plan_transition(from: &PlanState, to: &PlanState) -> bool {
    matches!(
        (from, to),
        (PlanState::Draft, PlanState::Abandoned)
            | (PlanState::Active, PlanState::Completed)
            | (PlanState::Active, PlanState::Failed)
            | (PlanState::Active, PlanState::Invalidated)
            | (PlanState::Active, PlanState::Abandoned)
            | (PlanState::Active, PlanState::Superseded)
            | (PlanState::Failed, PlanState::Superseded)
            | (PlanState::Invalidated, PlanState::Superseded)
            | (PlanState::Abandoned, PlanState::Superseded)
    )
}

fn allowed_step_transition(from: &PlanStepState, to: &PlanStepState) -> bool {
    matches!(
        (from, to),
        (PlanStepState::Active, PlanStepState::Completed)
            | (PlanStepState::Active, PlanStepState::Failed)
            | (PlanStepState::Active, PlanStepState::Invalidated)
            | (PlanStepState::Active, PlanStepState::Abandoned)
            | (PlanStepState::Committed, PlanStepState::Invalidated)
            | (PlanStepState::Committed, PlanStepState::Abandoned)
    )
}

fn next_plan_id(projection: &PlanProjection) -> PlanId {
    PlanId::parse(format!("plan-{}", projection.plans.len() + 1)).expect("generated plan id")
}

fn invalid_event(message: impl Into<String>) -> JournalError {
    JournalError::InvalidEvent(message.into())
}
