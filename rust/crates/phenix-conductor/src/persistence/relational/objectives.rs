fn objective_origin_columns(origin: &ObjectiveOrigin) -> (&'static str, Option<String>) {
    match origin {
        ObjectiveOrigin::Root => ("root", None),
        ObjectiveOrigin::Derived { parent } => ("derived", Some(parent.to_string())),
    }
}

fn objective_cause_columns(
    cause: &ObjectiveTransitionCause,
) -> (&'static str, Option<String>, Option<String>) {
    match cause {
        ObjectiveTransitionCause::UserIntent => ("user_intent", None, None),
        ObjectiveTransitionCause::AgentAction { execution_id } => {
            ("agent_action", Some(execution_id.to_string()), None)
        }
        ObjectiveTransitionCause::ExecutionOutcome { execution_id } => {
            ("execution_outcome", Some(execution_id.to_string()), None)
        }
        ObjectiveTransitionCause::EvidenceAssessment { evidence_ref } => {
            ("evidence_assessment", None, Some(evidence_ref.clone()))
        }
        ObjectiveTransitionCause::Policy { description } => {
            ("policy", None, Some(description.clone()))
        }
    }
}

fn objective_state_token(state: &ObjectiveState) -> &'static str {
    match state {
        ObjectiveState::Draft => "draft",
        ObjectiveState::Active => "active",
        ObjectiveState::Completed => "completed",
        ObjectiveState::Failed => "failed",
        ObjectiveState::Invalidated => "invalidated",
        ObjectiveState::Abandoned => "abandoned",
        ObjectiveState::Superseded => "superseded",
    }
}
