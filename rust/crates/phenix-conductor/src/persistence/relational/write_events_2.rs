fn insert_event(
    transaction: &Transaction<'_>,
    sequence: u64,
    event: &DomainEvent,
) -> Result<(), PersistenceError> {
    let sequence = sql_u64(sequence, "journal sequence")?;
    match event {
        DomainEvent::ConfigurationRevisionActivated {
            revision,
            fingerprint,
        } => {
            transaction.execute(
                "INSERT INTO configuration_revisions(revision_id, fingerprint, activated_sequence)
                 VALUES (?1, ?2, ?3)",
                params![revision.to_string(), fingerprint.to_string(), sequence],
            )?;
        }
        DomainEvent::SessionCreated { session } => {
            let target = insert_target(transaction, &session.default_target)?;
            transaction.execute(
                "INSERT INTO sessions(
                     session_id, parent_session_id, workspace_id, config_revision_id, name,
                     default_target_id, state, created_sequence
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    session.id.to_string(),
                    session.parent_session.as_ref().map(ToString::to_string),
                    session.workspace_id.to_string(),
                    session.config_revision.to_string(),
                    session.name.as_deref(),
                    target,
                    session_state_token(&session.state),
                    sequence,
                ],
            )?;
        }
        DomainEvent::SessionConfigRebased {
            session_id,
            config_revision,
        } => {
            transaction.execute(
                "INSERT INTO session_config_rebases(sequence, session_id, config_revision_id)
                 VALUES (?1, ?2, ?3)",
                params![
                    sequence,
                    session_id.to_string(),
                    config_revision.to_string()
                ],
            )?;
        }
        DomainEvent::SessionRenamed { session_id, name } => {
            transaction.execute(
                "INSERT INTO session_renames(sequence, session_id, name) VALUES (?1, ?2, ?3)",
                params![sequence, session_id.to_string(), name],
            )?;
        }
        DomainEvent::SessionTargetChanged { session_id, target } => {
            let target = insert_target(transaction, target)?;
            transaction.execute(
                "INSERT INTO session_target_changes(sequence, session_id, target_id)
                 VALUES (?1, ?2, ?3)",
                params![sequence, session_id.to_string(), target],
            )?;
        }
        DomainEvent::SessionClosed { session_id } => {
            transaction.execute(
                "INSERT INTO session_closures(sequence, session_id) VALUES (?1, ?2)",
                params![sequence, session_id.to_string()],
            )?;
        }
        DomainEvent::ExecutionCreated { execution, payload } => {
            insert_execution(transaction, sequence, execution, payload)?;
        }
        DomainEvent::WorkerProfileBound {
            execution_id,
            profile_id,
        } => {
            transaction.execute(
                "INSERT INTO execution_worker_profiles(execution_id, profile_id, bound_sequence)\n                 VALUES (?1, ?2, ?3)",
                params![execution_id.to_string(), profile_id.to_string(), sequence],
            )?;
        }
        DomainEvent::RootSubmissionAccepted {
            session_id,
            execution_id,
            ingress_order,
        } => {
            transaction.execute(
                "INSERT INTO accepted_root_submissions(
                     session_id, ingress_order, execution_id, accepted_sequence
                 ) VALUES (?1, ?2, ?3, ?4)",
                params![
                    session_id.to_string(),
                    sql_u64(*ingress_order, "root ingress order")?,
                    execution_id.to_string(),
                    sequence,
                ],
            )?;
        }
        DomainEvent::ExecutionStateChanged {
            execution_id,
            state,
        } => {
            transaction.execute(
                "INSERT INTO execution_state_changes(sequence, execution_id, state)
                 VALUES (?1, ?2, ?3)",
                params![
                    sequence,
                    execution_id.to_string(),
                    execution_state_token(state)
                ],
            )?;
        }
        DomainEvent::AttemptGroupCreated { group } => {
            transaction.execute(
                "INSERT INTO attempt_groups(
                     attempt_group_id, parent_execution_id, callable_id, invariant_goal,
                     created_sequence
                 ) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    group.id.to_string(),
                    group.parent_execution.to_string(),
                    group.callable.to_string(),
                    group.goal.as_str(),
                    sequence,
                ],
            )?;
            for (index, execution_id) in group.attempts.iter().enumerate() {
                transaction.execute(
                    "INSERT INTO attempt_executions(
                         attempt_group_id, attempt_number, execution_id, started_sequence
                     ) VALUES (?1, ?2, ?3, ?4)",
                    params![
                        group.id.to_string(),
                        sql_usize(index + 1, "attempt number")?,
                        execution_id.to_string(),
                        sequence,
                    ],
                )?;
            }
            for failure in &group.failures {
                insert_attempt_failure(transaction, sequence, &group.id, failure)?;
            }
        }
        DomainEvent::AttemptFailureRecorded { group_id, failure } => {
            insert_attempt_failure(transaction, sequence, group_id, failure)?;
        }
        DomainEvent::AttemptRetryStarted {
            group_id,
            execution_id,
        } => {
            let attempt = transaction.query_row(
                "SELECT COALESCE(MAX(attempt_number), 0) + 1
                 FROM attempt_executions WHERE attempt_group_id = ?1",
                params![group_id.to_string()],
                |row| row.get::<_, i64>(0),
            )?;
            transaction.execute(
                "INSERT INTO attempt_executions(
                     attempt_group_id, attempt_number, execution_id, started_sequence
                 ) VALUES (?1, ?2, ?3, ?4)",
                params![
                    group_id.to_string(),
                    attempt,
                    execution_id.to_string(),
                    sequence
                ],
            )?;
        }
        DomainEvent::OrchestrationFailureInterfaceStarted {
            parent_execution,
            failed_child,
            interface_execution,
        } => {
            transaction.execute(
                "INSERT INTO orchestration_failure_interfaces(
                     failed_child_execution_id, parent_execution_id, interface_execution_id,
                     started_sequence
                 ) VALUES (?1, ?2, ?3, ?4)",
                params![
                    failed_child.to_string(),
                    parent_execution.to_string(),
                    interface_execution.to_string(),
                    sequence,
                ],
            )?;
        }
        DomainEvent::OrchestrationDecisionMade { decision } => {
            let (kind, recovery) = decision_columns(&decision.decision);
            transaction.execute(
                "INSERT INTO parent_failure_decisions(
                     failed_child_execution_id, parent_execution_id, decider_execution_id,
                     decision_kind, recovery_execution_id, decided_sequence
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    decision.failed_child.to_string(),
                    decision.parent_execution.to_string(),
                    decision.decider_execution.as_ref().map(ToString::to_string),
                    kind,
                    recovery,
                    sequence,
                ],
            )?;
        }
        DomainEvent::OrchestrationNodeStarted {
            execution_id,
            node_id,
            child_execution_id,
        } => {
            transaction.execute(
                "INSERT INTO orchestration_node_bindings(
                     orchestration_execution_id, node_id, child_execution_id, bound_sequence
                 ) VALUES (?1, ?2, ?3, ?4)",
                params![
                    execution_id.to_string(),
                    node_id.to_string(),
                    child_execution_id.to_string(),
                    sequence,
                ],
            )?;
        }
        DomainEvent::OrchestrationNodeInputBound {
            execution_id,
            node_id,
            input,
        } => {
            let input = insert_value(transaction, input)?;
            transaction.execute(
                "INSERT INTO orchestration_node_inputs(
                     orchestration_execution_id, node_id, input_value_id, bound_sequence
                 ) VALUES (?1, ?2, ?3, ?4)",
                params![
                    execution_id.to_string(),
                    node_id.to_string(),
                    input,
                    sequence
                ],
            )?;
        }
        DomainEvent::OrchestrationSynthesisStarted {
            execution_id,
            interface_execution_id,
        } => {
            transaction.execute(
                "INSERT INTO orchestration_synthesis(
                     orchestration_execution_id, interface_execution_id, started_sequence
                 ) VALUES (?1, ?2, ?3)",
                params![
                    execution_id.to_string(),
                    interface_execution_id.to_string(),
                    sequence,
                ],
            )?;
        }
        DomainEvent::ExecutionOutputRecorded {
            execution_id,
            output,
        } => {
            let output = insert_value(transaction, output)?;
            transaction.execute(
                "INSERT INTO execution_outputs(
                     execution_id, output_value_id, recorded_sequence
                 ) VALUES (?1, ?2, ?3)",
                params![execution_id.to_string(), output, sequence],
            )?;
        }
        DomainEvent::DiagnosticWritePatchCaptured { patch } => {
            transaction.execute(
                "INSERT INTO diagnostic_write_patches(
                     execution_id, path, patch, captured_sequence
                 ) VALUES (?1, ?2, ?3, ?4)",
                params![
                    patch.execution_id.to_string(),
                    patch.path.to_string_lossy(),
                    patch.patch.as_str(),
                    sequence,
                ],
            )?;
        }
        DomainEvent::LanguageObservationRecorded { observation } => {
            let operation_value = serde_json::to_value(&observation.operation)
                .map_err(|error| invalid(format!("cannot encode language operation: {error}")))?;
            let result_value = serde_json::to_value(&observation.result)
                .map_err(|error| invalid(format!("cannot encode language result: {error}")))?;
            let operation = insert_value(transaction, &operation_value)?;
            let result = insert_value(transaction, &result_value)?;
            transaction.execute(
                "INSERT INTO language_observations(
                     recorded_sequence, observation_id, execution_id, workspace_id, service_kind,
                     provider_id, provider_epoch, operation_value_id, result_value_id
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    sequence,
                    observation.id.to_string(),
                    observation.execution.to_string(),
                    observation.workspace.to_string(),
                    observation.service.to_string(),
                    observation.provider.to_string(),
                    sql_u64(observation.provider_epoch, "language provider epoch")?,
                    operation,
                    result,
                ],
            )?;
        }
        DomainEvent::ContextResourceRevisionRegistered { resource } => {
            let (scope_kind, scope_id, scope_path) =
                context_scope_columns(&resource.descriptor.scope);
            let (source_kind, source_id, source_event_sequence) =
                context_source_columns(&resource.source_ref)?;
            let source_revision = match &resource.source_ref {
                ExactReference::Context { revision, .. } => revision,
                _ => &resource.descriptor.revision,
            };
            transaction.execute(
                "INSERT INTO context_resource_revisions(
                     recorded_sequence, resource_id, revision, resource_kind, title, description,
                     scope_kind, scope_id, scope_path, estimated_cost, tier, source_kind, source_id,
                     source_event_sequence, source_revision, content_identity, content
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
                params![
                    sequence,
                    resource.descriptor.id.to_string(),
                    resource.descriptor.revision.to_string(),
                    context_resource_kind_token(&resource.descriptor.kind),
                    resource.descriptor.title.as_str(),
                    resource.descriptor.description.as_str(),
                    scope_kind,
                    scope_id,
                    scope_path,
                    sql_u64(resource.descriptor.estimated_cost, "context estimated cost")?,
                    context_tier_token(&resource.tier),
                    source_kind,
                    source_id,
                    source_event_sequence,
                    source_revision.to_string(),
                    resource.content_identity.to_string(),
                    resource.content.as_deref(),
                ],
            )?;
        }
        DomainEvent::ContextInjectionRecorded { injection } => {
            if let ExactReference::Context { revision, .. } = &injection.source_ref {
                if revision != &injection.source_revision {
                    return Err(invalid(
                        "context source reference revision does not match injection source revision",
                    ));
                }
            }
            let (source_kind, source_id, source_event_sequence) =
                context_source_columns(&injection.source_ref)?;
            transaction.execute(
                "INSERT INTO context_injections(
                     recorded_sequence, execution_id, source_kind, source_id, source_event_sequence,
                     source_revision, requester, reason, lifetime, content_identity
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    sequence,
                    injection.execution_id.to_string(),
                    source_kind,
                    source_id,
                    source_event_sequence,
                    injection.source_revision.to_string(),
                    context_requester_token(&injection.requested_by),
                    injection.reason.as_str(),
                    context_lifetime_token(&injection.lifetime),
                    injection.content_identity.to_string(),
                ],
            )?;
        }
        DomainEvent::ContextCheckpointRecorded { checkpoint } => {
            let target = insert_target(
                transaction,
                &ExecutionTarget::Fixed(checkpoint.generation.model.clone()),
            )?;
            transaction.execute(
                "INSERT INTO context_checkpoints(
                     recorded_sequence, execution_id, summary, compactor_target_id,
                     previous_checkpoint_sequence
                 ) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    sequence,
                    checkpoint.execution_id.to_string(),
                    checkpoint.summary.as_str(),
                    target,
                    checkpoint
                        .generation
                        .previous_checkpoint_sequence
                        .map(|value| sql_u64(value, "previous context checkpoint sequence"))
                        .transpose()?,
                ],
            )?;
            for (index, range) in checkpoint.covered_history.iter().enumerate() {
                transaction.execute(
                    "INSERT INTO context_checkpoint_ranges(
                         checkpoint_sequence, range_index, start_sequence, end_sequence
                     ) VALUES (?1, ?2, ?3, ?4)",
                    params![
                        sequence,
                        sql_usize(index, "context checkpoint range index")?,
                        sql_u64(range.start_sequence, "context checkpoint range start")?,
                        sql_u64(range.end_sequence, "context checkpoint range end")?,
                    ],
                )?;
            }
            for (index, reference) in checkpoint.retained_refs.iter().enumerate() {
                let (source_kind, source_id, source_event_sequence) =
                    context_source_columns(reference)?;
                let source_revision = match reference {
                    ExactReference::Context { revision, .. } => Some(revision.to_string()),
                    _ => None,
                };
                transaction.execute(
                    "INSERT INTO context_checkpoint_retained_refs(
                         checkpoint_sequence, ref_index, source_kind, source_id,
                         source_event_sequence, source_revision
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        sequence,
                        sql_usize(index, "context checkpoint ref index")?,
                        source_kind,
                        source_id,
                        source_event_sequence,
                        source_revision,
                    ],
                )?;
            }
        }
        DomainEvent::ObjectiveSemanticsActivated => {}
        DomainEvent::ObjectiveCreated { objective } => {
            let (origin, parent) = objective_origin_columns(&objective.origin);
            transaction.execute(
                "INSERT INTO objective_creations(
                     created_sequence, objective_id, workspace_id, origin_kind, parent_objective_id,
                     statement, state, supersedes_objective_id
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    sequence,
                    objective.id.to_string(),
                    objective.workspace.to_string(),
                    origin,
                    parent,
                    objective.statement,
                    objective_state_token(&objective.state),
                    objective.supersedes.as_ref().map(ToString::to_string),
                ],
            )?;
            insert_objective_criteria(
                transaction,
                "objective_creation_criteria",
                "created_sequence",
                sequence,
                &objective.criteria,
            )?;
        }
        DomainEvent::ObjectiveDraftRevised { objective } => {
            transaction.execute(
                "INSERT INTO objective_draft_revisions(sequence, objective_id, statement)
                 VALUES (?1, ?2, ?3)",
                params![sequence, objective.id.to_string(), objective.statement],
            )?;
            insert_objective_criteria(
                transaction,
                "objective_draft_revision_criteria",
                "sequence",
                sequence,
                &objective.criteria,
            )?;
        }
        DomainEvent::ObjectiveEvidenceRecorded {
            objective_id,
            evidence,
        } => {
            transaction.execute(
                "INSERT INTO objective_evidence(sequence, objective_id, criterion_id, evidence_ref)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    sequence,
                    objective_id.to_string(),
                    evidence.criterion_id.to_string(),
                    evidence.evidence_ref,
                ],
            )?;
        }
        DomainEvent::ObjectiveStateChanged { transition } => {
            let (kind, execution, detail) = objective_cause_columns(&transition.cause);
            transaction.execute(
                "INSERT INTO objective_state_changes(
                     sequence, objective_id, from_state, to_state, cause_kind,
                     cause_execution_id, cause_detail
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    sequence,
                    transition.objective_id.to_string(),
                    objective_state_token(&transition.from),
                    objective_state_token(&transition.to),
                    kind,
                    execution,
                    detail,
                ],
            )?;
        }
        DomainEvent::ExecutionObjectivesAssigned { assignment } => {
            transaction.execute(
                "INSERT INTO execution_objective_assignments(
                     sequence, execution_id, primary_objective_id
                 ) VALUES (?1, ?2, ?3)",
                params![
                    sequence,
                    assignment.execution_id.to_string(),
                    assignment.primary.to_string(),
                ],
            )?;
            for objective in &assignment.supporting {
                transaction.execute(
                    "INSERT INTO execution_supporting_objectives(sequence, objective_id)
                     VALUES (?1, ?2)",
                    params![sequence, objective.to_string()],
                )?;
            }
        }
        event @ (DomainEvent::PlanCreated { .. }
        | DomainEvent::PlanDraftRevised { .. }
        | DomainEvent::PlanStateChanged { .. }
        | DomainEvent::PlanStepStateChanged { .. }
        | DomainEvent::ExecutionPlanAssigned { .. }) => {
            plan_relational::insert_event(transaction, sequence, event)?;
        }
        DomainEvent::InvocationResolved {
            execution_id,
            route,
        } => {
            let requested = insert_target(transaction, &route.requested_target)?;
            let model = insert_target(transaction, &ExecutionTarget::Fixed(route.model.clone()))?;
            transaction.execute(
                "INSERT INTO resolved_routing(
                     execution_id, requested_target_id, model_target_id, config_revision_id,
                     resolved_sequence
                 ) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    execution_id.to_string(),
                    requested,
                    model,
                    route.config_revision.to_string(),
                    sequence,
                ],
            )?;
        }
        DomainEvent::WorkspaceCheckpointCaptured {
            execution_id,
            workspace_id,
            files,
        } => {
            transaction.execute(
                "INSERT INTO workspace_checkpoints(
                     checkpoint_sequence, execution_id, workspace_id
                 ) VALUES (?1, ?2, ?3)",
                params![sequence, execution_id.to_string(), workspace_id.to_string()],
            )?;
            for (path, version) in files {
                let (state, hash, kind) = file_version_columns(version);
                transaction.execute(
                    "INSERT INTO workspace_checkpoint_files(
                         checkpoint_sequence, path, version_state, content_hash, file_kind
                     ) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![sequence, path.to_string_lossy(), state, hash, kind],
                )?;
            }
        }
        DomainEvent::WorkspaceFileObserved {
            execution_id,
            observation,
        } => {
            let (state, hash, kind) = file_version_columns(&observation.version);
            transaction.execute(
                "INSERT INTO workspace_observation_events(
                     observation_id, execution_id, path, version_state, content_hash, file_kind,
                     observed_sequence
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    observation.id.to_string(),
                    execution_id.to_string(),
                    observation.path.to_string_lossy(),
                    state,
                    hash,
                    kind,
                    sequence,
                ],
            )?;
        }
        DomainEvent::FrontendEvent { event } => {
            insert_frontend_event(transaction, sequence, event)?
        }
    }
    Ok(())
}
