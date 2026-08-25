fn execution_loop(
    executions: ExecutionQueue,
    context: ExecutionWorkerContext,
) -> Result<(), ServerError> {
    while let Some(job) = executions.next()? {
        let result = execute_execution(&job.execution_id, &job.group_id, &context).and_then(|()| {
            enqueue_pending_execution_group(&context.runtime, &job.group_id, &executions)
        });
        let group_quiescent = execution_group_quiescent(&context.runtime, &job.group_id);
        let release_group =
            result.is_err() || group_quiescent.as_ref().map_or(true, |value| *value);
        let group_released = executions.complete(&job, release_group)?;
        if group_released {
            context
                .workspace_phases
                .lock()
                .map_err(|_| ServerError::StatePoisoned("workspace phases"))?
                .remove(&job.group_id);
        }
        result?;
        group_quiescent?;
    }
    Ok(())
}

fn enqueue_pending_execution_group(
    runtime: &SharedRuntime,
    group_id: &ExecutionId,
    executions: &ExecutionQueue,
) -> Result<(), ServerError> {
    for job in pending_execution_jobs(runtime, group_id)? {
        executions.enqueue(job)?;
    }
    Ok(())
}

fn pending_execution_jobs(
    runtime: &SharedRuntime,
    group_id: &ExecutionId,
) -> Result<Vec<ExecutionJob>, ServerError> {
    let snapshot = runtime
        .lock()
        .map_err(|_| ServerError::StatePoisoned("conductor runtime"))?
        .snapshot();
    Ok(snapshot
        .executions
        .iter()
        .filter(|execution| {
            execution.state == ExecutionState::Pending
                && execution_group_id(&snapshot.executions, &execution.id).as_ref()
                    == Some(group_id)
                && !execution_has_blocking_ancestor(&snapshot.executions, &execution.id)
        })
        .map(|execution| ExecutionJob {
            execution_id: execution.id.clone(),
            session_id: execution.session_id.clone(),
            group_id: group_id.clone(),
        })
        .collect())
}

fn execution_group_quiescent(
    runtime: &SharedRuntime,
    group_id: &ExecutionId,
) -> Result<bool, ServerError> {
    let snapshot = runtime
        .lock()
        .map_err(|_| ServerError::StatePoisoned("conductor runtime"))?
        .snapshot();
    let mut found = false;
    for execution in &snapshot.executions {
        if execution_group_id(&snapshot.executions, &execution.id).as_ref() != Some(group_id) {
            continue;
        }
        found = true;
        match execution.state {
            ExecutionState::Running => return Ok(false),
            ExecutionState::Pending
                if !execution_has_blocking_ancestor(&snapshot.executions, &execution.id) =>
            {
                return Ok(false);
            }
            _ => {}
        }
    }
    Ok(found)
}

fn execution_group_id(
    executions: &[phenix_core::ExecutionSummary],
    execution_id: &ExecutionId,
) -> Option<ExecutionId> {
    let mut current = execution_id.clone();
    loop {
        let execution = executions
            .iter()
            .find(|execution| execution.id == current)?;
        let Some(parent) = execution.parent_execution.as_ref() else {
            return Some(current);
        };
        current = parent.clone();
    }
}

fn execution_has_blocking_ancestor(
    executions: &[phenix_core::ExecutionSummary],
    execution_id: &ExecutionId,
) -> bool {
    let mut parent = executions
        .iter()
        .find(|execution| execution.id == *execution_id)
        .and_then(|execution| execution.parent_execution.clone());
    while let Some(parent_id) = parent {
        let Some(parent_execution) = executions
            .iter()
            .find(|execution| execution.id == parent_id)
        else {
            return true;
        };
        if matches!(
            parent_execution.state,
            ExecutionState::Failed | ExecutionState::Cancelled | ExecutionState::Interrupted
        ) {
            return true;
        }
        parent = parent_execution.parent_execution.clone();
    }
    false
}

fn execute_execution(
    execution_id: &ExecutionId,
    group_id: &ExecutionId,
    context: &ExecutionWorkerContext,
) -> Result<(), ServerError> {
    let runtime = &context.runtime;
    let active_scopes = &context.active_scopes;
    let workspace_leases = &context.workspace_leases;
    let workspace_consistency = context.workspace_consistency.as_ref();
    let store = context.store.as_ref();
    let persist_lock = &context.persist_lock;

    let lease_request = {
        let runtime_guard = runtime
            .lock()
            .map_err(|_| ServerError::StatePoisoned("conductor runtime"))?;
        match runtime_guard.execution_state(execution_id) {
            Some(ExecutionState::Pending) => {
                let snapshot = runtime_guard.snapshot();
                if execution_has_blocking_ancestor(&snapshot.executions, execution_id) {
                    return Ok(());
                }
                runtime_guard.workspace_lease_request(execution_id)
            }
            Some(state) if is_terminal_state(&state) => return Ok(()),
            Some(_) => return Ok(()),
            None => return Ok(()),
        }
    };
    let lease_request = match lease_request {
        Ok(request) => request,
        Err(error) => {
            fail_shared_execution(
                runtime,
                execution_id,
                map_conductor_error(error),
                store,
                persist_lock,
            )?;
            return Ok(());
        }
    };
    let workspace_id = lease_request.workspace_id.clone();
    let lease_mode = lease_request.mode;
    let _workspace_lease = workspace_leases.acquire(lease_request)?;
    let starts_write_phase = context
        .workspace_phases
        .lock()
        .map_err(|_| ServerError::StatePoisoned("workspace phases"))?
        .entry(group_id.clone())
        .or_default()
        .enter(lease_mode);

    if starts_write_phase && workspace_id.as_str() != IN_MEMORY_WORKSPACE_ID {
        let consistency = workspace_consistency
            .ok_or_else(|| ServerError::WorkspaceConsistencyUnavailable(workspace_id.clone()))?;
        let files = consistency.checkpoint_baseline()?;
        {
            let mut runtime_guard = runtime
                .lock()
                .map_err(|_| ServerError::StatePoisoned("conductor runtime"))?;
            runtime_guard.record_workspace_checkpoint(execution_id, workspace_id.clone(), files)?;
        }
        persist_shared(runtime, store, persist_lock)?;
    }

    let provider_kind = {
        let runtime_guard = runtime
            .lock()
            .map_err(|_| ServerError::StatePoisoned("conductor runtime"))?;
        match runtime_guard.execution_state(execution_id) {
            Some(ExecutionState::Pending) => runtime_guard.execution_provider_kind(execution_id),
            Some(state) if is_terminal_state(&state) => return Ok(()),
            Some(_) => return Ok(()),
            None => return Ok(()),
        }
    };
    let provider_kind = match provider_kind {
        Ok(kind) => kind,
        Err(error) => {
            fail_shared_execution(
                runtime,
                execution_id,
                map_conductor_error(error),
                store,
                persist_lock,
            )?;
            return Ok(());
        }
    };

    match provider_kind {
        ExecutionProviderKind::Model => {
            execute_model_execution(execution_id, &workspace_id, context)
        }
        _ => execute_provider_execution(execution_id, runtime, active_scopes, store, persist_lock),
    }
}

fn execute_model_execution(
    execution_id: &ExecutionId,
    workspace_id: &WorkspaceId,
    context: &ExecutionWorkerContext,
) -> Result<(), ServerError> {
    let runtime = &context.runtime;
    let backends = &context.backends;
    let active_scopes = &context.active_scopes;
    let workspace_leases = &context.workspace_leases;
    let workspace_consistency = context.workspace_consistency.as_ref();
    let store = context.store.as_ref();
    let persist_lock = &context.persist_lock;
    let resolved = {
        let mut runtime_guard = runtime
            .lock()
            .map_err(|_| ServerError::StatePoisoned("conductor runtime"))?;
        let mut resolved = runtime_guard.resolve_invocation(execution_id);
        if let Ok(invocation) = &mut resolved {
            if let Err(error) = semantic_tools::extend_semantic_tools(
                &runtime_guard,
                invocation,
                workspace_consistency.is_some() && workspace_id.as_str() != IN_MEMORY_WORKSPACE_ID,
            ) {
                resolved = Err(error);
            }
        }
        resolved
    };
    let resolved = match resolved {
        Ok(resolved) => resolved,
        Err(error) => {
            fail_shared_execution(
                runtime,
                execution_id,
                map_conductor_error(error),
                store,
                persist_lock,
            )?;
            return Ok(());
        }
    };
    // A routed decision is durable audit state. Persist it before any backend
    // session can observe or execute the resolved invocation.
    persist_shared(runtime, store, persist_lock)?;

    let backend_id = resolved.model.backend.clone();
    let Some(backend) = backends.get(&backend_id).cloned() else {
        fail_shared_execution(
            runtime,
            execution_id,
            map_backend_error(BackendError::Unsupported(format!(
                "backend is not registered: {backend_id}"
            ))),
            store,
            persist_lock,
        )?;
        return Ok(());
    };

    let capabilities = backend
        .lock()
        .map_err(|_| ServerError::StatePoisoned("backend"))?
        .capabilities();
    let prepared = {
        let runtime_guard = runtime
            .lock()
            .map_err(|_| ServerError::StatePoisoned("conductor runtime"))?;
        runtime_guard.prepare_invocation(resolved, &capabilities)
    };
    let prepared = match prepared {
        Ok(prepared) => prepared,
        Err(error) => {
            fail_shared_execution(
                runtime,
                execution_id,
                map_conductor_error(error),
                store,
                persist_lock,
            )?;
            return Ok(());
        }
    };

    let backend_session = {
        let mut backend = backend
            .lock()
            .map_err(|_| ServerError::StatePoisoned("backend"))?;
        let request = prepared.backend_session_request();
        if capabilities.persistent_sessions
            && matches!(
                &prepared.resolved.requested_target,
                ExecutionTarget::Fixed(_)
            )
        {
            backend.open_persistent_session(&prepared.resolved.session_id, request)
        } else {
            backend.open_session(request)
        }
    };
    let backend_session = match backend_session {
        Ok(session) => session,
        Err(error) => {
            fail_shared_execution(
                runtime,
                execution_id,
                map_backend_error(error),
                store,
                persist_lock,
            )?;
            return Ok(());
        }
    };

    active_scopes
        .lock()
        .map_err(|_| ServerError::StatePoisoned("active scopes"))?
        .insert(
            execution_id.clone(),
            LiveExecutionScope::Backend(backend_session.clone()),
        );
    let _scope_lease = LiveExecutionLease {
        scopes: active_scopes.clone(),
        execution_id: execution_id.clone(),
    };

    if !begin_execution(runtime, execution_id, store, persist_lock)? {
        return Ok(());
    }

    let mut host = SharedRuntimeHost {
        runtime: runtime.clone(),
        execution_id: execution_id.clone(),
        allowed_tools: prepared.allowed_tools(),
        workspace_id: workspace_id.clone(),
        workspace_leases: workspace_leases.clone(),
        workspace_consistency: workspace_consistency.cloned(),
        store: store.cloned(),
        persist_lock: persist_lock.clone(),
    };
    let result = backend_session.execute(prepared.backend_execution_request(), &mut host);
    finish_model_execution(runtime, execution_id, result, store, persist_lock)
}

fn execute_provider_execution(
    execution_id: &ExecutionId,
    runtime: &SharedRuntime,
    active_scopes: &ActiveScopes,
    store: Option<&SqliteStore>,
    persist_lock: &Arc<Mutex<()>>,
) -> Result<(), ServerError> {
    let prepared = {
        let runtime_guard = runtime
            .lock()
            .map_err(|_| ServerError::StatePoisoned("conductor runtime"))?;
        runtime_guard.prepare_provider_execution(execution_id)
    };
    let (provider, request) = match prepared {
        Ok(prepared) => prepared,
        Err(error) => {
            fail_shared_execution(
                runtime,
                execution_id,
                map_conductor_error(error),
                store,
                persist_lock,
            )?;
            return Ok(());
        }
    };

    active_scopes
        .lock()
        .map_err(|_| ServerError::StatePoisoned("active scopes"))?
        .insert(
            execution_id.clone(),
            LiveExecutionScope::Provider(provider.clone()),
        );
    let _scope_lease = LiveExecutionLease {
        scopes: active_scopes.clone(),
        execution_id: execution_id.clone(),
    };

    if !begin_execution(runtime, execution_id, store, persist_lock)? {
        return Ok(());
    }

    let mut host = SharedProviderHost {
        runtime: runtime.clone(),
        execution_id: execution_id.clone(),
        store: store.cloned(),
        persist_lock: persist_lock.clone(),
    };
    let result = provider.execute(&request, &mut host);
    finish_provider_execution(runtime, execution_id, result, store, persist_lock)
}

fn begin_execution(
    runtime: &SharedRuntime,
    execution_id: &ExecutionId,
    store: Option<&SqliteStore>,
    persist_lock: &Arc<Mutex<()>>,
) -> Result<bool, ServerError> {
    let should_execute = {
        let mut runtime_guard = runtime
            .lock()
            .map_err(|_| ServerError::StatePoisoned("conductor runtime"))?;
        match runtime_guard.execution_state(execution_id) {
            Some(ExecutionState::Pending) => {
                runtime_guard.set_state(execution_id, ExecutionState::Running)?;
                true
            }
            Some(state) if is_terminal_state(&state) => false,
            Some(_) => {
                fail_runtime_execution(
                    &mut runtime_guard,
                    execution_id,
                    protocol_error(
                        ErrorCode::InvalidRequest,
                        format!("execution is not pending: {execution_id}"),
                    ),
                )?;
                false
            }
            None => false,
        }
    };
    persist_shared(runtime, store, persist_lock)?;
    Ok(should_execute)
}

fn finish_model_execution(
    runtime: &SharedRuntime,
    execution_id: &ExecutionId,
    result: Result<(), BackendError>,
    store: Option<&SqliteStore>,
    persist_lock: &Arc<Mutex<()>>,
) -> Result<(), ServerError> {
    {
        let mut runtime_guard = runtime
            .lock()
            .map_err(|_| ServerError::StatePoisoned("conductor runtime"))?;
        if runtime_guard.execution_state(execution_id) == Some(ExecutionState::Running) {
            match result {
                Ok(()) => runtime_guard.set_state(execution_id, ExecutionState::Completed)?,
                Err(error) => fail_runtime_execution(
                    &mut runtime_guard,
                    execution_id,
                    map_backend_error(error),
                )?,
            }
        }
    }
    persist_shared(runtime, store, persist_lock)?;
    Ok(())
}

fn finish_provider_execution(
    runtime: &SharedRuntime,
    execution_id: &ExecutionId,
    result: Result<(), ExecutionProviderError>,
    store: Option<&SqliteStore>,
    persist_lock: &Arc<Mutex<()>>,
) -> Result<(), ServerError> {
    {
        let mut runtime_guard = runtime
            .lock()
            .map_err(|_| ServerError::StatePoisoned("conductor runtime"))?;
        if runtime_guard.execution_state(execution_id) == Some(ExecutionState::Running) {
            match result {
                Ok(()) => runtime_guard.set_state(execution_id, ExecutionState::Completed)?,
                Err(error) => fail_runtime_execution(
                    &mut runtime_guard,
                    execution_id,
                    map_execution_provider_error(error),
                )?,
            }
        }
    }
    persist_shared(runtime, store, persist_lock)?;
    Ok(())
}

fn fail_shared_execution(
    runtime: &SharedRuntime,
    execution_id: &ExecutionId,
    error: ProtocolError,
    store: Option<&SqliteStore>,
    persist_lock: &Arc<Mutex<()>>,
) -> Result<(), ServerError> {
    {
        let mut runtime = runtime
            .lock()
            .map_err(|_| ServerError::StatePoisoned("conductor runtime"))?;
        fail_runtime_execution(&mut runtime, execution_id, error)?;
    }
    persist_shared(runtime, store, persist_lock)?;
    Ok(())
}
