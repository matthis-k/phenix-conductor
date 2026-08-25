    pub fn install_workspace_consistency(
        &mut self,
        descriptor: WorkspaceDescriptor,
    ) -> Result<(), ServerError> {
        let consistency = WorkspaceConsistency::new(&descriptor)?;
        self.lock_runtime()?.bind_workspace(descriptor.id)?;
        self.workspace_consistency = Some(consistency);
        Ok(())
    }

    pub fn install_workspace_tools_into(
        &self,
        configuration: &mut CompiledConfiguration,
    ) -> Result<(), ServerError> {
        let consistency = self
            .workspace_consistency
            .clone()
            .ok_or(ServerError::WorkspaceConsistencyNotInstalled)?;
        workspace_tools::register_into(configuration, consistency)?;
        Ok(())
    }

    pub fn install_workspace_tools(&mut self) -> Result<(), ServerError> {
        let consistency = self
            .workspace_consistency
            .clone()
            .ok_or(ServerError::WorkspaceConsistencyNotInstalled)?;
        let mut runtime = self.lock_runtime()?;
        workspace_tools::register(&mut runtime, consistency)?;
        Ok(())
    }

    fn capture_workspace_checkpoint(
        &self,
        execution_id: &ExecutionId,
    ) -> Result<Reply, ProtocolError> {
        let consistency = self.workspace_consistency.as_ref().ok_or_else(|| {
            protocol_error(
                ErrorCode::InvalidRequest,
                "workspace consistency is not installed",
            )
        })?;
        let request = self
            .lock_runtime()
            .map_err(|error| protocol_error(ErrorCode::BackendProtocol, error.to_string()))?
            .workspace_lease_request(execution_id)
            .map_err(map_conductor_error)?;
        if request.mode != WorkspaceLeaseMode::Write {
            return Err(protocol_error(
                ErrorCode::InvalidRequest,
                "workspace checkpoints require filesystem-write authority",
            ));
        }
        if !self
            .workspace_leases
            .holds_write(&request.workspace_id, execution_id)
            .map_err(|error| protocol_error(ErrorCode::BackendProtocol, error.to_string()))?
        {
            return Err(protocol_error(
                ErrorCode::InvalidRequest,
                "workspace checkpoints require the execution's active write lease",
            ));
        }
        let files = consistency
            .checkpoint_baseline()
            .map_err(|error| protocol_error(ErrorCode::BackendProtocol, error.to_string()))?;
        self.lock_runtime()
            .map_err(|error| protocol_error(ErrorCode::BackendProtocol, error.to_string()))?
            .record_workspace_checkpoint(execution_id, request.workspace_id, files)
            .map_err(map_conductor_error)?;
        Ok(Reply::Accepted)
    }
