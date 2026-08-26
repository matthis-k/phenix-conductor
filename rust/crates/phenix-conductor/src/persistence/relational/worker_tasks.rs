fn insert_worker_task_event(
    transaction: &Transaction<'_>,
    sequence: i64,
    event: &DomainEvent,
) -> Result<(), PersistenceError> {
    match event {
        DomainEvent::WorkerTaskCreated { task } => {
            let schema = insert_value(transaction, &task.expected_result_schema)?;
            let authority = insert_value(transaction, &serde_json::to_value(&task.delegated_authority).map_err(|_| invalid("worker task authority is not serializable"))?)?;
            transaction.execute(
                "INSERT INTO worker_tasks(task_id, parent_execution_id, primary_objective_id, plan_id, plan_step_id, description, profile_id, expected_result_schema_value_id, delegated_authority_value_id, created_sequence) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    task.id.to_string(), task.parent_execution.to_string(), task.primary_objective.to_string(),
                    task.plan_step.as_ref().map(|step| step.plan_id.to_string()),
                    task.plan_step.as_ref().map(|step| step.step_id.to_string()),
                    task.description, task.profile_id.to_string(), schema, authority, sequence,
                ],
            )?;
            for (ordinal, objective) in task.supporting_objectives.iter().enumerate() {
                transaction.execute("INSERT INTO worker_task_supporting_objectives(task_id, ordinal, objective_id) VALUES (?1, ?2, ?3)", params![task.id.to_string(), sql_usize(ordinal, "worker task supporting objective ordinal")?, objective.to_string()])?;
            }
            for (ordinal, dependency) in task.depends_on.iter().enumerate() {
                transaction.execute("INSERT INTO worker_task_dependencies(task_id, ordinal, dependency_task_id) VALUES (?1, ?2, ?3)", params![task.id.to_string(), sql_usize(ordinal, "worker task dependency ordinal")?, dependency.to_string()])?;
            }
            for (ordinal, reference) in task.input_refs.iter().enumerate() {
                let value = insert_value(transaction, &serde_json::to_value(reference).map_err(|_| invalid("worker task exact input reference is not serializable"))?)?;
                transaction.execute("INSERT INTO worker_task_input_refs(task_id, ordinal, reference_value_id) VALUES (?1, ?2, ?3)", params![task.id.to_string(), sql_usize(ordinal, "worker task input reference ordinal")?, value])?;
            }
        }
        DomainEvent::WorkerTaskStarted { task_id, execution_id } => {
            transaction.execute("INSERT INTO worker_task_state_events(sequence, task_id, event_kind, execution_id, cause) VALUES (?1, ?2, 'started', ?3, NULL)", params![sequence, task_id.to_string(), execution_id.to_string()])?;
        }
        DomainEvent::WorkerTaskVerificationRequired { task_id } => {
            transaction.execute("INSERT INTO worker_task_verification_requirements(sequence, task_id) VALUES (?1, ?2)", params![sequence, task_id.to_string()])?;
        }
        DomainEvent::WorkerResultRecorded { result } => {
            let output = insert_value(transaction, &result.output)?;
            transaction.execute("INSERT INTO worker_results(sequence, task_id, execution_id, output_value_id) VALUES (?1, ?2, ?3, ?4)", params![sequence, result.task_id.to_string(), result.execution_id.to_string(), output])?;
            insert_worker_sequence_refs(transaction, "worker_result_evidence_refs", sequence, &result.evidence_refs)?;
            insert_worker_sequence_refs(transaction, "worker_result_artifact_refs", sequence, &result.artifact_refs)?;
        }
        DomainEvent::WorkerVerificationRecorded { task_id, result } => {
            let (status, verifier, reason, evidence) = match result {
                crate::WorkerVerificationResult::Passed { verifier_execution_id, evidence_refs } => ("passed", verifier_execution_id, None, evidence_refs),
                crate::WorkerVerificationResult::Failed { verifier_execution_id, reason, evidence_refs } => ("failed", verifier_execution_id, Some(reason.as_str()), evidence_refs),
            };
            transaction.execute("INSERT INTO worker_verifications(sequence, task_id, verifier_execution_id, status, reason) VALUES (?1, ?2, ?3, ?4, ?5)", params![sequence, task_id.to_string(), verifier.to_string(), status, reason])?;
            insert_worker_sequence_refs(transaction, "worker_verification_evidence_refs", sequence, evidence)?;
        }
        DomainEvent::WorkerFailureAnalysisRecorded { task_id, analysis } => {
            transaction.execute("INSERT INTO worker_failure_analyses(sequence, task_id, analyzer_execution_id, diagnosis, proposed_action) VALUES (?1, ?2, ?3, ?4, ?5)", params![sequence, task_id.to_string(), analysis.analyzer_execution_id.to_string(), analysis.diagnosis, worker_failure_action_token(&analysis.proposed_action)])?;
            insert_worker_sequence_refs(transaction, "worker_failure_analysis_evidence_refs", sequence, &analysis.evidence_refs)?;
        }
        DomainEvent::WorkerTaskCompleted { task_id, execution_id, result_refs } => {
            transaction.execute("INSERT INTO worker_task_state_events(sequence, task_id, event_kind, execution_id, cause) VALUES (?1, ?2, 'completed', ?3, NULL)", params![sequence, task_id.to_string(), execution_id.to_string()])?;
            for (ordinal, reference) in result_refs.iter().enumerate() {
                let value = insert_value(transaction, &serde_json::to_value(reference).map_err(|_| invalid("worker task result reference is not serializable"))?)?;
                transaction.execute("INSERT INTO worker_task_result_refs(sequence, ordinal, reference_value_id) VALUES (?1, ?2, ?3)", params![sequence, sql_usize(ordinal, "worker task result reference ordinal")?, value])?;
            }
        }
        DomainEvent::WorkerTaskFailed { task_id, execution_id, cause } => {
            transaction.execute("INSERT INTO worker_task_state_events(sequence, task_id, event_kind, execution_id, cause) VALUES (?1, ?2, 'failed', ?3, ?4)", params![sequence, task_id.to_string(), execution_id.to_string(), cause])?;
        }
        _ => unreachable!("worker task persistence received unrelated event"),
    }
    Ok(())
}

fn insert_worker_sequence_refs(
    transaction: &Transaction<'_>,
    table: &str,
    sequence: i64,
    references: &[ExactReference],
) -> Result<(), PersistenceError> {
    let sql = format!("INSERT INTO {table}(sequence, ordinal, reference_value_id) VALUES (?1, ?2, ?3)");
    for (ordinal, reference) in references.iter().enumerate() {
        let value = insert_value(transaction, &serde_json::to_value(reference).map_err(|_| invalid("worker exact reference is not serializable"))?)?;
        transaction.execute(&sql, params![sequence, sql_usize(ordinal, "worker exact reference ordinal")?, value])?;
    }
    Ok(())
}

fn load_worker_task_event(connection: &Connection, sequence: i64, event_type: &str) -> Result<DomainEvent, PersistenceError> {
    match event_type {
        "worker_task_created" => {
            let (task_id, parent, primary, plan_id, step_id, description, profile, schema, authority) = connection.query_row(
                "SELECT task_id, parent_execution_id, primary_objective_id, plan_id, plan_step_id, description, profile_id, expected_result_schema_value_id, delegated_authority_value_id FROM worker_tasks WHERE created_sequence = ?1",
                params![sequence],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, Option<String>>(3)?, row.get::<_, Option<String>>(4)?, row.get::<_, String>(5)?, row.get::<_, String>(6)?, row.get::<_, i64>(7)?, row.get::<_, i64>(8)?)),
            )?;
            let task_id = crate::WorkerTaskId::parse(task_id).map_err(|_| invalid("database contains invalid worker task id"))?;
            let supporting_objectives = load_worker_task_string_rows(connection, "worker_task_supporting_objectives", "objective_id", &task_id.to_string())?.into_iter().map(|id| parse_id(id, "objective", ObjectiveId::parse)).collect::<Result<BTreeSet<_>, _>>()?;
            let depends_on = load_worker_task_string_rows(connection, "worker_task_dependencies", "dependency_task_id", &task_id.to_string())?.into_iter().map(|id| crate::WorkerTaskId::parse(id).map_err(|_| invalid("database contains invalid worker task dependency"))).collect::<Result<BTreeSet<_>, _>>()?;
            let input_refs = load_worker_task_refs(connection, "worker_task_input_refs", "task_id", &task_id.to_string())?;
            let expected_result_schema = load_value(connection, schema)?;
            let delegated_authority: ExecutionAuthority = serde_json::from_value(load_value(connection, authority)?).map_err(|_| invalid("database contains invalid worker task authority"))?;
            let plan_step = match (plan_id, step_id) {
                (None, None) => None,
                (Some(plan), Some(step)) => Some(crate::WorkerPlanStepRef { plan_id: parse_id(plan, "plan", PlanId::parse)?, step_id: parse_id(step, "plan step", phenix_core::PlanStepId::parse)? }),
                _ => return Err(invalid("database contains malformed worker task plan binding")),
            };
            Ok(DomainEvent::WorkerTaskCreated { task: crate::WorkerTaskRecord {
                id: task_id,
                parent_execution: parse_id(parent, "execution", ExecutionId::parse)?,
                primary_objective: parse_id(primary, "objective", ObjectiveId::parse)?,
                supporting_objectives,
                plan_step,
                description,
                profile_id: WorkerProfileId::parse(profile).map_err(|_| invalid("database contains invalid worker profile"))?,
                depends_on,
                input_refs,
                expected_result_schema,
                delegated_authority,
                state: crate::WorkerTaskState::Pending,
            }})
        }
        "worker_task_verification_required" => {
            let task = connection.query_row("SELECT task_id FROM worker_task_verification_requirements WHERE sequence = ?1", params![sequence], |row| row.get::<_, String>(0))?;
            Ok(DomainEvent::WorkerTaskVerificationRequired { task_id: crate::WorkerTaskId::parse(task).map_err(|_| invalid("database contains invalid worker task id"))? })
        }
        "worker_result_recorded" => {
            let (task, execution, output) = connection.query_row("SELECT task_id, execution_id, output_value_id FROM worker_results WHERE sequence = ?1", params![sequence], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?)))?;
            Ok(DomainEvent::WorkerResultRecorded { result: crate::WorkerResultEnvelope {
                task_id: crate::WorkerTaskId::parse(task).map_err(|_| invalid("database contains invalid worker task id"))?,
                execution_id: parse_id(execution, "execution", ExecutionId::parse)?,
                output: load_value(connection, output)?,
                evidence_refs: load_worker_sequence_refs(connection, "worker_result_evidence_refs", sequence)?,
                artifact_refs: load_worker_sequence_refs(connection, "worker_result_artifact_refs", sequence)?,
            }})
        }
        "worker_verification_recorded" => {
            let (task, verifier, status, reason) = connection.query_row("SELECT task_id, verifier_execution_id, status, reason FROM worker_verifications WHERE sequence = ?1", params![sequence], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, Option<String>>(3)?)))?;
            let verifier_execution_id = parse_id(verifier, "execution", ExecutionId::parse)?;
            let evidence_refs = load_worker_sequence_refs(connection, "worker_verification_evidence_refs", sequence)?;
            let result = match status.as_str() {
                "passed" if reason.is_none() => crate::WorkerVerificationResult::Passed { verifier_execution_id, evidence_refs },
                "failed" => crate::WorkerVerificationResult::Failed { verifier_execution_id, reason: reason.ok_or_else(|| invalid("database worker verification failure is missing reason"))?, evidence_refs },
                _ => return Err(invalid("database contains invalid worker verification status")),
            };
            Ok(DomainEvent::WorkerVerificationRecorded { task_id: crate::WorkerTaskId::parse(task).map_err(|_| invalid("database contains invalid worker task id"))?, result })
        }
        "worker_failure_analysis_recorded" => {
            let (task, analyzer, diagnosis, action) = connection.query_row("SELECT task_id, analyzer_execution_id, diagnosis, proposed_action FROM worker_failure_analyses WHERE sequence = ?1", params![sequence], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?)))?;
            Ok(DomainEvent::WorkerFailureAnalysisRecorded {
                task_id: crate::WorkerTaskId::parse(task).map_err(|_| invalid("database contains invalid worker task id"))?,
                analysis: crate::WorkerFailureAnalysis {
                    analyzer_execution_id: parse_id(analyzer, "execution", ExecutionId::parse)?,
                    diagnosis,
                    evidence_refs: load_worker_sequence_refs(connection, "worker_failure_analysis_evidence_refs", sequence)?,
                    proposed_action: parse_worker_failure_action(&action)?,
                },
            })
        }
        "worker_task_started" | "worker_task_completed" | "worker_task_failed" => {
            let (task, execution, cause) = connection.query_row("SELECT task_id, execution_id, cause FROM worker_task_state_events WHERE sequence = ?1", params![sequence], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, Option<String>>(2)?)))?;
            let task_id = crate::WorkerTaskId::parse(task).map_err(|_| invalid("database contains invalid worker task id"))?;
            let execution_id = parse_id(execution, "execution", ExecutionId::parse)?;
            match event_type {
                "worker_task_started" => Ok(DomainEvent::WorkerTaskStarted { task_id, execution_id }),
                "worker_task_completed" => Ok(DomainEvent::WorkerTaskCompleted { task_id, execution_id, result_refs: load_worker_task_result_refs(connection, sequence)? }),
                "worker_task_failed" => Ok(DomainEvent::WorkerTaskFailed { task_id, execution_id, cause: cause.ok_or_else(|| invalid("database worker task failure is missing cause"))? }),
                _ => unreachable!(),
            }
        }
        _ => Err(invalid(format!("unsupported worker task event type: {event_type}"))),
    }
}

fn worker_failure_action_token(action: &crate::WorkerFailureAction) -> &'static str {
    match action {
        crate::WorkerFailureAction::Retry => "retry",
        crate::WorkerFailureAction::SuccessorTask => "successor_task",
        crate::WorkerFailureAction::InvalidatePlan => "invalidate_plan",
        crate::WorkerFailureAction::FailPlan => "fail_plan",
        crate::WorkerFailureAction::Continue => "continue",
        crate::WorkerFailureAction::FailParent => "fail_parent",
    }
}

fn parse_worker_failure_action(value: &str) -> Result<crate::WorkerFailureAction, PersistenceError> {
    match value {
        "retry" => Ok(crate::WorkerFailureAction::Retry),
        "successor_task" => Ok(crate::WorkerFailureAction::SuccessorTask),
        "invalidate_plan" => Ok(crate::WorkerFailureAction::InvalidatePlan),
        "fail_plan" => Ok(crate::WorkerFailureAction::FailPlan),
        "continue" => Ok(crate::WorkerFailureAction::Continue),
        "fail_parent" => Ok(crate::WorkerFailureAction::FailParent),
        _ => Err(invalid("database contains invalid worker failure action")),
    }
}

fn load_worker_task_string_rows(connection: &Connection, table: &str, column: &str, task_id: &str) -> Result<Vec<String>, PersistenceError> {
    let sql = format!("SELECT {column} FROM {table} WHERE task_id = ?1 ORDER BY ordinal");
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params![task_id], |row| row.get::<_, String>(0))?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn load_worker_task_refs(connection: &Connection, table: &str, key: &str, key_value: &str) -> Result<Vec<ExactReference>, PersistenceError> {
    let sql = format!("SELECT reference_value_id FROM {table} WHERE {key} = ?1 ORDER BY ordinal");
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params![key_value], |row| row.get::<_, i64>(0))?;
    rows.map(|row| {
        let value = load_value(connection, row?)?;
        serde_json::from_value(value).map_err(|_| invalid("database contains invalid worker task exact reference"))
    }).collect()
}

fn load_worker_sequence_refs(connection: &Connection, table: &str, sequence: i64) -> Result<Vec<ExactReference>, PersistenceError> {
    let sql = format!("SELECT reference_value_id FROM {table} WHERE sequence = ?1 ORDER BY ordinal");
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params![sequence], |row| row.get::<_, i64>(0))?;
    rows.map(|row| {
        let value = load_value(connection, row?)?;
        serde_json::from_value(value).map_err(|_| invalid("database contains invalid worker exact reference"))
    }).collect()
}

fn load_worker_task_result_refs(connection: &Connection, sequence: i64) -> Result<Vec<ExactReference>, PersistenceError> {
    load_worker_sequence_refs(connection, "worker_task_result_refs", sequence)
}
