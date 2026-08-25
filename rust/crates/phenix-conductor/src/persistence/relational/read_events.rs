fn parse_context_resource_kind(value: &str) -> Result<ContextResourceKind, PersistenceError> {
    match value {
        "skill" => Ok(ContextResourceKind::Skill),
        "project_document" => Ok(ContextResourceKind::ProjectDocument),
        "objective" => Ok(ContextResourceKind::Objective),
        "plan" => Ok(ContextResourceKind::Plan),
        "artifact" => Ok(ContextResourceKind::Artifact),
        other => Err(invalid(format!("unknown context resource kind: {other}"))),
    }
}

fn parse_context_tier(value: &str) -> Result<ContextTier, PersistenceError> {
    match value {
        "mandatory_content" => Ok(ContextTier::MandatoryContent),
        "mandatory_metadata" => Ok(ContextTier::MandatoryMetadata),
        "discoverable_content" => Ok(ContextTier::DiscoverableContent),
        other => Err(invalid(format!("unknown context tier: {other}"))),
    }
}

fn parse_context_scope(
    kind: &str,
    id: Option<String>,
    path: Option<String>,
) -> Result<ContextScope, PersistenceError> {
    match kind {
        "workspace" => Ok(ContextScope::Workspace {
            workspace_id: parse_id(
                context_scope_id(id, path, kind)?,
                "context workspace scope",
                WorkspaceId::parse,
            )?,
        }),
        "execution" => Ok(ContextScope::Execution {
            execution_id: parse_id(
                context_scope_id(id, path, kind)?,
                "context execution scope",
                ExecutionId::parse,
            )?,
        }),
        "objective" => Ok(ContextScope::Objective {
            objective_id: parse_id(
                context_scope_id(id, path, kind)?,
                "context objective scope",
                ObjectiveId::parse,
            )?,
        }),
        "configuration" => Ok(ContextScope::Configuration {
            revision: parse_id(
                context_scope_id(id, path, kind)?,
                "context configuration scope",
                phenix_core::ConfigRevisionId::parse,
            )?,
        }),
        "path" => {
            if id.is_some() {
                return Err(invalid("context path scope must not contain a scope id"));
            }
            Ok(ContextScope::Path {
                path: PathBuf::from(
                    path.ok_or_else(|| invalid("context path scope is missing its path"))?,
                ),
            })
        }
        other => Err(invalid(format!("unknown context scope kind: {other}"))),
    }
}

fn parse_context_source(
    kind: &str,
    id: Option<String>,
    event_sequence: Option<i64>,
    source_revision: &ContextRevision,
) -> Result<ExactReference, PersistenceError> {
    match kind {
        "objective" => Ok(ExactReference::Objective(parse_id(
            context_source_id(id, event_sequence, kind)?,
            "context objective source",
            ObjectiveId::parse,
        )?)),
        "plan" => Ok(ExactReference::Plan(parse_id(
            context_source_id(id, event_sequence, kind)?,
            "context plan source",
            PlanId::parse,
        )?)),
        "execution" => Ok(ExactReference::Execution(parse_id(
            context_source_id(id, event_sequence, kind)?,
            "context execution source",
            ExecutionId::parse,
        )?)),
        "event" => {
            if id.is_some() {
                return Err(invalid("context event source must not contain a source id"));
            }
            Ok(ExactReference::Event(runtime_u64(
                event_sequence
                    .ok_or_else(|| invalid("context event source is missing its sequence"))?,
                "context source event sequence",
            )?))
        }
        "file_observation" => Ok(ExactReference::FileObservation(parse_id(
            context_source_id(id, event_sequence, kind)?,
            "context file observation source",
            FileObservationId::parse,
        )?)),
        "language_observation" => Ok(ExactReference::LanguageObservation(parse_id(
            context_source_id(id, event_sequence, kind)?,
            "context language observation source",
            LanguageObservationId::parse,
        )?)),
        "context" => Ok(ExactReference::Context {
            resource_id: parse_id(
                context_source_id(id, event_sequence, kind)?,
                "context resource source",
                ContextResourceId::parse,
            )?,
            revision: source_revision.clone(),
        }),
        other => Err(invalid(format!("unknown context source kind: {other}"))),
    }
}

fn parse_checkpoint_reference(
    kind: &str,
    id: Option<String>,
    event_sequence: Option<i64>,
    revision: Option<String>,
) -> Result<ExactReference, PersistenceError> {
    let fallback = ContextRevision::parse("checkpoint:none")
        .expect("static context revision is valid");
    let revision = revision
        .map(|value| parse_id(value, "checkpoint context revision", ContextRevision::parse))
        .transpose()?;
    if kind == "context" && revision.is_none() {
        return Err(invalid("checkpoint context reference is missing its revision"));
    }
    parse_context_source(kind, id, event_sequence, revision.as_ref().unwrap_or(&fallback))
}

fn parse_context_requester(value: &str) -> Result<ContextInjectionRequester, PersistenceError> {
    match value {
        "agent" => Ok(ContextInjectionRequester::Agent),
        "user" => Ok(ContextInjectionRequester::User),
        "orchestration" => Ok(ContextInjectionRequester::Orchestration),
        "context_policy" => Ok(ContextInjectionRequester::ContextPolicy),
        "hook" => Ok(ContextInjectionRequester::Hook),
        "frontend" => Ok(ContextInjectionRequester::Frontend),
        other => Err(invalid(format!(
            "unknown context injection requester: {other}"
        ))),
    }
}

fn parse_context_lifetime(value: &str) -> Result<ContextInjectionLifetime, PersistenceError> {
    match value {
        "single_request" => Ok(ContextInjectionLifetime::SingleRequest),
        "execution" => Ok(ContextInjectionLifetime::Execution),
        "objective" => Ok(ContextInjectionLifetime::Objective),
        other => Err(invalid(format!(
            "unknown context injection lifetime: {other}"
        ))),
    }
}

fn load_entries(connection: &Connection) -> Result<Vec<crate::JournalEntry>, PersistenceError> {
    let mut statement =
        connection.prepare("SELECT sequence, event_type FROM domain_events ORDER BY sequence")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut entries = Vec::new();
    for row in rows {
        let (sequence, event_type) = row?;
        entries.push(crate::JournalEntry {
            sequence: runtime_u64(sequence, "journal sequence")?,
            event: load_event(connection, sequence, &event_type)?,
        });
    }
    Ok(entries)
}
