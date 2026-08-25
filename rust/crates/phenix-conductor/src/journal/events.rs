use crate::{ConfigRevisionFingerprint, ExecutionPayload, WorkerProfileId};
use phenix_core::{
    AttemptGroup, AttemptGroupId, ConfigRevisionId, ContextInjection, ContextResourceRevision,
    DiagnosticWritePatch, ExactReference, ExecutionAuthority, ExecutionEvent, ExecutionId,
    ExecutionObjectiveAssignment, ExecutionPlanAssignment, ExecutionState, ExecutionSummary,
    ExecutionTarget, FailureAttemptSummary, FileObservation, FileVersion, LanguageObservation,
    ModelTarget, ObjectiveCriterionEvidence, ObjectiveId, ObjectiveRecord, ObjectiveTransition,
    OrchestrationFailureDecisionRecord, OrchestrationNodeId, PlanRecord, PlanStepTransition,
    PlanTransition, SessionId, SessionSummary, WorkspaceId,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::path::PathBuf;

pub const JOURNAL_FORMAT_VERSION: u64 = 4;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum JournalExecutionPayload {
    Invocation {
        input: String,
        #[serde(default)]
        authority: ExecutionAuthority,
    },
    Orchestration {
        input: serde_json::Value,
        #[serde(default)]
        authority: ExecutionAuthority,
    },
}

impl JournalExecutionPayload {
    #[must_use]
    pub(crate) fn authority(&self) -> &ExecutionAuthority {
        match self {
            Self::Invocation { authority, .. } | Self::Orchestration { authority, .. } => authority,
        }
    }

    pub(crate) fn set_authority(&mut self, authority: ExecutionAuthority) {
        match self {
            Self::Invocation {
                authority: current, ..
            }
            | Self::Orchestration {
                authority: current, ..
            } => *current = authority,
        }
    }
}

impl From<&ExecutionPayload> for JournalExecutionPayload {
    fn from(value: &ExecutionPayload) -> Self {
        match value {
            ExecutionPayload::Invocation { input } => Self::Invocation {
                input: input.clone(),
                authority: ExecutionAuthority::read_only(),
            },
            ExecutionPayload::Orchestration { input } => Self::Orchestration {
                input: input.clone(),
                authority: ExecutionAuthority::read_only(),
            },
        }
    }
}

impl From<JournalExecutionPayload> for ExecutionPayload {
    fn from(value: JournalExecutionPayload) -> Self {
        match value {
            JournalExecutionPayload::Invocation { input, .. } => Self::Invocation { input },
            JournalExecutionPayload::Orchestration { input, .. } => Self::Orchestration { input },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResolvedRoute {
    pub requested_target: ExecutionTarget,
    pub model: ModelTarget,
    pub config_revision: ConfigRevisionId,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DomainEvent {
    ConfigurationRevisionActivated {
        revision: ConfigRevisionId,
        fingerprint: ConfigRevisionFingerprint,
    },
    SessionCreated {
        session: SessionSummary,
    },
    SessionConfigRebased {
        session_id: SessionId,
        config_revision: ConfigRevisionId,
    },
    SessionRenamed {
        session_id: SessionId,
        name: String,
    },
    SessionTargetChanged {
        session_id: SessionId,
        target: ExecutionTarget,
    },
    SessionClosed {
        session_id: SessionId,
    },
    ExecutionCreated {
        execution: ExecutionSummary,
        payload: JournalExecutionPayload,
    },
    WorkerProfileBound {
        execution_id: ExecutionId,
        profile_id: WorkerProfileId,
    },
    RootSubmissionAccepted {
        session_id: SessionId,
        execution_id: ExecutionId,
        ingress_order: u64,
    },
    ExecutionStateChanged {
        execution_id: ExecutionId,
        state: ExecutionState,
    },
    AttemptGroupCreated {
        group: AttemptGroup,
    },
    AttemptFailureRecorded {
        group_id: AttemptGroupId,
        failure: FailureAttemptSummary,
    },
    AttemptRetryStarted {
        group_id: AttemptGroupId,
        execution_id: ExecutionId,
    },
    OrchestrationFailureInterfaceStarted {
        parent_execution: ExecutionId,
        failed_child: ExecutionId,
        interface_execution: ExecutionId,
    },
    OrchestrationDecisionMade {
        decision: OrchestrationFailureDecisionRecord,
    },
    OrchestrationNodeStarted {
        execution_id: ExecutionId,
        node_id: OrchestrationNodeId,
        child_execution_id: ExecutionId,
    },
    OrchestrationNodeInputBound {
        execution_id: ExecutionId,
        node_id: OrchestrationNodeId,
        input: serde_json::Value,
    },
    OrchestrationSynthesisStarted {
        execution_id: ExecutionId,
        interface_execution_id: ExecutionId,
    },
    ExecutionOutputRecorded {
        execution_id: ExecutionId,
        output: serde_json::Value,
    },
    DiagnosticWritePatchCaptured {
        patch: DiagnosticWritePatch,
    },
    LanguageObservationRecorded {
        observation: LanguageObservation,
    },
    ContextResourceRevisionRegistered {
        resource: ContextResourceRevision,
    },
    ContextInjectionRecorded {
        injection: ContextInjection,
    },
    ObjectiveSemanticsActivated,
    ObjectiveCreated {
        objective: ObjectiveRecord,
    },
    ObjectiveDraftRevised {
        objective: ObjectiveRecord,
    },
    ObjectiveEvidenceRecorded {
        objective_id: ObjectiveId,
        evidence: ObjectiveCriterionEvidence,
    },
    ObjectiveStateChanged {
        transition: ObjectiveTransition,
    },
    ExecutionObjectivesAssigned {
        assignment: ExecutionObjectiveAssignment,
    },
    PlanCreated {
        plan: PlanRecord,
    },
    PlanDraftRevised {
        plan: PlanRecord,
        expected_revision: u64,
    },
    PlanStateChanged {
        transition: PlanTransition,
    },
    PlanStepStateChanged {
        transition: PlanStepTransition,
    },
    ExecutionPlanAssigned {
        assignment: ExecutionPlanAssignment,
    },
    InvocationResolved {
        execution_id: ExecutionId,
        route: ResolvedRoute,
    },
    WorkspaceCheckpointCaptured {
        execution_id: ExecutionId,
        workspace_id: WorkspaceId,
        #[serde(default)]
        files: BTreeMap<PathBuf, FileVersion>,
    },
    WorkspaceFileObserved {
        execution_id: ExecutionId,
        observation: FileObservation,
    },
    FrontendEvent {
        event: ExecutionEvent,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JournalEntry {
    pub sequence: u64,
    pub event: DomainEvent,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RuntimeJournal {
    pub format_version: u64,
    pub config_revision: ConfigRevisionId,
    pub config_fingerprint: ConfigRevisionFingerprint,
    pub entries: Vec<JournalEntry>,
}

impl RuntimeJournal {
    #[must_use]
    pub fn new(
        config_revision: ConfigRevisionId,
        config_fingerprint: ConfigRevisionFingerprint,
    ) -> Self {
        Self {
            format_version: JOURNAL_FORMAT_VERSION,
            config_revision,
            config_fingerprint,
            entries: Vec::new(),
        }
    }

    pub fn validate_structure(&self) -> Result<(), JournalError> {
        if self.format_version != JOURNAL_FORMAT_VERSION {
            return Err(JournalError::InvalidFormat(format!(
                "unsupported journal format version: {}",
                self.format_version
            )));
        }
        for (index, entry) in self.entries.iter().enumerate() {
            let expected = u64::try_from(index)
                .map_err(|_| JournalError::InvalidFormat("journal is too large".to_owned()))?
                + 1;
            if entry.sequence != expected {
                return Err(JournalError::InvalidSequence {
                    expected,
                    actual: entry.sequence,
                });
            }
        }
        self.validate_observation_ids()?;
        self.validate_context_resource_revisions()?;
        crate::objectives::validate_journal_objectives(self)?;
        crate::plans::validate_journal_plans(self)?;
        Ok(())
    }

    fn validate_observation_ids(&self) -> Result<(), JournalError> {
        let mut file_ids = BTreeMap::new();
        let mut language_ids = BTreeMap::new();
        for entry in &self.entries {
            match &entry.event {
                DomainEvent::WorkspaceFileObserved { observation, .. } => {
                    if file_ids
                        .insert(observation.id.clone(), entry.sequence)
                        .is_some()
                    {
                        return Err(JournalError::InvalidEvent(format!(
                            "file observation id recorded more than once: {}",
                            observation.id
                        )));
                    }
                }
                DomainEvent::LanguageObservationRecorded { observation }
                    if language_ids
                        .insert(observation.id.clone(), entry.sequence)
                        .is_some() =>
                {
                    return Err(JournalError::InvalidEvent(format!(
                        "language observation id recorded more than once: {}",
                        observation.id
                    )));
                }
                DomainEvent::LanguageObservationRecorded { .. } => {}
                _ => {}
            }
        }
        Ok(())
    }

    fn validate_context_resource_revisions(&self) -> Result<(), JournalError> {
        let mut revisions = BTreeMap::new();
        for entry in &self.entries {
            let DomainEvent::ContextResourceRevisionRegistered { resource } = &entry.event else {
                continue;
            };
            let key = (
                resource.descriptor.id.clone(),
                resource.descriptor.revision.clone(),
            );
            if let ExactReference::Context {
                resource_id,
                revision,
            } = &resource.source_ref
            {
                if resource_id != &resource.descriptor.id
                    || revision != &resource.descriptor.revision
                {
                    return Err(JournalError::InvalidEvent(format!(
                        "context resource {}@{} carries mismatched exact source reference {}",
                        resource.descriptor.id, resource.descriptor.revision, resource.source_ref
                    )));
                }
            }
            if revisions.insert(key, resource).is_some() {
                return Err(JournalError::InvalidEvent(format!(
                    "context resource revision registered more than once: {}@{}",
                    resource.descriptor.id, resource.descriptor.revision
                )));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JournalError {
    InvalidSequence { expected: u64, actual: u64 },
    InvalidFormat(String),
    InvalidEvent(String),
}

impl Display for JournalError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSequence { expected, actual } => {
                write!(
                    f,
                    "journal sequence mismatch: expected {expected}, found {actual}"
                )
            }
            Self::InvalidFormat(message) => write!(f, "invalid journal format: {message}"),
            Self::InvalidEvent(message) => write!(f, "invalid journal event: {message}"),
        }
    }
}

impl Error for JournalError {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn orchestration_payload_serializes_typed_input() {
        let payload: JournalExecutionPayload = serde_json::from_value(json!({
            "kind": "orchestration",
            "input": {"goal": "implement"}
        }))
        .unwrap();
        assert!(matches!(
            payload,
            JournalExecutionPayload::Orchestration {
                ref input,
                ..
            } if input == &json!({"goal": "implement"})
        ));
        assert_eq!(payload.authority(), &ExecutionAuthority::read_only());
        assert_eq!(
            serde_json::to_value(&payload).unwrap()["kind"],
            "orchestration"
        );
    }
}
