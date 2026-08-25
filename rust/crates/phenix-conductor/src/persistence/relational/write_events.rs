fn event_type(event: &DomainEvent) -> &'static str {
    match event {
        DomainEvent::ConfigurationRevisionActivated { .. } => "configuration_revision_activated",
        DomainEvent::SessionCreated { .. } => "session_created",
        DomainEvent::SessionConfigRebased { .. } => "session_config_rebased",
        DomainEvent::SessionRenamed { .. } => "session_renamed",
        DomainEvent::SessionTargetChanged { .. } => "session_target_changed",
        DomainEvent::SessionClosed { .. } => "session_closed",
        DomainEvent::ExecutionCreated { .. } => "execution_created",
        DomainEvent::WorkerProfileBound { .. } => "worker_profile_bound",
        DomainEvent::RootSubmissionAccepted { .. } => "root_submission_accepted",
        DomainEvent::ExecutionStateChanged { .. } => "execution_state_changed",
        DomainEvent::AttemptGroupCreated { .. } => "attempt_group_created",
        DomainEvent::AttemptFailureRecorded { .. } => "attempt_failure_recorded",
        DomainEvent::AttemptRetryStarted { .. } => "attempt_retry_started",
        DomainEvent::OrchestrationFailureInterfaceStarted { .. } => {
            "orchestration_failure_interface_started"
        }
        DomainEvent::OrchestrationDecisionMade { .. } => "orchestration_decision_made",
        DomainEvent::OrchestrationNodeStarted { .. } => "orchestration_node_started",
        DomainEvent::OrchestrationNodeInputBound { .. } => "orchestration_node_input_bound",
        DomainEvent::OrchestrationSynthesisStarted { .. } => "orchestration_synthesis_started",
        DomainEvent::ExecutionOutputRecorded { .. } => "execution_output_recorded",
        DomainEvent::DiagnosticWritePatchCaptured { .. } => "diagnostic_write_patch_captured",
        DomainEvent::LanguageObservationRecorded { .. } => "language_observation_recorded",
        DomainEvent::ContextResourceRevisionRegistered { .. } => {
            "context_resource_revision_registered"
        }
        DomainEvent::ContextInjectionRecorded { .. } => "context_injection_recorded",
        DomainEvent::ContextCheckpointRecorded { .. } => "context_checkpoint_recorded",
        DomainEvent::ObjectiveSemanticsActivated => "objective_semantics_activated",
        DomainEvent::ObjectiveCreated { .. } => "objective_created",
        DomainEvent::ObjectiveDraftRevised { .. } => "objective_draft_revised",
        DomainEvent::ObjectiveEvidenceRecorded { .. } => "objective_evidence_recorded",
        DomainEvent::ObjectiveStateChanged { .. } => "objective_state_changed",
        DomainEvent::ExecutionObjectivesAssigned { .. } => "execution_objectives_assigned",
        DomainEvent::PlanCreated { .. } => "plan_created",
        DomainEvent::PlanDraftRevised { .. } => "plan_draft_revised",
        DomainEvent::PlanStateChanged { .. } => "plan_state_changed",
        DomainEvent::PlanStepStateChanged { .. } => "plan_step_state_changed",
        DomainEvent::ExecutionPlanAssigned { .. } => "execution_plan_assigned",
        DomainEvent::InvocationResolved { .. } => "invocation_resolved",
        DomainEvent::WorkspaceCheckpointCaptured { .. } => "workspace_checkpoint_captured",
        DomainEvent::WorkspaceFileObserved { .. } => "workspace_file_observed",
        DomainEvent::FrontendEvent { .. } => "frontend_event",
    }
}
