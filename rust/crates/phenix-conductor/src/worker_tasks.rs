use crate::{
    ConductorError, ConductorRuntime, DomainEvent, JournalError, RuntimeJournal, WorkerProfileId,
};
use phenix_core::{
    ExactReference, ExecutionAuthority, ExecutionId, ExecutionState, ObjectiveId, ObjectiveState,
    PlanId, PlanState, PlanStepId, PlanStepState,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkerTaskId(String);

impl WorkerTaskId {
    pub fn parse(value: impl Into<String>) -> Result<Self, WorkerTaskError> {
        let value = value.into();
        if value.trim().is_empty() {
            Err(WorkerTaskError::InvalidId)
        } else {
            Ok(Self(value))
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for WorkerTaskId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkerPlanStepRef {
    pub plan_id: PlanId,
    pub step_id: PlanStepId,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum WorkerTaskState {
    Pending,
    Running {
        execution_id: ExecutionId,
    },
    Completed {
        execution_id: ExecutionId,
        #[serde(default)]
        result_refs: Vec<ExactReference>,
    },
    Failed {
        execution_id: ExecutionId,
        cause: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkerTaskRecord {
    pub id: WorkerTaskId,
    pub parent_execution: ExecutionId,
    pub primary_objective: ObjectiveId,
    #[serde(default)]
    pub supporting_objectives: BTreeSet<ObjectiveId>,
    pub plan_step: Option<WorkerPlanStepRef>,
    pub description: String,
    pub profile_id: WorkerProfileId,
    #[serde(default)]
    pub depends_on: BTreeSet<WorkerTaskId>,
    #[serde(default)]
    pub input_refs: Vec<ExactReference>,
    pub expected_result_schema: Value,
    pub delegated_authority: ExecutionAuthority,
    pub state: WorkerTaskState,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WorkerTaskRequest {
    pub primary_objective: ObjectiveId,
    pub supporting_objectives: BTreeSet<ObjectiveId>,
    pub plan_step: Option<WorkerPlanStepRef>,
    pub description: String,
    pub profile_id: WorkerProfileId,
    pub depends_on: BTreeSet<WorkerTaskId>,
    pub input_refs: Vec<ExactReference>,
    pub expected_result_schema: Value,
    pub delegated_authority: ExecutionAuthority,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorkerTaskError {
    InvalidId,
    InvalidDescription,
    UnknownTask(WorkerTaskId),
    DuplicateTask(WorkerTaskId),
    UnknownDependency(WorkerTaskId),
    DependencyCycle,
    Blocked(WorkerTaskId),
    ObjectiveScope(ObjectiveId),
    PlanScope {
        plan_id: PlanId,
        step_id: PlanStepId,
    },
    InvalidState(WorkerTaskId),
    InvalidFailureCause,
    InvalidResult(String),
}

impl Display for WorkerTaskError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidId => f.write_str("worker task identifier must not be empty"),
            Self::InvalidDescription => f.write_str("worker task description must not be empty"),
            Self::UnknownTask(id) => write!(f, "unknown worker task: {id}"),
            Self::DuplicateTask(id) => write!(f, "worker task already exists: {id}"),
            Self::UnknownDependency(id) => write!(f, "unknown worker task dependency: {id}"),
            Self::DependencyCycle => f.write_str("worker task dependencies contain a cycle"),
            Self::Blocked(id) => write!(f, "worker task is blocked: {id}"),
            Self::ObjectiveScope(id) => {
                write!(f, "worker task objective is outside parent scope: {id}")
            }
            Self::PlanScope { plan_id, step_id } => write!(
                f,
                "worker task plan step is not runnable: {plan_id}/{step_id}"
            ),
            Self::InvalidState(id) => write!(f, "worker task has invalid lifecycle state: {id}"),
            Self::InvalidFailureCause => f.write_str("worker task failure cause must not be empty"),
            Self::InvalidResult(message) => write!(f, "invalid worker result: {message}"),
        }
    }
}

impl Error for WorkerTaskError {}

#[derive(Default)]
struct WorkerTaskProjection {
    tasks: BTreeMap<WorkerTaskId, WorkerTaskRecord>,
    execution_parents: BTreeMap<ExecutionId, Option<ExecutionId>>,
    worker_profiles: BTreeMap<ExecutionId, WorkerProfileId>,
    bound_executions: BTreeSet<ExecutionId>,
    attempt_group_roots: BTreeMap<phenix_core::AttemptGroupId, ExecutionId>,
    attempt_roots: BTreeMap<ExecutionId, ExecutionId>,
}

impl WorkerTaskProjection {
    fn same_attempt_lineage(&self, initial: &ExecutionId, candidate: &ExecutionId) -> bool {
        if initial == candidate {
            return true;
        }
        let initial_root = self.attempt_roots.get(initial).unwrap_or(initial);
        let candidate_root = self.attempt_roots.get(candidate).unwrap_or(candidate);
        initial_root == candidate_root
    }

    fn apply(&mut self, event: &DomainEvent) -> Result<(), JournalError> {
        match event {
            DomainEvent::ExecutionCreated { execution, .. } => {
                self.execution_parents
                    .insert(execution.id.clone(), execution.parent_execution.clone());
            }
            DomainEvent::WorkerProfileBound {
                execution_id,
                profile_id,
            } => {
                if !self.execution_parents.contains_key(execution_id) {
                    return Err(invalid(format!(
                        "worker profile bound to unknown execution: {execution_id}"
                    )));
                }
                self.worker_profiles
                    .insert(execution_id.clone(), profile_id.clone());
            }
            DomainEvent::AttemptGroupCreated { group } => {
                let root =
                    group.attempts.first().cloned().ok_or_else(|| {
                        invalid("worker task retry group has no initial execution")
                    })?;
                self.attempt_group_roots
                    .insert(group.id.clone(), root.clone());
                for attempt in &group.attempts {
                    self.attempt_roots.insert(attempt.clone(), root.clone());
                }
            }
            DomainEvent::AttemptRetryStarted {
                group_id,
                execution_id,
            } => {
                if !self.execution_parents.contains_key(execution_id) {
                    return Err(invalid(format!(
                        "worker task retry references unknown execution: {execution_id}"
                    )));
                }
                let root = self.attempt_group_roots.get(group_id).ok_or_else(|| {
                    invalid(format!(
                        "worker task retry references unknown attempt group: {group_id}"
                    ))
                })?;
                self.attempt_roots
                    .insert(execution_id.clone(), root.clone());
            }
            DomainEvent::WorkerTaskCreated { task } => {
                if task.description.trim().is_empty() || task.state != WorkerTaskState::Pending {
                    return Err(invalid(
                        "worker task must be created pending with a description",
                    ));
                }
                if self.tasks.insert(task.id.clone(), task.clone()).is_some() {
                    return Err(invalid(format!("duplicate worker task id: {}", task.id)));
                }
            }
            DomainEvent::WorkerTaskStarted {
                task_id,
                execution_id,
            } => {
                let execution_parent =
                    self.execution_parents.get(execution_id).ok_or_else(|| {
                        invalid(format!(
                            "worker task {task_id} started with unknown execution {execution_id}"
                        ))
                    })?;
                let task = self
                    .tasks
                    .get(task_id)
                    .ok_or_else(|| invalid(format!("unknown worker task: {task_id}")))?;
                if task.state != WorkerTaskState::Pending {
                    return Err(invalid(format!(
                        "worker task started from non-pending state: {task_id}"
                    )));
                }
                if execution_parent.as_ref() != Some(&task.parent_execution) {
                    return Err(invalid(format!(
                        "worker task {task_id} execution {execution_id} does not belong to parent {}",
                        task.parent_execution
                    )));
                }
                if self.worker_profiles.get(execution_id) != Some(&task.profile_id) {
                    return Err(invalid(format!(
                        "worker task {task_id} execution {execution_id} does not use profile {}",
                        task.profile_id
                    )));
                }
                if !self.bound_executions.insert(execution_id.clone()) {
                    return Err(invalid(format!(
                        "worker execution already bound to another task: {execution_id}"
                    )));
                }
                self.tasks
                    .get_mut(task_id)
                    .expect("worker task was validated above")
                    .state = WorkerTaskState::Running {
                    execution_id: execution_id.clone(),
                };
            }
            DomainEvent::WorkerTaskCompleted {
                task_id,
                execution_id,
                result_refs,
            } => {
                let task = self
                    .tasks
                    .get(task_id)
                    .ok_or_else(|| invalid(format!("unknown worker task: {task_id}")))?;
                let WorkerTaskState::Running {
                    execution_id: initial_execution,
                } = &task.state
                else {
                    return Err(invalid(format!(
                        "worker task completion does not match running execution: {task_id}"
                    )));
                };
                if !self.same_attempt_lineage(initial_execution, execution_id) {
                    return Err(invalid(format!(
                        "worker task completion is outside its retry lineage: {task_id}"
                    )));
                }
                self.tasks
                    .get_mut(task_id)
                    .expect("worker task was validated above")
                    .state = WorkerTaskState::Completed {
                    execution_id: execution_id.clone(),
                    result_refs: result_refs.clone(),
                };
            }
            DomainEvent::WorkerTaskFailed {
                task_id,
                execution_id,
                cause,
            } => {
                if cause.trim().is_empty() {
                    return Err(invalid("worker task failure cause must not be empty"));
                }
                let task = self
                    .tasks
                    .get(task_id)
                    .ok_or_else(|| invalid(format!("unknown worker task: {task_id}")))?;
                let WorkerTaskState::Running {
                    execution_id: initial_execution,
                } = &task.state
                else {
                    return Err(invalid(format!(
                        "worker task failure does not match running execution: {task_id}"
                    )));
                };
                if !self.same_attempt_lineage(initial_execution, execution_id) {
                    return Err(invalid(format!(
                        "worker task failure is outside its retry lineage: {task_id}"
                    )));
                }
                self.tasks
                    .get_mut(task_id)
                    .expect("worker task was validated above")
                    .state = WorkerTaskState::Failed {
                    execution_id: execution_id.clone(),
                    cause: cause.clone(),
                };
            }
            _ => {}
        }
        Ok(())
    }

    fn validate_dependencies(&self) -> Result<(), JournalError> {
        for task in self.tasks.values() {
            for dependency in &task.depends_on {
                if !self.tasks.contains_key(dependency) {
                    return Err(invalid(format!(
                        "worker task {} references unknown dependency {dependency}",
                        task.id
                    )));
                }
            }
        }
        let mut visiting = BTreeSet::new();
        let mut visited = BTreeSet::new();
        for id in self.tasks.keys() {
            self.visit(id, &mut visiting, &mut visited)?;
        }
        Ok(())
    }

    fn visit(
        &self,
        id: &WorkerTaskId,
        visiting: &mut BTreeSet<WorkerTaskId>,
        visited: &mut BTreeSet<WorkerTaskId>,
    ) -> Result<(), JournalError> {
        if visited.contains(id) {
            return Ok(());
        }
        if !visiting.insert(id.clone()) {
            return Err(invalid("worker task dependency graph contains a cycle"));
        }
        let task = self
            .tasks
            .get(id)
            .ok_or_else(|| invalid(format!("unknown worker task: {id}")))?;
        for dependency in &task.depends_on {
            self.visit(dependency, visiting, visited)?;
        }
        visiting.remove(id);
        visited.insert(id.clone());
        Ok(())
    }
}

fn invalid(message: impl Into<String>) -> JournalError {
    JournalError::InvalidEvent(message.into())
}

pub(crate) fn validate_journal_worker_tasks(journal: &RuntimeJournal) -> Result<(), JournalError> {
    let mut projection = WorkerTaskProjection::default();
    for entry in &journal.entries {
        projection.apply(&entry.event)?;
    }
    projection.validate_dependencies()
}

impl ConductorRuntime {
    fn worker_task_projection(&self) -> Result<WorkerTaskProjection, ConductorError> {
        let mut projection = WorkerTaskProjection::default();
        for entry in &self.journal.entries {
            projection.apply(&entry.event)?;
        }
        projection.validate_dependencies()?;
        Ok(projection)
    }

    pub fn worker_tasks(&self) -> Result<Vec<WorkerTaskRecord>, ConductorError> {
        Ok(self.worker_task_projection()?.tasks.into_values().collect())
    }

    pub fn worker_task(&self, id: &WorkerTaskId) -> Result<WorkerTaskRecord, ConductorError> {
        self.worker_task_projection()?
            .tasks
            .get(id)
            .cloned()
            .ok_or_else(|| WorkerTaskError::UnknownTask(id.clone()).into())
    }

    pub fn create_worker_task(
        &mut self,
        parent_execution: &ExecutionId,
        request: WorkerTaskRequest,
    ) -> Result<WorkerTaskRecord, ConductorError> {
        if request.description.trim().is_empty() {
            return Err(WorkerTaskError::InvalidDescription.into());
        }
        let assignment = self
            .execution_objectives(parent_execution)?
            .ok_or_else(|| {
                crate::ObjectiveError::MissingExecutionObjective(parent_execution.clone())
            })?;
        if assignment.primary != request.primary_objective {
            return Err(WorkerTaskError::ObjectiveScope(request.primary_objective).into());
        }
        if !request
            .supporting_objectives
            .is_subset(&assignment.supporting)
        {
            let outside = request
                .supporting_objectives
                .difference(&assignment.supporting)
                .next()
                .expect("non-subset has an element")
                .clone();
            return Err(WorkerTaskError::ObjectiveScope(outside).into());
        }
        for objective_id in
            std::iter::once(&request.primary_objective).chain(request.supporting_objectives.iter())
        {
            if self.objective(objective_id)?.state != ObjectiveState::Active {
                return Err(WorkerTaskError::ObjectiveScope(objective_id.clone()).into());
            }
        }
        self.configuration_for_execution(parent_execution)?
            .resolve_worker_profile(&request.profile_id)?;
        for reference in &request.input_refs {
            self.resolve_exact_reference(reference)?;
        }
        let projection = self.worker_task_projection()?;
        for dependency in &request.depends_on {
            if !projection.tasks.contains_key(dependency) {
                return Err(WorkerTaskError::UnknownDependency(dependency.clone()).into());
            }
        }
        if let Some(plan_step) = &request.plan_step {
            self.validate_worker_plan_scope(&request.primary_objective, plan_step)?;
        }
        let next = projection
            .tasks
            .keys()
            .filter_map(|id| {
                id.as_str()
                    .strip_prefix("worker-task-")
                    .and_then(|value| value.parse::<u64>().ok())
            })
            .max()
            .unwrap_or(0)
            + 1;
        let task = WorkerTaskRecord {
            id: WorkerTaskId::parse(format!("worker-task-{next}"))
                .expect("generated worker task id"),
            parent_execution: parent_execution.clone(),
            primary_objective: request.primary_objective,
            supporting_objectives: request.supporting_objectives,
            plan_step: request.plan_step,
            description: request.description,
            profile_id: request.profile_id,
            depends_on: request.depends_on,
            input_refs: request.input_refs,
            expected_result_schema: request.expected_result_schema,
            delegated_authority: request.delegated_authority,
            state: WorkerTaskState::Pending,
        };
        self.record_domain_event(DomainEvent::WorkerTaskCreated { task: task.clone() })?;
        Ok(task)
    }

    fn validate_worker_plan_scope(
        &self,
        primary: &ObjectiveId,
        plan_step: &WorkerPlanStepRef,
    ) -> Result<(), ConductorError> {
        let plan = self.plan(&plan_step.plan_id)?;
        let step = plan
            .steps
            .iter()
            .find(|step| step.id == plan_step.step_id)
            .ok_or_else(|| WorkerTaskError::PlanScope {
                plan_id: plan_step.plan_id.clone(),
                step_id: plan_step.step_id.clone(),
            })?;
        let objective_matches =
            plan.objective_refs.contains(primary) || step.objective_refs.contains(primary);
        let runnable_state = plan.state == PlanState::Active
            && matches!(step.state, PlanStepState::Committed | PlanStepState::Active);
        let dependencies_complete = step.depends_on.iter().all(|dependency| {
            plan.steps
                .iter()
                .find(|candidate| candidate.id == *dependency)
                .is_some_and(|candidate| candidate.state == PlanStepState::Completed)
        });
        if !objective_matches || !runnable_state || !dependencies_complete {
            return Err(WorkerTaskError::PlanScope {
                plan_id: plan_step.plan_id.clone(),
                step_id: plan_step.step_id.clone(),
            }
            .into());
        }
        Ok(())
    }

    pub fn worker_task_is_runnable(&self, id: &WorkerTaskId) -> Result<bool, ConductorError> {
        let projection = self.worker_task_projection()?;
        let task = projection
            .tasks
            .get(id)
            .ok_or_else(|| WorkerTaskError::UnknownTask(id.clone()))?;
        if task.state != WorkerTaskState::Pending {
            return Ok(false);
        }
        if task.depends_on.iter().any(|dependency| {
            !matches!(
                projection.tasks.get(dependency).map(|task| &task.state),
                Some(WorkerTaskState::Completed { .. })
            )
        }) {
            return Ok(false);
        }
        if self.objective(&task.primary_objective)?.state != ObjectiveState::Active {
            return Ok(false);
        }
        if task.supporting_objectives.iter().any(|id| {
            self.objective(id)
                .map(|objective| objective.state != ObjectiveState::Active)
                .unwrap_or(true)
        }) {
            return Ok(false);
        }
        if let Some(plan_step) = &task.plan_step {
            if self
                .validate_worker_plan_scope(&task.primary_objective, plan_step)
                .is_err()
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub fn runnable_worker_tasks(&self) -> Result<Vec<WorkerTaskRecord>, ConductorError> {
        let mut runnable = self
            .worker_tasks()?
            .into_iter()
            .filter_map(|task| {
                self.worker_task_is_runnable(&task.id)
                    .ok()
                    .filter(|runnable| *runnable)
                    .map(|_| task)
            })
            .collect::<Vec<_>>();
        runnable.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(runnable)
    }

    pub fn start_worker_task(
        &mut self,
        id: &WorkerTaskId,
    ) -> Result<phenix_core::ExecutionSummary, ConductorError> {
        let task = self.worker_task(id)?;
        if !self.worker_task_is_runnable(id)? {
            return Err(WorkerTaskError::Blocked(id.clone()).into());
        }
        let child = self.start_worker_profile_with_restrictions(
            &task.parent_execution,
            &task.profile_id,
            task.description.clone(),
            &task.delegated_authority,
        )?;
        if let Some(plan_step) = &task.plan_step {
            self.assign_execution_to_plan_step(&child.id, &plan_step.plan_id, &plan_step.step_id)?;
        }
        self.project_execution_context(&child.id)?;
        self.record_domain_event(DomainEvent::WorkerTaskStarted {
            task_id: id.clone(),
            execution_id: child.id.clone(),
        })?;
        Ok(child)
    }

    pub(crate) fn current_worker_task_execution(
        &self,
        initial_execution: &ExecutionId,
    ) -> ExecutionId {
        self.attempt_groups
            .values()
            .find(|group| group.contains_execution(initial_execution))
            .and_then(phenix_core::AttemptGroup::latest_execution)
            .cloned()
            .unwrap_or_else(|| initial_execution.clone())
    }

    pub fn complete_worker_task(
        &mut self,
        id: &WorkerTaskId,
        result_refs: Vec<ExactReference>,
    ) -> Result<WorkerTaskRecord, ConductorError> {
        let task = self.worker_task(id)?;
        let WorkerTaskState::Running {
            execution_id: initial_execution,
        } = task.state
        else {
            return Err(WorkerTaskError::InvalidState(id.clone()).into());
        };
        let execution_id = self.current_worker_task_execution(&initial_execution);
        let execution = self
            .executions
            .get(&execution_id)
            .ok_or_else(|| ConductorError::UnknownExecution(execution_id.clone()))?;
        if execution.summary.state != ExecutionState::Completed {
            return Err(WorkerTaskError::InvalidState(id.clone()).into());
        }
        let expected_result_refs = self.worker_completion_result_refs(id, &execution_id)?;
        if result_refs != expected_result_refs {
            return Err(WorkerTaskError::InvalidResult(
                "completion references must exactly match the recorded worker result".to_owned(),
            )
            .into());
        }
        self.record_domain_event(DomainEvent::WorkerTaskCompleted {
            task_id: id.clone(),
            execution_id,
            result_refs,
        })?;
        self.worker_task(id)
    }

    pub fn fail_worker_task(
        &mut self,
        id: &WorkerTaskId,
        cause: impl Into<String>,
    ) -> Result<WorkerTaskRecord, ConductorError> {
        let cause = cause.into();
        if cause.trim().is_empty() {
            return Err(WorkerTaskError::InvalidFailureCause.into());
        }
        let task = self.worker_task(id)?;
        let WorkerTaskState::Running {
            execution_id: initial_execution,
        } = task.state
        else {
            return Err(WorkerTaskError::InvalidState(id.clone()).into());
        };
        let execution_id = self.current_worker_task_execution(&initial_execution);
        let state = self
            .executions
            .get(&execution_id)
            .ok_or_else(|| ConductorError::UnknownExecution(execution_id.clone()))?
            .summary
            .state
            .clone();
        if !matches!(
            state,
            ExecutionState::Failed | ExecutionState::Cancelled | ExecutionState::Interrupted
        ) {
            return Err(WorkerTaskError::InvalidState(id.clone()).into());
        }
        self.record_domain_event(DomainEvent::WorkerTaskFailed {
            task_id: id.clone(),
            execution_id,
            cause,
        })?;
        self.worker_task(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(id: &str, dependencies: &[&str]) -> WorkerTaskRecord {
        WorkerTaskRecord {
            id: WorkerTaskId::parse(id).unwrap(),
            parent_execution: ExecutionId::parse("execution-parent").unwrap(),
            primary_objective: ObjectiveId::parse("objective-1").unwrap(),
            supporting_objectives: BTreeSet::new(),
            plan_step: None,
            description: "bounded work".to_owned(),
            profile_id: WorkerProfileId::parse("worker.review").unwrap(),
            depends_on: dependencies
                .iter()
                .map(|id| WorkerTaskId::parse(*id).unwrap())
                .collect(),
            input_refs: Vec::new(),
            expected_result_schema: serde_json::json!({"type": "object"}),
            delegated_authority: ExecutionAuthority::read_only(),
            state: WorkerTaskState::Pending,
        }
    }

    #[test]
    fn worker_task_projection_rejects_dependency_cycles() {
        let mut projection = WorkerTaskProjection::default();
        projection
            .apply(&DomainEvent::WorkerTaskCreated {
                task: task("worker-task-1", &["worker-task-2"]),
            })
            .unwrap();
        projection
            .apply(&DomainEvent::WorkerTaskCreated {
                task: task("worker-task-2", &["worker-task-1"]),
            })
            .unwrap();
        assert!(
            matches!(projection.validate_dependencies(), Err(JournalError::InvalidEvent(message)) if message.contains("cycle"))
        );
    }

    #[test]
    fn worker_task_projection_requires_exact_started_execution() {
        let mut projection = WorkerTaskProjection::default();
        projection
            .apply(&DomainEvent::WorkerTaskCreated {
                task: task("worker-task-1", &[]),
            })
            .unwrap();
        let result = projection.apply(&DomainEvent::WorkerTaskStarted {
            task_id: WorkerTaskId::parse("worker-task-1").unwrap(),
            execution_id: ExecutionId::parse("execution-missing").unwrap(),
        });
        assert!(
            matches!(result, Err(JournalError::InvalidEvent(message)) if message.contains("unknown execution"))
        );
    }

    #[test]
    fn worker_task_projection_rejects_started_execution_from_wrong_parent() {
        let mut projection = WorkerTaskProjection::default();
        projection
            .apply(&DomainEvent::WorkerTaskCreated {
                task: task("worker-task-1", &[]),
            })
            .unwrap();
        let child = ExecutionId::parse("execution-child").unwrap();
        projection.execution_parents.insert(
            child.clone(),
            Some(ExecutionId::parse("execution-other-parent").unwrap()),
        );
        projection.worker_profiles.insert(
            child.clone(),
            WorkerProfileId::parse("worker.review").unwrap(),
        );

        let result = projection.apply(&DomainEvent::WorkerTaskStarted {
            task_id: WorkerTaskId::parse("worker-task-1").unwrap(),
            execution_id: child,
        });
        assert!(
            matches!(result, Err(JournalError::InvalidEvent(message)) if message.contains("does not belong to parent"))
        );
    }

    #[test]
    fn worker_task_projection_rejects_started_execution_with_wrong_profile() {
        let mut projection = WorkerTaskProjection::default();
        projection
            .apply(&DomainEvent::WorkerTaskCreated {
                task: task("worker-task-1", &[]),
            })
            .unwrap();
        let child = ExecutionId::parse("execution-child").unwrap();
        projection.execution_parents.insert(
            child.clone(),
            Some(ExecutionId::parse("execution-parent").unwrap()),
        );
        projection.worker_profiles.insert(
            child.clone(),
            WorkerProfileId::parse("worker.other").unwrap(),
        );

        let result = projection.apply(&DomainEvent::WorkerTaskStarted {
            task_id: WorkerTaskId::parse("worker-task-1").unwrap(),
            execution_id: child,
        });
        assert!(
            matches!(result, Err(JournalError::InvalidEvent(message)) if message.contains("does not use profile"))
        );
    }

    #[test]
    fn worker_task_projection_rejects_execution_bound_to_two_tasks() {
        let mut projection = WorkerTaskProjection::default();
        for id in ["worker-task-1", "worker-task-2"] {
            projection
                .apply(&DomainEvent::WorkerTaskCreated {
                    task: task(id, &[]),
                })
                .unwrap();
        }
        let child = ExecutionId::parse("execution-child").unwrap();
        projection.execution_parents.insert(
            child.clone(),
            Some(ExecutionId::parse("execution-parent").unwrap()),
        );
        projection.worker_profiles.insert(
            child.clone(),
            WorkerProfileId::parse("worker.review").unwrap(),
        );
        projection
            .apply(&DomainEvent::WorkerTaskStarted {
                task_id: WorkerTaskId::parse("worker-task-1").unwrap(),
                execution_id: child.clone(),
            })
            .unwrap();

        let result = projection.apply(&DomainEvent::WorkerTaskStarted {
            task_id: WorkerTaskId::parse("worker-task-2").unwrap(),
            execution_id: child,
        });
        assert!(
            matches!(result, Err(JournalError::InvalidEvent(message)) if message.contains("already bound"))
        );
    }

    #[test]
    fn worker_task_projection_accepts_completion_from_canonical_retry_lineage() {
        let mut projection = WorkerTaskProjection::default();
        projection
            .apply(&DomainEvent::WorkerTaskCreated {
                task: task("worker-task-1", &[]),
            })
            .unwrap();
        let initial = ExecutionId::parse("execution-initial").unwrap();
        projection.execution_parents.insert(
            initial.clone(),
            Some(ExecutionId::parse("execution-parent").unwrap()),
        );
        projection.worker_profiles.insert(
            initial.clone(),
            WorkerProfileId::parse("worker.review").unwrap(),
        );
        projection
            .apply(&DomainEvent::WorkerTaskStarted {
                task_id: WorkerTaskId::parse("worker-task-1").unwrap(),
                execution_id: initial.clone(),
            })
            .unwrap();

        let group_id = phenix_core::AttemptGroupId::parse("attempt-group-worker").unwrap();
        let group = phenix_core::AttemptGroup::from_first_failure(
            group_id.clone(),
            ExecutionId::parse("execution-parent").unwrap(),
            phenix_core::CallableId::parse("agent.worker").unwrap(),
            "same bounded approach",
            phenix_core::FailureAttemptSummary {
                execution_id: initial,
                attempt: 1,
                approach: "same bounded approach".to_owned(),
                failure_at: "tool".to_owned(),
                reason: "transient failure".to_owned(),
                completed_work: Vec::new(),
            },
        );
        projection
            .apply(&DomainEvent::AttemptGroupCreated { group })
            .unwrap();
        let retry = ExecutionId::parse("execution-retry").unwrap();
        projection.execution_parents.insert(
            retry.clone(),
            Some(ExecutionId::parse("execution-parent").unwrap()),
        );
        projection
            .apply(&DomainEvent::AttemptRetryStarted {
                group_id,
                execution_id: retry.clone(),
            })
            .unwrap();
        projection
            .apply(&DomainEvent::WorkerTaskCompleted {
                task_id: WorkerTaskId::parse("worker-task-1").unwrap(),
                execution_id: retry.clone(),
                result_refs: Vec::new(),
            })
            .unwrap();

        assert!(matches!(
            projection.tasks[&WorkerTaskId::parse("worker-task-1").unwrap()].state,
            WorkerTaskState::Completed { ref execution_id, .. } if execution_id == &retry
        ));
    }
}
