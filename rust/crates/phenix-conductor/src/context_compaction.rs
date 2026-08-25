use crate::{
    ConductorError, ConductorRuntime, ContextBudgetPolicy, ContextProjectionInspection,
    DomainEvent, JournalEntry,
};
use phenix_core::{ExactReference, ExecutionId, ModelTarget};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContextCompactionConfiguration {
    pub compactor_target: ModelTarget,
    pub budget_policy: ContextBudgetPolicy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContextHistoryRange {
    pub start_sequence: u64,
    pub end_sequence: u64,
}

impl ContextHistoryRange {
    #[must_use]
    pub fn contains(self, sequence: u64) -> bool {
        self.start_sequence <= sequence && sequence <= self.end_sequence
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContextCompactionOutput {
    pub summary: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContextCheckpointGeneration {
    pub model: ModelTarget,
    pub previous_checkpoint_sequence: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContextCheckpoint {
    pub execution_id: ExecutionId,
    pub summary: String,
    pub covered_history: Vec<ContextHistoryRange>,
    pub retained_refs: Vec<ExactReference>,
    pub generation: ContextCheckpointGeneration,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContextCompactionRequest {
    pub execution_id: ExecutionId,
    pub raw_history: Vec<JournalEntry>,
    pub previous_summary: Option<String>,
    pub covered_history: Vec<ContextHistoryRange>,
    pub retained_refs: Vec<ExactReference>,
}

impl ConductorRuntime {
    pub fn context_compaction_configuration_for_execution(
        &self,
        execution_id: &ExecutionId,
    ) -> Result<Option<ContextCompactionConfiguration>, ConductorError> {
        Ok(self
            .configuration_for_execution(execution_id)?
            .context_compaction
            .clone())
    }

    #[must_use]
    pub fn latest_context_checkpoint(
        &self,
        execution_id: &ExecutionId,
    ) -> Option<(u64, &ContextCheckpoint)> {
        self.journal
            .entries
            .iter()
            .rev()
            .find_map(|entry| match &entry.event {
                DomainEvent::ContextCheckpointRecorded { checkpoint }
                    if checkpoint.execution_id == *execution_id =>
                {
                    Some((entry.sequence, checkpoint))
                }
                _ => None,
            })
    }

    pub fn prepare_context_compaction(
        &self,
        execution_id: &ExecutionId,
    ) -> Result<ContextCompactionRequest, ConductorError> {
        if !self.executions.contains_key(execution_id) {
            return Err(ConductorError::UnknownExecution(execution_id.clone()));
        }
        let previous = self.latest_context_checkpoint(execution_id);
        let previous_ranges = previous
            .map(|(_, checkpoint)| checkpoint.covered_history.clone())
            .unwrap_or_default();
        let mut raw_history = Vec::new();
        let mut sequences = previous_ranges
            .iter()
            .flat_map(|range| range.start_sequence..=range.end_sequence)
            .collect::<Vec<_>>();
        for entry in &self.journal.entries {
            if event_references_execution(&entry.event, execution_id)
                && !previous_ranges
                    .iter()
                    .any(|range| range.contains(entry.sequence))
                && !matches!(entry.event, DomainEvent::ContextCheckpointRecorded { .. })
            {
                sequences.push(entry.sequence);
                raw_history.push(entry.clone());
            }
        }
        sequences.sort_unstable();
        sequences.dedup();
        let projection = self.project_execution_context(execution_id)?;
        let retained_refs = retained_projection_refs(&projection.injections);
        Ok(ContextCompactionRequest {
            execution_id: execution_id.clone(),
            raw_history,
            previous_summary: previous.map(|(_, checkpoint)| checkpoint.summary.clone()),
            covered_history: contiguous_ranges(&sequences),
            retained_refs,
        })
    }

    pub fn record_context_checkpoint(
        &mut self,
        request: &ContextCompactionRequest,
        output: ContextCompactionOutput,
    ) -> Result<ContextCheckpoint, ConductorError> {
        if output.summary.trim().is_empty() {
            return Err(ConductorError::InvalidExecutionData {
                execution_id: request.execution_id.clone(),
                message: "context compactor returned an empty summary".to_owned(),
            });
        }
        let configuration = self
            .context_compaction_configuration_for_execution(&request.execution_id)?
            .ok_or_else(|| ConductorError::InvalidExecutionData {
                execution_id: request.execution_id.clone(),
                message: "context compaction is not configured".to_owned(),
            })?;
        for reference in &request.retained_refs {
            self.resolve_exact_reference(reference)?;
        }
        let checkpoint = ContextCheckpoint {
            execution_id: request.execution_id.clone(),
            summary: output.summary,
            covered_history: request.covered_history.clone(),
            retained_refs: request.retained_refs.clone(),
            generation: ContextCheckpointGeneration {
                model: configuration.compactor_target,
                previous_checkpoint_sequence: self
                    .latest_context_checkpoint(&request.execution_id)
                    .map(|(sequence, _)| sequence),
            },
        };
        self.record_domain_event(DomainEvent::ContextCheckpointRecorded {
            checkpoint: checkpoint.clone(),
        })?;
        Ok(checkpoint)
    }
}

fn retained_projection_refs(injections: &[ContextProjectionInspection]) -> Vec<ExactReference> {
    let mut refs = Vec::new();
    for injection in injections {
        if !refs.contains(&injection.source_ref) {
            refs.push(injection.source_ref.clone());
        }
    }
    refs
}

fn contiguous_ranges(sequences: &[u64]) -> Vec<ContextHistoryRange> {
    let Some((&first, rest)) = sequences.split_first() else {
        return Vec::new();
    };
    let mut ranges = Vec::new();
    let mut start = first;
    let mut end = first;
    for &sequence in rest {
        if sequence == end.saturating_add(1) {
            end = sequence;
        } else {
            ranges.push(ContextHistoryRange {
                start_sequence: start,
                end_sequence: end,
            });
            start = sequence;
            end = sequence;
        }
    }
    ranges.push(ContextHistoryRange {
        start_sequence: start,
        end_sequence: end,
    });
    ranges
}

fn event_references_execution(event: &DomainEvent, execution_id: &ExecutionId) -> bool {
    match event {
        DomainEvent::ExecutionCreated { execution, .. } => execution.id == *execution_id,
        DomainEvent::WorkerProfileBound {
            execution_id: id, ..
        }
        | DomainEvent::RootSubmissionAccepted {
            execution_id: id, ..
        }
        | DomainEvent::ExecutionStateChanged {
            execution_id: id, ..
        }
        | DomainEvent::ExecutionOutputRecorded {
            execution_id: id, ..
        }
        | DomainEvent::InvocationResolved {
            execution_id: id, ..
        }
        | DomainEvent::WorkspaceCheckpointCaptured {
            execution_id: id, ..
        }
        | DomainEvent::WorkspaceFileObserved {
            execution_id: id, ..
        } => id == execution_id,
        DomainEvent::DiagnosticWritePatchCaptured { patch } => patch.execution_id == *execution_id,
        DomainEvent::LanguageObservationRecorded { observation } => {
            observation.execution == *execution_id
        }
        DomainEvent::ContextInjectionRecorded { injection } => {
            injection.execution_id == *execution_id
        }
        DomainEvent::ExecutionObjectivesAssigned { assignment } => {
            assignment.execution_id == *execution_id
        }
        DomainEvent::ExecutionPlanAssigned { assignment } => {
            assignment.execution_id == *execution_id
        }
        DomainEvent::OrchestrationFailureInterfaceStarted {
            parent_execution,
            failed_child,
            interface_execution,
        } => {
            parent_execution == execution_id
                || failed_child == execution_id
                || interface_execution == execution_id
        }
        DomainEvent::OrchestrationNodeStarted {
            execution_id: id,
            child_execution_id,
            ..
        } => id == execution_id || child_execution_id == execution_id,
        DomainEvent::OrchestrationNodeInputBound {
            execution_id: id, ..
        }
        | DomainEvent::OrchestrationSynthesisStarted {
            execution_id: id, ..
        } => id == execution_id,
        DomainEvent::FrontendEvent { event } => event.execution_id == *execution_id,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_model_target(model: &str) -> ModelTarget {
        ModelTarget {
            backend: phenix_core::BackendId::parse("mock").unwrap(),
            provider: phenix_core::ProviderId::parse("mock").unwrap(),
            model: phenix_core::ModelId::parse(model).unwrap(),
            inference: phenix_core::InferenceOptions::default(),
        }
    }

    #[test]
    fn repeated_compaction_retains_raw_history_provenance() {
        let mut runtime = ConductorRuntime::new();
        let target = test_model_target("history");
        let session = runtime
            .create_session(
                None,
                None,
                phenix_core::ExecutionTarget::Fixed(target.clone()),
            )
            .unwrap();
        let execution = runtime.submit(&session.id, "compact me").unwrap();
        let first = runtime.prepare_context_compaction(&execution.id).unwrap();
        assert!(!first.raw_history.is_empty());
        runtime
            .record_domain_event(DomainEvent::ContextCheckpointRecorded {
                checkpoint: ContextCheckpoint {
                    execution_id: execution.id.clone(),
                    summary: "first summary".to_owned(),
                    covered_history: first.covered_history.clone(),
                    retained_refs: first.retained_refs.clone(),
                    generation: ContextCheckpointGeneration {
                        model: target,
                        previous_checkpoint_sequence: None,
                    },
                },
            })
            .unwrap();
        runtime
            .set_state(&execution.id, phenix_core::ExecutionState::Running)
            .unwrap();

        let second = runtime.prepare_context_compaction(&execution.id).unwrap();
        assert_eq!(second.previous_summary.as_deref(), Some("first summary"));
        for range in &first.covered_history {
            for sequence in range.start_sequence..=range.end_sequence {
                assert!(second
                    .covered_history
                    .iter()
                    .any(|candidate| candidate.contains(sequence)));
            }
        }
        assert!(second.raw_history.iter().all(|entry| !first
            .covered_history
            .iter()
            .any(|range| range.contains(entry.sequence))));
        assert!(second
            .raw_history
            .iter()
            .any(|entry| matches!(entry.event, DomainEvent::ExecutionStateChanged { .. })));
    }

    #[test]
    fn checkpointing_never_creates_child_execution_from_token_pressure() {
        let mut runtime = ConductorRuntime::new();
        let target = test_model_target("no-child");
        let mut configuration = runtime.current_compiled_configuration().unwrap();
        configuration.configure_context_compaction(ContextCompactionConfiguration {
            compactor_target: target.clone(),
            budget_policy: ContextBudgetPolicy {
                output_reserve_tokens: 256,
                safety_margin_tokens: 128,
            },
        });
        runtime.reload_configuration(configuration).unwrap();
        let session = runtime
            .create_session(None, None, phenix_core::ExecutionTarget::Fixed(target))
            .unwrap();
        let execution = runtime
            .submit(&session.id, "compact without delegating")
            .unwrap();
        let before = runtime.snapshot().executions.len();
        let request = runtime.prepare_context_compaction(&execution.id).unwrap();
        runtime
            .record_context_checkpoint(
                &request,
                ContextCompactionOutput {
                    summary: "bounded summary".to_owned(),
                },
            )
            .unwrap();
        assert_eq!(runtime.snapshot().executions.len(), before);
    }

    #[test]
    fn contiguous_history_ranges_preserve_gaps() {
        assert_eq!(
            contiguous_ranges(&[1, 2, 4, 7, 8, 9]),
            vec![
                ContextHistoryRange {
                    start_sequence: 1,
                    end_sequence: 2,
                },
                ContextHistoryRange {
                    start_sequence: 4,
                    end_sequence: 4,
                },
                ContextHistoryRange {
                    start_sequence: 7,
                    end_sequence: 9,
                },
            ]
        );
    }
}
