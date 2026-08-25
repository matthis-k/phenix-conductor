fn load_event(
    connection: &Connection,
    sequence: i64,
    event_type: &str,
) -> Result<DomainEvent, PersistenceError> {
    match event_type {
        "configuration_revision_activated" => {
            let (revision, fingerprint) = connection.query_row(
                "SELECT revision_id, fingerprint FROM configuration_revisions
                 WHERE activated_sequence = ?1",
                params![sequence],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )?;
            Ok(DomainEvent::ConfigurationRevisionActivated {
                revision: parse_id(
                    revision,
                    "configuration revision",
                    phenix_core::ConfigRevisionId::parse,
                )?,
                fingerprint: ConfigRevisionFingerprint(fingerprint),
            })
        }
        "session_created" => load_session_created(connection, sequence),
        "session_config_rebased" => {
            let (session, revision) = connection.query_row(
                "SELECT session_id, config_revision_id FROM session_config_rebases
                 WHERE sequence = ?1",
                params![sequence],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )?;
            Ok(DomainEvent::SessionConfigRebased {
                session_id: parse_id(session, "session", SessionId::parse)?,
                config_revision: parse_id(
                    revision,
                    "configuration revision",
                    phenix_core::ConfigRevisionId::parse,
                )?,
            })
        }
        "session_renamed" => {
            let (session, name) = connection.query_row(
                "SELECT session_id, name FROM session_renames WHERE sequence = ?1",
                params![sequence],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )?;
            Ok(DomainEvent::SessionRenamed {
                session_id: parse_id(session, "session", SessionId::parse)?,
                name,
            })
        }
        "session_target_changed" => {
            let (session, target) = connection.query_row(
                "SELECT session_id, target_id FROM session_target_changes WHERE sequence = ?1",
                params![sequence],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
            )?;
            Ok(DomainEvent::SessionTargetChanged {
                session_id: parse_id(session, "session", SessionId::parse)?,
                target: load_target(connection, target)?,
            })
        }
        "session_closed" => {
            let session = connection.query_row(
                "SELECT session_id FROM session_closures WHERE sequence = ?1",
                params![sequence],
                |row| row.get::<_, String>(0),
            )?;
            Ok(DomainEvent::SessionClosed {
                session_id: parse_id(session, "session", SessionId::parse)?,
            })
        }
        "execution_created" => load_execution_created(connection, sequence),
        "worker_profile_bound" => {
            let (execution, profile) = connection.query_row(
                "SELECT execution_id, profile_id FROM execution_worker_profiles WHERE bound_sequence = ?1",
                params![sequence],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )?;
            Ok(DomainEvent::WorkerProfileBound {
                execution_id: parse_id(execution, "execution", ExecutionId::parse)?,
                profile_id: WorkerProfileId::parse(profile)
                    .map_err(|_| invalid("database contains an invalid worker profile"))?,
            })
        }
        "root_submission_accepted" => {
            let (session, execution, ingress) = connection.query_row(
                "SELECT session_id, execution_id, ingress_order
                 FROM accepted_root_submissions WHERE accepted_sequence = ?1",
                params![sequence],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                },
            )?;
            Ok(DomainEvent::RootSubmissionAccepted {
                session_id: parse_id(session, "session", SessionId::parse)?,
                execution_id: parse_id(execution, "execution", ExecutionId::parse)?,
                ingress_order: runtime_u64(ingress, "root ingress order")?,
            })
        }
        "execution_state_changed" => {
            let (execution, state) = connection.query_row(
                "SELECT execution_id, state FROM execution_state_changes WHERE sequence = ?1",
                params![sequence],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )?;
            Ok(DomainEvent::ExecutionStateChanged {
                execution_id: parse_id(execution, "execution", ExecutionId::parse)?,
                state: parse_execution_state(&state)?,
            })
        }
        "attempt_group_created" => load_attempt_group_created(connection, sequence),
        "attempt_failure_recorded" => {
            let (group, failure) = load_attempt_failure(connection, sequence)?;
            Ok(DomainEvent::AttemptFailureRecorded {
                group_id: group,
                failure,
            })
        }
        "attempt_retry_started" => {
            let (group, execution) = connection.query_row(
                "SELECT attempt_group_id, execution_id FROM attempt_executions
                 WHERE started_sequence = ?1",
                params![sequence],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )?;
            Ok(DomainEvent::AttemptRetryStarted {
                group_id: parse_id(group, "attempt group", AttemptGroupId::parse)?,
                execution_id: parse_id(execution, "execution", ExecutionId::parse)?,
            })
        }
        "orchestration_failure_interface_started" => {
            let (parent, failed, interface) = connection.query_row(
                "SELECT parent_execution_id, failed_child_execution_id, interface_execution_id
                 FROM orchestration_failure_interfaces WHERE started_sequence = ?1",
                params![sequence],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )?;
            Ok(DomainEvent::OrchestrationFailureInterfaceStarted {
                parent_execution: parse_id(parent, "execution", ExecutionId::parse)?,
                failed_child: parse_id(failed, "execution", ExecutionId::parse)?,
                interface_execution: parse_id(interface, "execution", ExecutionId::parse)?,
            })
        }
        "orchestration_decision_made" => Ok(DomainEvent::OrchestrationDecisionMade {
            decision: load_decision(connection, sequence)?,
        }),
        "orchestration_node_started" => {
            let (execution, node, child) = connection.query_row(
                "SELECT orchestration_execution_id, node_id, child_execution_id
                 FROM orchestration_node_bindings WHERE bound_sequence = ?1",
                params![sequence],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )?;
            Ok(DomainEvent::OrchestrationNodeStarted {
                execution_id: parse_id(execution, "execution", ExecutionId::parse)?,
                node_id: parse_id(node, "orchestration node", OrchestrationNodeId::parse)?,
                child_execution_id: parse_id(child, "execution", ExecutionId::parse)?,
            })
        }
        "orchestration_node_input_bound" => {
            let (execution, node, value) = connection.query_row(
                "SELECT orchestration_execution_id, node_id, input_value_id
                 FROM orchestration_node_inputs WHERE bound_sequence = ?1",
                params![sequence],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                },
            )?;
            Ok(DomainEvent::OrchestrationNodeInputBound {
                execution_id: parse_id(execution, "execution", ExecutionId::parse)?,
                node_id: parse_id(node, "orchestration node", OrchestrationNodeId::parse)?,
                input: load_value(connection, value)?,
            })
        }
        "orchestration_synthesis_started" => {
            let (execution, interface) = connection.query_row(
                "SELECT orchestration_execution_id, interface_execution_id
                 FROM orchestration_synthesis WHERE started_sequence = ?1",
                params![sequence],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )?;
            Ok(DomainEvent::OrchestrationSynthesisStarted {
                execution_id: parse_id(execution, "execution", ExecutionId::parse)?,
                interface_execution_id: parse_id(interface, "execution", ExecutionId::parse)?,
            })
        }
        "execution_output_recorded" => {
            let (execution, value) = connection.query_row(
                "SELECT execution_id, output_value_id FROM execution_outputs
                 WHERE recorded_sequence = ?1",
                params![sequence],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
            )?;
            Ok(DomainEvent::ExecutionOutputRecorded {
                execution_id: parse_id(execution, "execution", ExecutionId::parse)?,
                output: load_value(connection, value)?,
            })
        }
        "diagnostic_write_patch_captured" => {
            let (execution, path, patch) = connection.query_row(
                "SELECT execution_id, path, patch FROM diagnostic_write_patches
                 WHERE captured_sequence = ?1",
                params![sequence],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )?;
            Ok(DomainEvent::DiagnosticWritePatchCaptured {
                patch: DiagnosticWritePatch {
                    execution_id: parse_id(execution, "execution", ExecutionId::parse)?,
                    path: PathBuf::from(path),
                    patch,
                },
            })
        }
        "language_observation_recorded" => {
            let row = connection.query_row(
                "SELECT observation_id, execution_id, workspace_id, service_kind, provider_id,
                    provider_epoch, operation_value_id, result_value_id
             FROM language_observations WHERE recorded_sequence = ?1",
                params![sequence],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, i64>(6)?,
                        row.get::<_, i64>(7)?,
                    ))
                },
            )?;
            let operation = serde_json::from_value(load_value(connection, row.6)?)
                .map_err(|error| invalid(format!("invalid stored language operation: {error}")))?;
            let result = serde_json::from_value(load_value(connection, row.7)?)
                .map_err(|error| invalid(format!("invalid stored language result: {error}")))?;
            Ok(DomainEvent::LanguageObservationRecorded {
                observation: phenix_core::LanguageObservation {
                    id: parse_id(row.0, "language observation", LanguageObservationId::parse)?,
                    execution: parse_id(row.1, "execution", ExecutionId::parse)?,
                    workspace: parse_id(row.2, "workspace", WorkspaceId::parse)?,
                    service: parse_id(
                        row.3,
                        "language service",
                        phenix_core::LanguageServiceKind::parse,
                    )?,
                    provider: parse_id(
                        row.4,
                        "language provider",
                        phenix_core::LanguageProviderId::parse,
                    )?,
                    provider_epoch: runtime_u64(row.5, "language provider epoch")?,
                    operation,
                    result,
                },
            })
        }
        "context_resource_revision_registered" => {
            let row = connection.query_row(
                "SELECT resource_id, revision, resource_kind, title, description, scope_kind,
                        scope_id, scope_path, estimated_cost, tier, source_kind, source_id,
                        source_event_sequence, source_revision, content_identity, content
                 FROM context_resource_revisions WHERE recorded_sequence = ?1",
                params![sequence],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, Option<String>>(7)?,
                        row.get::<_, i64>(8)?,
                        row.get::<_, String>(9)?,
                        row.get::<_, String>(10)?,
                        row.get::<_, Option<String>>(11)?,
                        row.get::<_, Option<i64>>(12)?,
                        row.get::<_, String>(13)?,
                        row.get::<_, String>(14)?,
                        row.get::<_, Option<String>>(15)?,
                    ))
                },
            )?;
            let revision = parse_id(row.1, "context resource revision", ContextRevision::parse)?;
            let source_revision =
                parse_id(row.13, "context source revision", ContextRevision::parse)?;
            Ok(DomainEvent::ContextResourceRevisionRegistered {
                resource: ContextResourceRevision {
                    descriptor: ContextDescriptor {
                        id: parse_id(row.0, "context resource", ContextResourceId::parse)?,
                        kind: parse_context_resource_kind(&row.2)?,
                        title: row.3,
                        description: row.4,
                        scope: parse_context_scope(&row.5, row.6, row.7)?,
                        revision,
                        estimated_cost: runtime_u64(row.8, "context estimated cost")?,
                    },
                    tier: parse_context_tier(&row.9)?,
                    source_ref: parse_context_source(&row.10, row.11, row.12, &source_revision)?,
                    content_identity: parse_id(
                        row.14,
                        "context content identity",
                        ContextRevision::parse,
                    )?,
                    content: row.15,
                },
            })
        }
        "context_injection_recorded" => {
            let row = connection.query_row(
                "SELECT execution_id, source_kind, source_id, source_event_sequence,
                        source_revision, requester, reason, lifetime, content_identity
                 FROM context_injections WHERE recorded_sequence = ?1",
                params![sequence],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<i64>>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, String>(8)?,
                    ))
                },
            )?;
            let source_revision =
                parse_id(row.4, "context source revision", ContextRevision::parse)?;
            Ok(DomainEvent::ContextInjectionRecorded {
                injection: ContextInjection {
                    execution_id: parse_id(row.0, "execution", ExecutionId::parse)?,
                    source_ref: parse_context_source(&row.1, row.2, row.3, &source_revision)?,
                    source_revision,
                    requested_by: parse_context_requester(&row.5)?,
                    reason: row.6,
                    lifetime: parse_context_lifetime(&row.7)?,
                    content_identity: parse_id(
                        row.8,
                        "context content identity",
                        ContextRevision::parse,
                    )?,
                },
            })
        }
        "context_checkpoint_recorded" => {
            let (execution, summary, target, previous) = connection.query_row(
                "SELECT execution_id, summary, compactor_target_id, previous_checkpoint_sequence
                 FROM context_checkpoints WHERE recorded_sequence = ?1",
                params![sequence],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, Option<i64>>(3)?,
                    ))
                },
            )?;
            let model = match load_target(connection, target)? {
                ExecutionTarget::Fixed(model) => model,
                ExecutionTarget::Routed(_) => {
                    return Err(invalid("context checkpoint compactor target must be fixed"));
                }
            };
            let mut range_statement = connection.prepare(
                "SELECT start_sequence, end_sequence FROM context_checkpoint_ranges
                 WHERE checkpoint_sequence = ?1 ORDER BY range_index",
            )?;
            let range_rows = range_statement.query_map(params![sequence], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
            })?;
            let mut covered_history = Vec::new();
            for row in range_rows {
                let (start, end) = row?;
                covered_history.push(ContextHistoryRange {
                    start_sequence: runtime_u64(start, "context checkpoint range start")?,
                    end_sequence: runtime_u64(end, "context checkpoint range end")?,
                });
            }
            let mut ref_statement = connection.prepare(
                "SELECT source_kind, source_id, source_event_sequence, source_revision
                 FROM context_checkpoint_retained_refs
                 WHERE checkpoint_sequence = ?1 ORDER BY ref_index",
            )?;
            let ref_rows = ref_statement.query_map(params![sequence], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })?;
            let mut retained_refs = Vec::new();
            for row in ref_rows {
                let (kind, id, event_sequence, revision) = row?;
                retained_refs.push(parse_checkpoint_reference(
                    &kind,
                    id,
                    event_sequence,
                    revision,
                )?);
            }
            Ok(DomainEvent::ContextCheckpointRecorded {
                checkpoint: ContextCheckpoint {
                    execution_id: parse_id(execution, "execution", ExecutionId::parse)?,
                    summary,
                    covered_history,
                    retained_refs,
                    generation: ContextCheckpointGeneration {
                        model,
                        previous_checkpoint_sequence: previous
                            .map(|value| runtime_u64(value, "previous context checkpoint sequence"))
                            .transpose()?,
                    },
                },
            })
        }
        "objective_semantics_activated" => Ok(DomainEvent::ObjectiveSemanticsActivated),
        "objective_created" => Ok(DomainEvent::ObjectiveCreated {
            objective: load_objective_creation(connection, sequence)?,
        }),
        "objective_draft_revised" => Ok(DomainEvent::ObjectiveDraftRevised {
            objective: load_objective_draft_revision(connection, sequence)?,
        }),
        "objective_evidence_recorded" => {
            let (objective, criterion, evidence_ref) = connection.query_row(
                "SELECT objective_id, criterion_id, evidence_ref FROM objective_evidence
                 WHERE sequence = ?1",
                params![sequence],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )?;
            Ok(DomainEvent::ObjectiveEvidenceRecorded {
                objective_id: parse_id(objective, "objective", ObjectiveId::parse)?,
                evidence: ObjectiveCriterionEvidence {
                    criterion_id: parse_id(
                        criterion,
                        "objective criterion",
                        phenix_core::ObjectiveCriterionId::parse,
                    )?,
                    evidence_ref,
                },
            })
        }
        "objective_state_changed" => Ok(DomainEvent::ObjectiveStateChanged {
            transition: load_objective_transition(connection, sequence)?,
        }),
        "execution_objectives_assigned" => Ok(DomainEvent::ExecutionObjectivesAssigned {
            assignment: load_execution_objective_assignment(connection, sequence)?,
        }),
        "plan_created"
        | "plan_draft_revised"
        | "plan_state_changed"
        | "plan_step_state_changed"
        | "execution_plan_assigned" => {
            plan_relational::load_event(connection, sequence, event_type)
        }
        "invocation_resolved" => load_invocation_resolved(connection, sequence),
        "workspace_checkpoint_captured" => load_checkpoint(connection, sequence),
        "workspace_file_observed" => load_observation(connection, sequence),
        "frontend_event" => Ok(DomainEvent::FrontendEvent {
            event: load_frontend_event(connection, sequence)?,
        }),
        other => Err(invalid(format!("unknown relational event type: {other}"))),
    }
}
