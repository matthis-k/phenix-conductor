impl ConductorRuntime {
    pub fn create_terminal(
        &mut self,
        execution_id: &ExecutionId,
    ) -> Result<TerminalRecord, ConductorError> {
        let authority = self.execution_authority(execution_id)?.clone();
        self.next_terminal += 1;
        let terminal = TerminalRecord {
            id: TerminalId::parse(format!("terminal-{}", self.next_terminal))
                .expect("generated terminal id"),
            owner: DurableResourceOwner::Execution(execution_id.clone()),
            created_by: execution_id.clone(),
            authority,
            state: DurableProcessState::Running,
            output_refs: Vec::new(),
        };
        self.record_domain_event(DomainEvent::TerminalCreated {
            terminal: terminal.clone(),
        })?;
        Ok(terminal)
    }

    pub fn create_job(&mut self, execution_id: &ExecutionId) -> Result<JobRecord, ConductorError> {
        let authority = self.execution_authority(execution_id)?.clone();
        self.next_job += 1;
        let job = JobRecord {
            id: JobId::parse(format!("job-{}", self.next_job)).expect("generated job id"),
            owner: DurableResourceOwner::Execution(execution_id.clone()),
            created_by: execution_id.clone(),
            authority,
            state: DurableProcessState::Running,
            output_refs: Vec::new(),
        };
        self.record_domain_event(DomainEvent::JobCreated { job: job.clone() })?;
        Ok(job)
    }

    #[must_use]
    pub fn terminal(&self, id: &TerminalId) -> Option<&TerminalRecord> {
        self.terminals.get(id)
    }

    #[must_use]
    pub fn job(&self, id: &JobId) -> Option<&JobRecord> {
        self.jobs.get(id)
    }

    pub fn bind_terminal_runtime_handle(
        &mut self,
        id: &TerminalId,
        handle: impl RuntimeProcessHandle + 'static,
    ) -> Result<(), ConductorError> {
        let terminal = self.require_terminal(id)?;
        if terminal.state != DurableProcessState::Running {
            return Err(self.process_resource_error(format!(
                "terminal {} is not running",
                id.as_str()
            )));
        }
        let created_by = terminal.created_by.clone();
        let creation_authority = terminal.authority.clone();
        let current_authority = self.execution_authority(&created_by)?;
        if !creation_authority.permits(handle.authority())
            || !current_authority.permits(handle.authority())
        {
            return Err(self.process_resource_error(format!(
                "terminal {} runtime handle exceeds process-resource authority",
                id.as_str()
            )));
        }
        self.terminal_runtime_handles
            .insert(id.clone(), Box::new(handle));
        Ok(())
    }

    pub fn bind_job_runtime_handle(
        &mut self,
        id: &JobId,
        handle: impl RuntimeProcessHandle + 'static,
    ) -> Result<(), ConductorError> {
        let job = self.require_job(id)?;
        if job.state != DurableProcessState::Running {
            return Err(self.process_resource_error(format!(
                "job {} is not running",
                id.as_str()
            )));
        }
        let created_by = job.created_by.clone();
        let creation_authority = job.authority.clone();
        let current_authority = self.execution_authority(&created_by)?;
        if !creation_authority.permits(handle.authority())
            || !current_authority.permits(handle.authority())
        {
            return Err(self.process_resource_error(format!(
                "job {} runtime handle exceeds process-resource authority",
                id.as_str()
            )));
        }
        self.job_runtime_handles.insert(id.clone(), Box::new(handle));
        Ok(())
    }

    #[must_use]
    pub fn terminal_has_runtime_handle(&self, id: &TerminalId) -> bool {
        self.terminal_runtime_handles.contains_key(id)
    }

    #[must_use]
    pub fn job_has_runtime_handle(&self, id: &JobId) -> bool {
        self.job_runtime_handles.contains_key(id)
    }

    pub fn finish_terminal(
        &mut self,
        id: &TerminalId,
        code: Option<i32>,
    ) -> Result<(), ConductorError> {
        self.require_terminal(id)?;
        self.record_domain_event(DomainEvent::TerminalStateChanged {
            terminal_id: id.clone(),
            state: DurableProcessState::Exited { code },
        })?;
        self.terminal_runtime_handles.remove(id);
        Ok(())
    }

    pub fn finish_job(&mut self, id: &JobId, code: Option<i32>) -> Result<(), ConductorError> {
        self.require_job(id)?;
        self.record_domain_event(DomainEvent::JobStateChanged {
            job_id: id.clone(),
            state: DurableProcessState::Exited { code },
        })?;
        self.job_runtime_handles.remove(id);
        Ok(())
    }

    pub fn promote_job_to_workspace(&mut self, id: &JobId) -> Result<(), ConductorError> {
        let job = self.require_job(id)?;
        self.execution_authority(&job.created_by)?;
        let workspace_id = self.workspace_id.clone();
        self.record_domain_event(DomainEvent::JobPromoted {
            job_id: id.clone(),
            workspace_id,
        })
    }

    pub fn record_terminal_output(
        &mut self,
        id: &TerminalId,
        output: phenix_core::ExactReference,
    ) -> Result<(), ConductorError> {
        self.require_terminal(id)?;
        self.record_domain_event(DomainEvent::TerminalOutputRecorded {
            terminal_id: id.clone(),
            output,
        })
    }

    pub fn record_job_output(
        &mut self,
        id: &JobId,
        output: phenix_core::ExactReference,
    ) -> Result<(), ConductorError> {
        self.require_job(id)?;
        self.record_domain_event(DomainEvent::JobOutputRecorded {
            job_id: id.clone(),
            output,
        })
    }

    pub fn reconcile_process_resource_authority(
        &mut self,
        execution_id: &ExecutionId,
    ) -> Result<(), ConductorError> {
        let current = self.execution_authority(execution_id)?.clone();
        let terminal_ids: Vec<_> = self
            .terminals
            .values()
            .filter(|resource| {
                resource.created_by == *execution_id
                    && resource.state == DurableProcessState::Running
                    && !current.permits(&resource.authority)
            })
            .map(|resource| resource.id.clone())
            .collect();
        let job_ids: Vec<_> = self
            .jobs
            .values()
            .filter(|resource| {
                resource.created_by == *execution_id
                    && resource.state == DurableProcessState::Running
                    && !current.permits(&resource.authority)
            })
            .map(|resource| resource.id.clone())
            .collect();
        for terminal_id in terminal_ids {
            self.record_domain_event(DomainEvent::TerminalStateChanged {
                terminal_id: terminal_id.clone(),
                state: DurableProcessState::Revoked,
            })?;
            self.terminal_runtime_handles.remove(&terminal_id);
        }
        for job_id in job_ids {
            self.record_domain_event(DomainEvent::JobStateChanged {
                job_id: job_id.clone(),
                state: DurableProcessState::Revoked,
            })?;
            self.job_runtime_handles.remove(&job_id);
        }
        Ok(())
    }

    fn require_terminal(&self, id: &TerminalId) -> Result<&TerminalRecord, ConductorError> {
        self.terminals
            .get(id)
            .ok_or_else(|| self.process_resource_error(format!("unknown terminal: {}", id.as_str())))
    }

    fn require_job(&self, id: &JobId) -> Result<&JobRecord, ConductorError> {
        self.jobs
            .get(id)
            .ok_or_else(|| self.process_resource_error(format!("unknown job: {}", id.as_str())))
    }

    fn process_resource_error(&self, message: String) -> ConductorError {
        ConductorError::InvalidExecutionData {
            execution_id: ExecutionId::parse("process-resource").expect("static id"),
            message,
        }
    }
}

#[cfg(test)]
mod process_resource_tests {
    use super::*;

    #[derive(Debug)]
    struct TestRuntimeHandle {
        authority: ExecutionAuthority,
    }

    impl RuntimeProcessHandle for TestRuntimeHandle {
        fn authority(&self) -> &ExecutionAuthority {
            &self.authority
        }
    }

    fn test_handle(authority: &ExecutionAuthority) -> TestRuntimeHandle {
        TestRuntimeHandle {
            authority: authority.clone(),
        }
    }

    fn process_runtime() -> (ConductorRuntime, SessionSummary, ExecutionSummary) {
        let mut runtime = ConductorRuntime::new();
        let session = runtime
            .create_session(
                None,
                None,
                ExecutionTarget::Fixed(ModelTarget {
                    backend: phenix_core::BackendId::parse("mock").unwrap(),
                    provider: phenix_core::ProviderId::parse("mock").unwrap(),
                    model: phenix_core::ModelId::parse("mock").unwrap(),
                    inference: phenix_core::InferenceOptions::default(),
                }),
            )
            .unwrap();
        let execution = runtime
            .submit(&session.id, "run process resources")
            .unwrap();
        (runtime, session, execution)
    }

    fn process_agent(id: &str) -> CallableDescriptor {
        CallableDescriptor {
            id: CallableId::parse(id).unwrap(),
            kind: CallableKind::Agent,
            description: "process resource test agent".to_owned(),
            input_schema: json!({"type": "object"}),
            output_schema: json!({"type": "object"}),
            capabilities: phenix_core::CapabilitySet::default(),
            policy: phenix_core::CallablePolicy::default(),
        }
    }

    fn process_authority(
        filesystem: phenix_core::FilesystemAuthority,
        network: phenix_core::NetworkAuthority,
        repository: phenix_core::RepositoryAuthority,
        ipc: &[&str],
        callables: &[&str],
    ) -> ExecutionAuthority {
        ExecutionAuthority {
            filesystem,
            network,
            repository,
            ipc: ipc.iter().map(|value| (*value).to_owned()).collect(),
            secrets: BTreeSet::new(),
            callables: callables
                .iter()
                .map(|value| CallableId::parse(*value).unwrap())
                .collect(),
        }
    }

    #[test]
    fn process_resource_lifecycle_replays_without_runtime_handles() {
        let (mut runtime, session, execution) = process_runtime();
        let terminal = runtime.create_terminal(&execution.id).unwrap();
        let job = runtime.create_job(&execution.id).unwrap();
        runtime.promote_job_to_workspace(&job.id).unwrap();
        runtime.finish_terminal(&terminal.id, Some(0)).unwrap();
        runtime.finish_job(&job.id, None).unwrap();
        let journal = runtime.journal().clone();
        let restored = ConductorRuntime::restore(journal).unwrap();
        assert_eq!(
            restored.terminal(&terminal.id).unwrap().state,
            DurableProcessState::Exited { code: Some(0) }
        );
        assert_eq!(
            restored.job(&job.id).unwrap().state,
            DurableProcessState::Exited { code: None }
        );
        assert_eq!(
            restored.job(&job.id).unwrap().owner,
            DurableResourceOwner::Workspace(session.workspace_id)
        );
    }

    #[test]
    fn live_handles_are_keyed_by_durable_identity_and_not_replayed() {
        let (mut runtime, _, execution) = process_runtime();
        let terminal = runtime.create_terminal(&execution.id).unwrap();
        let job = runtime.create_job(&execution.id).unwrap();
        runtime
            .bind_terminal_runtime_handle(&terminal.id, test_handle(&terminal.authority))
            .unwrap();
        runtime
            .bind_job_runtime_handle(&job.id, test_handle(&job.authority))
            .unwrap();
        assert!(runtime.terminal_has_runtime_handle(&terminal.id));
        assert!(runtime.job_has_runtime_handle(&job.id));

        let restored = ConductorRuntime::restore(runtime.journal().clone()).unwrap();
        assert_eq!(
            restored.terminal(&terminal.id).unwrap().state,
            DurableProcessState::Running
        );
        assert_eq!(
            restored.job(&job.id).unwrap().state,
            DurableProcessState::Running
        );
        assert!(!restored.terminal_has_runtime_handle(&terminal.id));
        assert!(!restored.job_has_runtime_handle(&job.id));
    }

    #[test]
    fn exit_and_authority_revocation_drop_live_handles() {
        let (mut runtime, _, execution) = process_runtime();
        runtime
            .executions
            .get_mut(&execution.id)
            .unwrap()
            .authority
            .ipc
            .insert("runtime-socket".to_owned());
        let terminal = runtime.create_terminal(&execution.id).unwrap();
        let job = runtime.create_job(&execution.id).unwrap();
        runtime
            .bind_terminal_runtime_handle(&terminal.id, test_handle(&terminal.authority))
            .unwrap();
        runtime
            .bind_job_runtime_handle(&job.id, test_handle(&job.authority))
            .unwrap();

        runtime.finish_terminal(&terminal.id, Some(0)).unwrap();
        assert!(!runtime.terminal_has_runtime_handle(&terminal.id));

        runtime
            .executions
            .get_mut(&execution.id)
            .unwrap()
            .authority
            .ipc
            .clear();
        runtime
            .reconcile_process_resource_authority(&execution.id)
            .unwrap();
        assert_eq!(
            runtime.job(&job.id).unwrap().state,
            DurableProcessState::Revoked
        );
        assert!(!runtime.job_has_runtime_handle(&job.id));
    }

    #[test]
    fn child_process_resource_authority_cannot_exceed_parent_delegation() {
        let mut runtime = ConductorRuntime::new();
        let parent_authority = process_authority(
            phenix_core::FilesystemAuthority::ReadOnly,
            phenix_core::NetworkAuthority::None,
            phenix_core::RepositoryAuthority::Read,
            &["parent-ipc"],
            &["agent.child"],
        );
        let child_maximum = process_authority(
            phenix_core::FilesystemAuthority::Write,
            phenix_core::NetworkAuthority::Outbound,
            phenix_core::RepositoryAuthority::Write,
            &["parent-ipc", "child-only-ipc"],
            &[],
        );
        runtime
            .register_agent(AgentDefinition::new(
                process_agent("agent.parent"),
                parent_authority.clone(),
            ))
            .unwrap();
        runtime
            .register_agent(AgentDefinition::new(
                process_agent("agent.child"),
                child_maximum.clone(),
            ))
            .unwrap();
        let session = runtime
            .create_session(
                None,
                None,
                ExecutionTarget::Fixed(ModelTarget {
                    backend: phenix_core::BackendId::parse("mock").unwrap(),
                    provider: phenix_core::ProviderId::parse("mock").unwrap(),
                    model: phenix_core::ModelId::parse("mock").unwrap(),
                    inference: phenix_core::InferenceOptions::default(),
                }),
            )
            .unwrap();
        let parent = runtime
            .start_session_callable(
                &session.id,
                &CallableId::parse("agent.parent").unwrap(),
                "parent",
            )
            .unwrap();
        let child = runtime
            .start_agent(
                &parent.id,
                &CallableId::parse("agent.child").unwrap(),
                "child",
            )
            .unwrap();
        let terminal = runtime.create_terminal(&child.id).unwrap();
        let job = runtime.create_job(&child.id).unwrap();
        let expected = parent_authority.attenuate(&child_maximum);

        assert_eq!(terminal.authority, expected);
        assert_eq!(job.authority, expected);
        assert!(!terminal.authority.ipc.contains("child-only-ipc"));
        assert_eq!(
            terminal.authority.filesystem,
            phenix_core::FilesystemAuthority::ReadOnly
        );
        assert_eq!(terminal.authority.network, phenix_core::NetworkAuthority::None);
        assert!(runtime
            .bind_terminal_runtime_handle(&terminal.id, test_handle(&child_maximum))
            .is_err());
        assert!(!runtime.terminal_has_runtime_handle(&terminal.id));
    }

    #[test]
    fn replay_rejects_invalid_terminal_transition_and_duplicate_promotion() {
        let (mut runtime, session, execution) = process_runtime();
        let terminal = runtime.create_terminal(&execution.id).unwrap();
        runtime.finish_terminal(&terminal.id, Some(0)).unwrap();
        let mut invalid_terminal = runtime.journal().clone();
        invalid_terminal.entries.push(JournalEntry {
            sequence: invalid_terminal.entries.last().unwrap().sequence + 1,
            event: DomainEvent::TerminalStateChanged {
                terminal_id: terminal.id.clone(),
                state: DurableProcessState::Revoked,
            },
        });
        assert!(ConductorRuntime::restore(invalid_terminal).is_err());

        let (mut runtime, _, execution) = process_runtime();
        let job = runtime.create_job(&execution.id).unwrap();
        runtime.promote_job_to_workspace(&job.id).unwrap();
        let mut invalid_promotion = runtime.journal().clone();
        invalid_promotion.entries.push(JournalEntry {
            sequence: invalid_promotion.entries.last().unwrap().sequence + 1,
            event: DomainEvent::JobPromoted {
                job_id: job.id,
                workspace_id: session.workspace_id,
            },
        });
        assert!(ConductorRuntime::restore(invalid_promotion).is_err());
    }

    #[test]
    fn sqlite_roundtrips_process_metadata_authority_promotion_and_output_refs() {
        let (mut runtime, session, execution) = process_runtime();
        runtime
            .executions
            .get_mut(&execution.id)
            .unwrap()
            .authority
            .ipc
            .insert("runtime-socket".to_owned());
        let terminal = runtime.create_terminal(&execution.id).unwrap();
        let job = runtime.create_job(&execution.id).unwrap();
        let output = phenix_core::ExactReference::Execution(execution.id.clone());
        runtime
            .record_terminal_output(&terminal.id, output.clone())
            .unwrap();
        runtime.record_job_output(&job.id, output.clone()).unwrap();
        runtime.promote_job_to_workspace(&job.id).unwrap();

        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "phenix-process-resources-{}-{nonce}",
            std::process::id()
        ));
        let store = SqliteStore::new(directory.join("state.db"));
        store.save(runtime.journal()).unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded, *runtime.journal());
        let restored = ConductorRuntime::restore(loaded).unwrap();

        let restored_terminal = restored.terminal(&terminal.id).unwrap();
        assert_eq!(restored_terminal.authority, terminal.authority);
        assert_eq!(restored_terminal.output_refs, vec![output.clone()]);
        let restored_job = restored.job(&job.id).unwrap();
        assert_eq!(restored_job.authority, job.authority);
        assert_eq!(restored_job.output_refs, vec![output]);
        assert_eq!(
            restored_job.owner,
            DurableResourceOwner::Workspace(session.workspace_id)
        );
        std::fs::remove_dir_all(directory).unwrap();
    }
}
