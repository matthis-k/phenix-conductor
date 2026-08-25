impl ConductorServer {
    fn export_session_debug(&self, session_id: &SessionId) -> Result<Reply, ProtocolError> {
        let runtime = self
            .lock_runtime()
            .map_err(|error| protocol_error(ErrorCode::BackendProtocol, error.to_string()))?;
        let session = runtime.session(session_id).map_err(map_conductor_error)?;
        let (workspace, versions) = match &self.workspace_consistency {
            Some(consistency) => {
                let versions = consistency.checkpoint_baseline().map_err(|error| {
                    protocol_error(ErrorCode::BackendProtocol, error.to_string())
                })?;
                (
                    consistency.descriptor(session.workspace_id.clone()),
                    versions,
                )
            }
            None => (
                WorkspaceDescriptor {
                    id: session.workspace_id,
                    root: PathBuf::new(),
                    scratch_paths: BTreeSet::new(),
                },
                BTreeMap::new(),
            ),
        };
        runtime
            .build_session_debug_bundle(session_id, workspace, &versions)
            .map(|bundle| Reply::SessionDebug {
                bundle: Box::new(bundle),
            })
            .map_err(map_conductor_error)
    }

    fn cancel_execution(&self, root: &ExecutionId) -> Result<Reply, ProtocolError> {
        let active = self
            .active_scopes
            .lock()
            .map_err(|_| protocol_error(ErrorCode::BackendProtocol, "active scope lock poisoned"))?
            .iter()
            .map(|(id, scope)| (id.clone(), scope.clone()))
            .collect::<Vec<_>>();

        let cancelled_active = {
            let mut runtime = self.runtime.lock().map_err(|_| {
                protocol_error(
                    ErrorCode::BackendProtocol,
                    "conductor runtime lock poisoned",
                )
            })?;
            runtime
                .cancel_execution(root)
                .map_err(map_conductor_error)?;
            active
                .into_iter()
                .filter(|(id, _)| runtime.execution_state(id) == Some(ExecutionState::Cancelled))
                .collect::<Vec<_>>()
        };

        for (execution_id, scope) in cancelled_active {
            scope.cancel(&execution_id)?;
        }
        Ok(Reply::Accepted)
    }

    fn close_session(&mut self, session_id: &SessionId) -> Result<Reply, ProtocolError> {
        let session = self
            .runtime
            .lock()
            .map_err(|_| {
                protocol_error(
                    ErrorCode::BackendProtocol,
                    "conductor runtime lock poisoned",
                )
            })?
            .validate_session_close(session_id)
            .map_err(map_conductor_error)?;
        if session.state == SessionState::Closed {
            return Ok(Reply::Session { session });
        }

        // Backend disposal precedes the durable close marker. A failed backend
        // therefore leaves the Phenix session active and retryable; backends are
        // required to make persistent close idempotent because earlier fanout
        // members may already have completed successfully.
        for backend in self.backends.values() {
            backend
                .lock()
                .map_err(|_| protocol_error(ErrorCode::BackendTransport, "backend lock poisoned"))?
                .close_persistent_session(session_id)
                .map_err(map_backend_error)?;
        }

        let session = self
            .runtime
            .lock()
            .map_err(|_| {
                protocol_error(
                    ErrorCode::BackendProtocol,
                    "conductor runtime lock poisoned",
                )
            })?
            .close_session(session_id)
            .map_err(map_conductor_error)?;
        Ok(Reply::Session { session })
    }
}
