fn context_resource_kind_token(kind: &ContextResourceKind) -> &'static str {
    match kind {
        ContextResourceKind::Skill => "skill",
        ContextResourceKind::ProjectDocument => "project_document",
        ContextResourceKind::Objective => "objective",
        ContextResourceKind::Plan => "plan",
    }
}

fn context_tier_token(tier: &ContextTier) -> &'static str {
    match tier {
        ContextTier::MandatoryContent => "mandatory_content",
        ContextTier::MandatoryMetadata => "mandatory_metadata",
        ContextTier::DiscoverableContent => "discoverable_content",
    }
}

fn context_scope_columns(scope: &ContextScope) -> (&'static str, Option<String>, Option<String>) {
    match scope {
        ContextScope::Workspace { workspace_id } => {
            ("workspace", Some(workspace_id.to_string()), None)
        }
        ContextScope::Execution { execution_id } => {
            ("execution", Some(execution_id.to_string()), None)
        }
        ContextScope::Objective { objective_id } => {
            ("objective", Some(objective_id.to_string()), None)
        }
        ContextScope::Path { path } => ("path", None, Some(path.to_string_lossy().into_owned())),
        ContextScope::Configuration { revision } => {
            ("configuration", Some(revision.to_string()), None)
        }
    }
}

fn context_scope_id(
    id: Option<String>,
    path: Option<String>,
    kind: &str,
) -> Result<String, PersistenceError> {
    if path.is_some() {
        return Err(invalid(format!(
            "context {kind} scope must not contain a path"
        )));
    }
    required_column(id, "context scope id")
}

fn context_source_columns(
    source: &ExactReference,
) -> Result<(&'static str, Option<String>, Option<i64>), PersistenceError> {
    match source {
        ExactReference::Objective(id) => Ok(("objective", Some(id.to_string()), None)),
        ExactReference::Plan(id) => Ok(("plan", Some(id.to_string()), None)),
        ExactReference::Execution(id) => Ok(("execution", Some(id.to_string()), None)),
        ExactReference::Event(sequence) => Ok((
            "event",
            None,
            Some(sql_u64(*sequence, "context source event sequence")?),
        )),
        ExactReference::FileObservation(id) => Ok(("file_observation", Some(id.to_string()), None)),
        ExactReference::LanguageObservation(id) => {
            Ok(("language_observation", Some(id.to_string()), None))
        }
        ExactReference::Context { resource_id, .. } => {
            Ok(("context", Some(resource_id.to_string()), None))
        }
    }
}

fn context_source_id(
    id: Option<String>,
    event_sequence: Option<i64>,
    kind: &str,
) -> Result<String, PersistenceError> {
    if event_sequence.is_some() {
        return Err(invalid(format!(
            "context {kind} source must not contain an event sequence"
        )));
    }
    required_column(id, "context source id")
}

fn context_requester_token(requester: &ContextInjectionRequester) -> &'static str {
    match requester {
        ContextInjectionRequester::Agent => "agent",
        ContextInjectionRequester::User => "user",
        ContextInjectionRequester::Orchestration => "orchestration",
        ContextInjectionRequester::ContextPolicy => "context_policy",
        ContextInjectionRequester::Hook => "hook",
        ContextInjectionRequester::Frontend => "frontend",
    }
}

fn context_lifetime_token(lifetime: &ContextInjectionLifetime) -> &'static str {
    match lifetime {
        ContextInjectionLifetime::SingleRequest => "single_request",
        ContextInjectionLifetime::Execution => "execution",
        ContextInjectionLifetime::Objective => "objective",
    }
}
