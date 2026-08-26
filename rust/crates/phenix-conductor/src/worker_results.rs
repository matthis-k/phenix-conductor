use crate::{
    ConductorError, ConductorRuntime, DomainEvent, JournalError, RuntimeJournal, WorkerTaskError,
    WorkerTaskId, WorkerTaskRecord, WorkerTaskState,
};
use phenix_core::{ExactReference, ExecutionId, ExecutionState};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkerResultEnvelope {
    pub task_id: WorkerTaskId,
    pub execution_id: ExecutionId,
    pub output: Value,
    #[serde(default)]
    pub evidence_refs: Vec<ExactReference>,
    #[serde(default)]
    pub artifact_refs: Vec<ExactReference>,
}

impl WorkerResultEnvelope {
    #[must_use]
    pub fn exact_refs(&self) -> Vec<ExactReference> {
        self.evidence_refs
            .iter()
            .chain(self.artifact_refs.iter())
            .cloned()
            .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum WorkerVerificationResult {
    Passed {
        verifier_execution_id: ExecutionId,
        #[serde(default)]
        evidence_refs: Vec<ExactReference>,
    },
    Failed {
        verifier_execution_id: ExecutionId,
        reason: String,
        #[serde(default)]
        evidence_refs: Vec<ExactReference>,
    },
}

impl WorkerVerificationResult {
    pub fn validate(&self) -> Result<(), WorkerResultError> {
        if let Self::Failed { reason, .. } = self {
            if reason.trim().is_empty() {
                return Err(WorkerResultError::InvalidVerificationFailure);
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn verifier_execution_id(&self) -> &ExecutionId {
        match self {
            Self::Passed {
                verifier_execution_id,
                ..
            }
            | Self::Failed {
                verifier_execution_id,
                ..
            } => verifier_execution_id,
        }
    }

    #[must_use]
    pub fn evidence_refs(&self) -> &[ExactReference] {
        match self {
            Self::Passed { evidence_refs, .. } | Self::Failed { evidence_refs, .. } => {
                evidence_refs
            }
        }
    }

    #[must_use]
    pub fn passed(&self) -> bool {
        matches!(self, Self::Passed { .. })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkerFailureAnalysis {
    pub analyzer_execution_id: ExecutionId,
    pub diagnosis: String,
    #[serde(default)]
    pub evidence_refs: Vec<ExactReference>,
    pub proposed_action: WorkerFailureAction,
}

impl WorkerFailureAnalysis {
    pub fn validate(&self) -> Result<(), WorkerResultError> {
        if self.diagnosis.trim().is_empty() {
            return Err(WorkerResultError::InvalidFailureAnalysis);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerFailureAction {
    Retry,
    SuccessorTask,
    InvalidatePlan,
    FailPlan,
    Continue,
    FailParent,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkerParentResultProjection {
    pub task_id: WorkerTaskId,
    pub execution_id: ExecutionId,
    pub output: Value,
    #[serde(default)]
    pub evidence_refs: Vec<ExactReference>,
    #[serde(default)]
    pub artifact_refs: Vec<ExactReference>,
    pub verification: Option<WorkerVerificationResult>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorkerResultError {
    InvalidFailureAnalysis,
    InvalidVerificationFailure,
    InvalidResultSchema(String),
    ResultSchemaMismatch(String),
}

impl Display for WorkerResultError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFailureAnalysis => {
                f.write_str("worker failure analysis diagnosis must not be empty")
            }
            Self::InvalidVerificationFailure => {
                f.write_str("failed worker verification reason must not be empty")
            }
            Self::InvalidResultSchema(message) => {
                write!(f, "invalid worker result schema: {message}")
            }
            Self::ResultSchemaMismatch(message) => {
                write!(f, "worker result does not match expected schema: {message}")
            }
        }
    }
}

impl Error for WorkerResultError {}

#[derive(Default)]
struct WorkerResultProjection {
    tasks: BTreeMap<WorkerTaskId, WorkerTaskRecord>,
    executions: BTreeSet<ExecutionId>,
    started: BTreeMap<WorkerTaskId, ExecutionId>,
    verification_required: BTreeSet<WorkerTaskId>,
    results: BTreeMap<WorkerTaskId, WorkerResultEnvelope>,
    verifications: BTreeMap<WorkerTaskId, Vec<WorkerVerificationResult>>,
    failure_analyses: BTreeMap<WorkerTaskId, Vec<WorkerFailureAnalysis>>,
}

impl WorkerResultProjection {
    fn apply(&mut self, event: &DomainEvent) -> Result<(), JournalError> {
        match event {
            DomainEvent::ExecutionCreated { execution, .. } => {
                self.executions.insert(execution.id.clone());
            }
            DomainEvent::WorkerTaskCreated { task } => {
                self.tasks.insert(task.id.clone(), task.clone());
            }
            DomainEvent::WorkerTaskStarted {
                task_id,
                execution_id,
            } => {
                self.started.insert(task_id.clone(), execution_id.clone());
            }
            DomainEvent::WorkerTaskVerificationRequired { task_id } => {
                if !self.tasks.contains_key(task_id) {
                    return Err(invalid(format!(
                        "verification policy references unknown worker task: {task_id}"
                    )));
                }
                if !self.verification_required.insert(task_id.clone()) {
                    return Err(invalid(format!(
                        "worker task verification policy recorded more than once: {task_id}"
                    )));
                }
            }
            DomainEvent::WorkerResultRecorded { result } => {
                let task = self.tasks.get(&result.task_id).ok_or_else(|| {
                    invalid(format!(
                        "worker result references unknown worker task: {}",
                        result.task_id
                    ))
                })?;
                if !self.executions.contains(&result.execution_id) {
                    return Err(invalid(format!(
                        "worker result references unknown execution: {}",
                        result.execution_id
                    )));
                }
                if !self.started.contains_key(&result.task_id) {
                    return Err(invalid(format!(
                        "worker result recorded before task start: {}",
                        result.task_id
                    )));
                }
                validate_result_schema(&task.expected_result_schema, &result.output)
                    .map_err(|error| invalid(error.to_string()))?;
                if self
                    .results
                    .insert(result.task_id.clone(), result.clone())
                    .is_some()
                {
                    return Err(invalid(format!(
                        "worker result recorded more than once: {}",
                        result.task_id
                    )));
                }
            }
            DomainEvent::WorkerVerificationRecorded { task_id, result } => {
                result
                    .validate()
                    .map_err(|error| invalid(error.to_string()))?;
                let worker_result = self.results.get(task_id).ok_or_else(|| {
                    invalid(format!(
                        "worker verification recorded before result: {task_id}"
                    ))
                })?;
                if !self.executions.contains(result.verifier_execution_id()) {
                    return Err(invalid(format!(
                        "worker verification references unknown execution: {}",
                        result.verifier_execution_id()
                    )));
                }
                if result.verifier_execution_id() == &worker_result.execution_id {
                    return Err(invalid(format!(
                        "worker execution cannot verify its own result: {task_id}"
                    )));
                }
                self.verifications
                    .entry(task_id.clone())
                    .or_default()
                    .push(result.clone());
            }
            DomainEvent::WorkerFailureAnalysisRecorded { task_id, analysis } => {
                analysis
                    .validate()
                    .map_err(|error| invalid(error.to_string()))?;
                if !self.tasks.contains_key(task_id) {
                    return Err(invalid(format!(
                        "worker failure analysis references unknown task: {task_id}"
                    )));
                }
                if !self.executions.contains(&analysis.analyzer_execution_id) {
                    return Err(invalid(format!(
                        "worker failure analysis references unknown execution: {}",
                        analysis.analyzer_execution_id
                    )));
                }
                self.failure_analyses
                    .entry(task_id.clone())
                    .or_default()
                    .push(analysis.clone());
            }
            DomainEvent::WorkerTaskCompleted {
                task_id,
                execution_id,
                result_refs,
            } => {
                let result = self.results.get(task_id).ok_or_else(|| {
                    invalid(format!("worker task completed without result: {task_id}"))
                })?;
                if &result.execution_id != execution_id {
                    return Err(invalid(format!(
                        "worker task completion/result execution mismatch: {task_id}"
                    )));
                }
                if &result.exact_refs() != result_refs {
                    return Err(invalid(format!(
                        "worker task completion/result references mismatch: {task_id}"
                    )));
                }
                if self.verification_required.contains(task_id)
                    && !self
                        .verifications
                        .get(task_id)
                        .and_then(|results| results.last())
                        .is_some_and(WorkerVerificationResult::passed)
                {
                    return Err(invalid(format!(
                        "worker task completed without required passing verification: {task_id}"
                    )));
                }
            }
            _ => {}
        }
        Ok(())
    }
}

fn invalid(message: impl Into<String>) -> JournalError {
    JournalError::InvalidEvent(message.into())
}

fn validate_result_schema(schema: &Value, output: &Value) -> Result<(), WorkerResultError> {
    let validator = jsonschema::validator_for(schema)
        .map_err(|error| WorkerResultError::InvalidResultSchema(error.to_string()))?;
    validator
        .validate(output)
        .map_err(|error| WorkerResultError::ResultSchemaMismatch(error.to_string()))
}

pub(crate) fn validate_journal_worker_results(
    journal: &RuntimeJournal,
) -> Result<(), JournalError> {
    let mut projection = WorkerResultProjection::default();
    for entry in &journal.entries {
        projection.apply(&entry.event)?;
    }
    Ok(())
}

impl ConductorRuntime {
    fn worker_result_projection(&self) -> Result<WorkerResultProjection, ConductorError> {
        let mut projection = WorkerResultProjection::default();
        for entry in &self.journal.entries {
            projection.apply(&entry.event)?;
        }
        Ok(projection)
    }

    pub fn require_worker_task_verification(
        &mut self,
        id: &WorkerTaskId,
    ) -> Result<(), ConductorError> {
        let task = self.worker_task(id)?;
        if matches!(
            task.state,
            WorkerTaskState::Completed { .. } | WorkerTaskState::Failed { .. }
        ) {
            return Err(WorkerTaskError::InvalidState(id.clone()).into());
        }
        let projection = self.worker_result_projection()?;
        if projection.results.contains_key(id) || projection.verification_required.contains(id) {
            return Err(WorkerTaskError::InvalidResult(
                "verification policy must be fixed before recording a result".to_owned(),
            )
            .into());
        }
        self.record_domain_event(DomainEvent::WorkerTaskVerificationRequired {
            task_id: id.clone(),
        })?;
        Ok(())
    }

    pub fn record_worker_result(
        &mut self,
        id: &WorkerTaskId,
        result: WorkerResultEnvelope,
    ) -> Result<WorkerResultEnvelope, ConductorError> {
        let task = self.worker_task(id)?;
        let WorkerTaskState::Running {
            execution_id: initial_execution,
        } = task.state
        else {
            return Err(WorkerTaskError::InvalidState(id.clone()).into());
        };
        let execution_id = self.current_worker_task_execution(&initial_execution);
        if result.task_id != *id || result.execution_id != execution_id {
            return Err(WorkerTaskError::InvalidResult(
                "worker result must bind to the exact task and current retry execution".to_owned(),
            )
            .into());
        }
        if self
            .executions
            .get(&execution_id)
            .ok_or_else(|| ConductorError::UnknownExecution(execution_id.clone()))?
            .summary
            .state
            != ExecutionState::Completed
        {
            return Err(WorkerTaskError::InvalidState(id.clone()).into());
        }
        validate_result_schema(&task.expected_result_schema, &result.output)
            .map_err(|error| WorkerTaskError::InvalidResult(error.to_string()))?;
        for reference in result
            .evidence_refs
            .iter()
            .chain(result.artifact_refs.iter())
        {
            self.resolve_exact_reference(reference)?;
        }
        if self.worker_result_projection()?.results.contains_key(id) {
            return Err(WorkerTaskError::InvalidResult(
                "worker result is already recorded".to_owned(),
            )
            .into());
        }
        self.record_domain_event(DomainEvent::WorkerResultRecorded {
            result: result.clone(),
        })?;
        Ok(result)
    }

    pub fn worker_result(
        &self,
        id: &WorkerTaskId,
    ) -> Result<Option<WorkerResultEnvelope>, ConductorError> {
        Ok(self.worker_result_projection()?.results.get(id).cloned())
    }

    pub fn record_worker_verification(
        &mut self,
        id: &WorkerTaskId,
        result: WorkerVerificationResult,
    ) -> Result<WorkerVerificationResult, ConductorError> {
        result
            .validate()
            .map_err(|error| WorkerTaskError::InvalidResult(error.to_string()))?;
        let projection = self.worker_result_projection()?;
        let worker_result = projection.results.get(id).ok_or_else(|| {
            WorkerTaskError::InvalidResult("worker result has not been recorded".to_owned())
        })?;
        if !projection.verification_required.contains(id) {
            return Err(WorkerTaskError::InvalidResult(
                "worker task does not require verification".to_owned(),
            )
            .into());
        }
        if result.verifier_execution_id() == &worker_result.execution_id {
            return Err(WorkerTaskError::InvalidResult(
                "worker execution cannot verify its own result".to_owned(),
            )
            .into());
        }
        self.executions
            .get(result.verifier_execution_id())
            .ok_or_else(|| {
                ConductorError::UnknownExecution(result.verifier_execution_id().clone())
            })?;
        for reference in result.evidence_refs() {
            self.resolve_exact_reference(reference)?;
        }
        self.record_domain_event(DomainEvent::WorkerVerificationRecorded {
            task_id: id.clone(),
            result: result.clone(),
        })?;
        Ok(result)
    }

    pub fn worker_verification(
        &self,
        id: &WorkerTaskId,
    ) -> Result<Option<WorkerVerificationResult>, ConductorError> {
        Ok(self
            .worker_result_projection()?
            .verifications
            .get(id)
            .and_then(|results| results.last())
            .cloned())
    }

    pub fn record_worker_failure_analysis(
        &mut self,
        id: &WorkerTaskId,
        analysis: WorkerFailureAnalysis,
    ) -> Result<WorkerFailureAnalysis, ConductorError> {
        analysis
            .validate()
            .map_err(|error| WorkerTaskError::InvalidResult(error.to_string()))?;
        let task = self.worker_task(id)?;
        let WorkerTaskState::Failed { execution_id, .. } = task.state else {
            return Err(WorkerTaskError::InvalidState(id.clone()).into());
        };
        if analysis.analyzer_execution_id == execution_id {
            return Err(WorkerTaskError::InvalidResult(
                "failed worker execution cannot analyze itself".to_owned(),
            )
            .into());
        }
        self.executions
            .get(&analysis.analyzer_execution_id)
            .ok_or_else(|| {
                ConductorError::UnknownExecution(analysis.analyzer_execution_id.clone())
            })?;
        for reference in &analysis.evidence_refs {
            self.resolve_exact_reference(reference)?;
        }
        self.record_domain_event(DomainEvent::WorkerFailureAnalysisRecorded {
            task_id: id.clone(),
            analysis: analysis.clone(),
        })?;
        Ok(analysis)
    }

    pub fn worker_failure_analyses(
        &self,
        id: &WorkerTaskId,
    ) -> Result<Vec<WorkerFailureAnalysis>, ConductorError> {
        Ok(self
            .worker_result_projection()?
            .failure_analyses
            .get(id)
            .cloned()
            .unwrap_or_default())
    }

    pub(crate) fn worker_completion_result_refs(
        &self,
        id: &WorkerTaskId,
        execution_id: &ExecutionId,
    ) -> Result<Vec<ExactReference>, ConductorError> {
        let projection = self.worker_result_projection()?;
        let result = projection.results.get(id).ok_or_else(|| {
            WorkerTaskError::InvalidResult("worker task has no structured result".to_owned())
        })?;
        if &result.execution_id != execution_id {
            return Err(WorkerTaskError::InvalidResult(
                "worker result belongs to a different execution".to_owned(),
            )
            .into());
        }
        if projection.verification_required.contains(id) {
            match projection
                .verifications
                .get(id)
                .and_then(|results| results.last())
            {
                Some(result) if result.passed() => {}
                Some(_) => {
                    return Err(WorkerTaskError::InvalidResult(
                        "latest required verification failed".to_owned(),
                    )
                    .into())
                }
                None => {
                    return Err(WorkerTaskError::InvalidResult(
                        "worker task requires independent verification".to_owned(),
                    )
                    .into())
                }
            }
        }
        Ok(result.exact_refs())
    }

    pub fn worker_result_for_parent(
        &self,
        id: &WorkerTaskId,
    ) -> Result<WorkerParentResultProjection, ConductorError> {
        let task = self.worker_task(id)?;
        let WorkerTaskState::Completed { execution_id, .. } = task.state else {
            return Err(WorkerTaskError::InvalidState(id.clone()).into());
        };
        let projection = self.worker_result_projection()?;
        let result = projection.results.get(id).ok_or_else(|| {
            WorkerTaskError::InvalidResult("worker task has no structured result".to_owned())
        })?;
        Ok(WorkerParentResultProjection {
            task_id: id.clone(),
            execution_id,
            output: result.output.clone(),
            evidence_refs: result.evidence_refs.clone(),
            artifact_refs: result.artifact_refs.clone(),
            verification: projection
                .verifications
                .get(id)
                .and_then(|results| results.last())
                .cloned(),
        })
    }
}
