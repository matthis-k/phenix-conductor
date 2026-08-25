impl ConductorRuntime {
    pub fn bind_workspace(&mut self, workspace_id: WorkspaceId) -> Result<(), ConductorError> {
        if let Some(session) = self
            .sessions
            .values()
            .find(|session| session.summary.workspace_id != workspace_id)
        {
            return Err(ConductorError::WorkspaceMismatch {
                expected: session.summary.workspace_id.clone(),
                actual: workspace_id,
            });
        }
        self.workspace_id = workspace_id;
        Ok(())
    }

    pub fn execution_read_set(
        &self,
        execution_id: &ExecutionId,
    ) -> Result<ExecutionReadSet, ConductorError> {
        if !self.executions.contains_key(execution_id) {
            return Err(ConductorError::UnknownExecution(execution_id.clone()));
        }
        Ok(self
            .read_sets
            .get(execution_id)
            .cloned()
            .unwrap_or_else(|| ExecutionReadSet::new(execution_id.clone())))
    }

    pub fn execution_workspace_validity(
        &self,
        execution_id: &ExecutionId,
        current: &BTreeMap<PathBuf, FileVersion>,
    ) -> Result<ExecutionWorkspaceValidity, ConductorError> {
        Ok(self
            .execution_read_set(execution_id)?
            .validity_against(current))
    }

    fn next_file_observation_id(&self) -> FileObservationId {
        let mut ordinal = self
            .journal
            .entries
            .iter()
            .filter(|entry| matches!(entry.event, DomainEvent::WorkspaceFileObserved { .. }))
            .count()
            + 1;
        loop {
            let candidate = FileObservationId::parse(format!("file-observation-{ordinal}"))
                .expect("generated file observation id");
            let exists = self.journal.entries.iter().any(|entry| {
                matches!(
                    &entry.event,
                    DomainEvent::WorkspaceFileObserved { observation, .. }
                        if observation.id == candidate
                )
            });
            if !exists {
                return candidate;
            }
            ordinal += 1;
        }
    }

    fn record_file_observation(
        &mut self,
        execution_id: &ExecutionId,
        observation: FileObservationInput,
    ) -> Result<FileObservation, ConductorError> {
        let observation = FileObservation {
            id: self.next_file_observation_id(),
            path: observation.path,
            version: observation.version,
        };
        self.record_domain_event(DomainEvent::WorkspaceFileObserved {
            execution_id: execution_id.clone(),
            observation: observation.clone(),
        })?;
        Ok(observation)
    }

    pub(crate) fn workspace_lease_request(
        &self,
        execution_id: &ExecutionId,
    ) -> Result<WorkspaceLeaseRequest, ConductorError> {
        let execution = self
            .executions
            .get(execution_id)
            .ok_or_else(|| ConductorError::UnknownExecution(execution_id.clone()))?;
        let session = self
            .sessions
            .get(&execution.summary.session_id)
            .expect("execution session invariant");
        Ok(WorkspaceLeaseRequest {
            workspace_id: session.summary.workspace_id.clone(),
            execution_id: execution_id.clone(),
            mode: execution.authority.filesystem.into(),
        })
    }
}
