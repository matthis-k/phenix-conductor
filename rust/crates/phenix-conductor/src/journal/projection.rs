mod attempts;
mod executions;
mod observations;
mod orchestration;
mod process_resources;
mod replay;
mod sessions;
mod state;

use super::{DomainEvent, JournalError};
pub(crate) use state::DurableProjection;

pub(crate) fn apply_domain_event(
    state: &mut DurableProjection<'_>,
    event: &DomainEvent,
) -> Result<(), JournalError> {
    match event {
        DomainEvent::ConfigurationRevisionActivated { .. }
        | DomainEvent::SessionCreated { .. }
        | DomainEvent::SessionConfigRebased { .. }
        | DomainEvent::SessionRenamed { .. }
        | DomainEvent::SessionTargetChanged { .. }
        | DomainEvent::SessionClosed { .. } => sessions::apply(state, event),

        DomainEvent::ExecutionCreated { .. }
        | DomainEvent::WorkerProfileBound { .. }
        | DomainEvent::RootSubmissionAccepted { .. }
        | DomainEvent::ExecutionStateChanged { .. }
        | DomainEvent::InvocationResolved { .. }
        | DomainEvent::FrontendEvent { .. } => executions::apply(state, event),

        DomainEvent::TerminalCreated { .. }
        | DomainEvent::JobCreated { .. }
        | DomainEvent::TerminalStateChanged { .. }
        | DomainEvent::JobStateChanged { .. }
        | DomainEvent::JobPromoted { .. }
        | DomainEvent::TerminalOutputRecorded { .. }
        | DomainEvent::JobOutputRecorded { .. } => process_resources::apply(state, event),

        DomainEvent::AttemptGroupCreated { .. }
        | DomainEvent::AttemptFailureRecorded { .. }
        | DomainEvent::AttemptRetryStarted { .. } => attempts::apply(state, event),

        DomainEvent::OrchestrationFailureInterfaceStarted { .. }
        | DomainEvent::OrchestrationDecisionMade { .. }
        | DomainEvent::OrchestrationNodeStarted { .. }
        | DomainEvent::OrchestrationNodeInputBound { .. }
        | DomainEvent::OrchestrationSynthesisStarted { .. }
        | DomainEvent::ExecutionOutputRecorded { .. } => orchestration::apply(state, event),

        DomainEvent::DiagnosticWritePatchCaptured { .. }
        | DomainEvent::LanguageObservationRecorded { .. }
        | DomainEvent::ContextInjectionRecorded { .. }
        | DomainEvent::ContextCheckpointRecorded { .. }
        | DomainEvent::WorkspaceCheckpointCaptured { .. }
        | DomainEvent::WorkspaceFileObserved { .. } => observations::apply(state, event),

        DomainEvent::WorkerTaskCreated { .. }
        | DomainEvent::WorkerTaskStarted { .. }
        | DomainEvent::WorkerTaskCompleted { .. }
        | DomainEvent::WorkerTaskFailed { .. }
        | DomainEvent::ContextResourceRevisionRegistered { .. }
        | DomainEvent::ObjectiveSemanticsActivated
        | DomainEvent::ObjectiveCreated { .. }
        | DomainEvent::ObjectiveDraftRevised { .. }
        | DomainEvent::ObjectiveEvidenceRecorded { .. }
        | DomainEvent::ObjectiveStateChanged { .. }
        | DomainEvent::ExecutionObjectivesAssigned { .. }
        | DomainEvent::PlanCreated { .. }
        | DomainEvent::PlanDraftRevised { .. }
        | DomainEvent::PlanStateChanged { .. }
        | DomainEvent::PlanStepStateChanged { .. }
        | DomainEvent::ExecutionPlanAssigned { .. }
        | DomainEvent::DecisionDraftCreated { .. }
        | DomainEvent::DecisionDraftRevised { .. }
        | DomainEvent::DecisionRecorded { .. }
        | DomainEvent::DecisionApplicabilityAssessed { .. } => Ok(()),
    }
}
