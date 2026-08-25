impl ConductorRuntime {
    fn record_execution_created(
        &mut self,
        execution: ExecutionSummary,
        mut payload: JournalExecutionPayload,
        restrictions: Option<&ExecutionAuthority>,
    ) -> Result<(), ConductorError> {
        let root_statement = if execution.parent_execution.is_none() {
            match &payload {
                JournalExecutionPayload::Invocation { input, .. } => Some(input.clone()),
                JournalExecutionPayload::Orchestration { input, .. } => Some(
                    serde_json::to_string(input).expect("orchestration input is JSON serializable"),
                ),
            }
        } else {
            None
        };
        let parent_execution = execution.parent_execution.clone();

        self.ensure_objective_semantics_active()?;
        let configured = self.effective_authority_for_execution(&execution)?;
        let effective = restrictions.map_or(configured.clone(), |requested| {
            configured.attenuate(requested)
        });
        let (secret_names, secret_values) = secret_material(&effective);
        redact_execution_payload(&mut payload, &secret_names, &secret_values);
        payload.set_authority(effective);
        self.record_domain_event(DomainEvent::ExecutionCreated {
            execution: execution.clone(),
            payload,
        })?;

        if let Some(parent_execution) = parent_execution {
            let parent = self
                .execution_objectives(&parent_execution)?
                .ok_or_else(|| {
                    ObjectiveError::MissingExecutionObjective(parent_execution.clone())
                })?;
            self.assign_execution_objectives(&execution.id, parent.primary, parent.supporting)?;
        } else {
            let objective = self.create_root_objective_from_user_intent(
                root_statement.expect("root invocation has user intent"),
                Vec::new(),
                None,
            )?;
            self.assign_execution_objectives(&execution.id, objective.id, BTreeSet::new())?;
        }
        Ok(())
    }

    pub fn context_descriptors_for_execution(
        &self,
        execution_id: &ExecutionId,
    ) -> Result<Vec<ContextDescriptor>, ConductorError> {
        let catalog = self
            .configuration_for_execution(execution_id)?
            .context_catalog();
        let mut descriptors = catalog.descriptors().cloned().collect::<Vec<_>>();

        if let Some(assignment) = self.execution_objectives(execution_id)? {
            let objective = self.objective(&assignment.primary)?;
            let encoded =
                serde_json::to_vec(&objective).expect("objective record is JSON serializable");
            let revision = ContextRevision::parse(format!(
                "sha256:{}",
                Sha256::digest(&encoded)
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            ))
            .expect("generated objective context revision");
            descriptors.push(ContextDescriptor {
                id: ContextResourceId::parse(format!("objective:{}", objective.id))
                    .expect("generated objective context resource id"),
                kind: ContextResourceKind::Objective,
                title: objective.statement.clone(),
                description: format!("Durable objective {}", objective.id),
                scope: ContextScope::Workspace {
                    workspace_id: objective.workspace.clone(),
                },
                revision,
                estimated_cost: encoded.len() as u64,
            });
        }

        if let Some(assignment) = self.execution_plan(execution_id)? {
            let plan = self.plan(&assignment.plan_id)?;
            let encoded = serde_json::to_vec(&plan).expect("plan record is JSON serializable");
            let revision = ContextRevision::parse(format!(
                "sha256:{}",
                Sha256::digest(&encoded)
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            ))
            .expect("generated plan context revision");
            descriptors.push(ContextDescriptor {
                id: ContextResourceId::parse(format!("plan:{}", plan.id))
                    .expect("generated plan context resource id"),
                kind: ContextResourceKind::Plan,
                title: format!("Plan {}", plan.id),
                description: format!("Durable plan {}", plan.id),
                scope: ContextScope::Workspace {
                    workspace_id: plan.workspace.clone(),
                },
                revision,
                estimated_cost: encoded.len() as u64,
            });
        }

        descriptors.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(descriptors)
    }

    pub(crate) fn has_skills_for_execution(
        &self,
        execution_id: &ExecutionId,
    ) -> Result<bool, ConductorError> {
        Ok(self.configuration_for_execution(execution_id)?.has_skills())
    }

    pub fn execution_worker_profile(
        &self,
        execution_id: &ExecutionId,
    ) -> Result<Option<WorkerProfileId>, ConductorError> {
        self.executions
            .get(execution_id)
            .map(|execution| execution.worker_profile.clone())
            .ok_or_else(|| ConductorError::UnknownExecution(execution_id.clone()))
    }

    pub fn execution_authority(
        &self,
        execution_id: &ExecutionId,
    ) -> Result<ExecutionAuthority, ConductorError> {
        self.executions
            .get(execution_id)
            .map(|execution| execution.authority.clone())
            .ok_or_else(|| ConductorError::UnknownExecution(execution_id.clone()))
    }

    fn effective_authority_for_execution(
        &self,
        execution: &ExecutionSummary,
    ) -> Result<ExecutionAuthority, ConductorError> {
        let configured = self.configured_authority_for_execution(execution)?;
        let Some(parent_id) = execution.parent_execution.as_ref() else {
            return Ok(configured);
        };
        let parent = self
            .executions
            .get(parent_id)
            .ok_or_else(|| ConductorError::UnknownExecution(parent_id.clone()))?;
        Ok(parent.authority.attenuate(&configured))
    }

    pub fn submit(
        &mut self,
        session_id: &SessionId,
        text: impl Into<String>,
    ) -> Result<ExecutionSummary, ConductorError> {
        self.submit_with_restrictions(session_id, text, None)
    }

    pub fn submit_with_restrictions(
        &mut self,
        session_id: &SessionId,
        text: impl Into<String>,
        restrictions: Option<&ExecutionAuthority>,
    ) -> Result<ExecutionSummary, ConductorError> {
        let text = text.into();
        if text.trim().is_empty() {
            return Err(ConductorError::EmptyInput);
        }
        self.ensure_session_active(session_id)?;
        let target = self
            .sessions
            .get(session_id)
            .expect("active session exists")
            .summary
            .default_target
            .clone();
        let summary = ExecutionSummary {
            id: self.new_execution_id(),
            session_id: session_id.clone(),
            parent_execution: None,
            kind: ExecutionKind::Root,
            callable: None,
            target,
            state: ExecutionState::Pending,
        };
        self.record_execution_created(
            summary.clone(),
            JournalExecutionPayload::Invocation {
                input: text.clone(),
                authority: ExecutionAuthority::read_only(),
            },
            restrictions,
        )?;
        self.accept_root_submission(&summary)?;
        self.push_event(&summary.id, ExecutionEventKind::UserInput { text })?;
        self.push_event(
            &summary.id,
            ExecutionEventKind::ExecutionStateChanged {
                state: ExecutionState::Pending,
            },
        )?;
        Ok(summary)
    }

    pub(crate) fn accept_root_submission(
        &mut self,
        execution: &ExecutionSummary,
    ) -> Result<(), ConductorError> {
        let ingress_order = self
            .next_root_ingress
            .get(&execution.session_id)
            .copied()
            .unwrap_or(0)
            .saturating_add(1);
        self.record_domain_event(DomainEvent::RootSubmissionAccepted {
            session_id: execution.session_id.clone(),
            execution_id: execution.id.clone(),
            ingress_order,
        })
    }

    #[must_use]
    pub fn root_ingress_order(&self, execution_id: &ExecutionId) -> Option<u64> {
        self.root_ingress.get(execution_id).copied()
    }

    #[must_use]
    pub fn pending_roots_in_ingress_order(&self) -> Vec<ExecutionSummary> {
        let mut roots = self
            .executions
            .values()
            .filter(|execution| {
                execution.summary.parent_execution.is_none()
                    && execution.summary.state == ExecutionState::Pending
            })
            .map(|execution| execution.summary.clone())
            .collect::<Vec<_>>();
        roots.sort_by(|left, right| {
            left.session_id.cmp(&right.session_id).then_with(|| {
                self.root_ingress_order(&left.id)
                    .cmp(&self.root_ingress_order(&right.id))
            })
        });
        roots
    }

    fn create_child(
        &mut self,
        parent_id: &ExecutionId,
        kind: ExecutionKind,
        callable: CallableId,
        payload: ExecutionPayload,
        restrictions: Option<&ExecutionAuthority>,
    ) -> Result<ExecutionSummary, ConductorError> {
        let parent = self
            .executions
            .get(parent_id)
            .ok_or_else(|| ConductorError::UnknownExecution(parent_id.clone()))?;
        if !parent.authority.callables.contains(&callable) {
            return Err(ConductorError::DelegationDenied {
                parent_execution: parent_id.clone(),
                callable,
            });
        }
        let parent = parent.summary.clone();
        self.ensure_session_active(&parent.session_id)?;
        let child = ExecutionSummary {
            id: self.new_execution_id(),
            session_id: parent.session_id,
            parent_execution: Some(parent.id.clone()),
            kind,
            callable: Some(callable),
            target: parent.target,
            state: ExecutionState::Pending,
        };
        self.record_execution_created(
            child.clone(),
            JournalExecutionPayload::from(&payload),
            restrictions,
        )?;
        self.push_event(
            parent_id,
            ExecutionEventKind::ChildExecutionStarted {
                child: child.id.clone(),
            },
        )?;
        self.push_event(
            &child.id,
            ExecutionEventKind::ExecutionStateChanged {
                state: ExecutionState::Pending,
            },
        )?;
        Ok(child)
    }

    pub fn drive_execution(
        &mut self,
        execution_id: &ExecutionId,
        backend: &mut dyn Backend,
    ) -> Result<(), ConductorError> {
        let resolved = self.resolve_invocation(execution_id)?;
        let capabilities = backend.capabilities();
        let prepared = self.prepare_invocation(resolved, &capabilities)?;
        let allowed_tools = prepared.allowed_tools();
        let backend_session = backend.open_session(prepared.backend_session_request())?;
        self.set_state(execution_id, ExecutionState::Running)?;
        let request = prepared.backend_execution_request();
        let result = {
            let mut host = RuntimeHost {
                runtime: self,
                execution_id: execution_id.clone(),
                allowed_tools,
            };
            backend_session.execute(request, &mut host)
        };
        if let Err(error) = result {
            self.set_state(execution_id, ExecutionState::Failed)?;
            return Err(ConductorError::Backend(error));
        }
        if self
            .executions
            .get(execution_id)
            .is_some_and(|execution| execution.summary.state == ExecutionState::Running)
        {
            self.set_state(execution_id, ExecutionState::Completed)?;
        }
        Ok(())
    }

    fn execution_subtree(
        &self,
        root: &ExecutionId,
    ) -> Result<BTreeSet<ExecutionId>, ConductorError> {
        if !self.executions.contains_key(root) {
            return Err(ConductorError::UnknownExecution(root.clone()));
        }
        let mut subtree = BTreeSet::from([root.clone()]);
        loop {
            let before = subtree.len();
            for (id, record) in &self.executions {
                if record
                    .summary
                    .parent_execution
                    .as_ref()
                    .is_some_and(|parent| subtree.contains(parent))
                {
                    subtree.insert(id.clone());
                }
            }
            if subtree.len() == before {
                break;
            }
        }
        Ok(subtree)
    }

    pub fn record_execution_output(
        &mut self,
        execution_id: &ExecutionId,
        mut output: Value,
    ) -> Result<(), ConductorError> {
        let execution = self
            .executions
            .get(execution_id)
            .ok_or_else(|| ConductorError::UnknownExecution(execution_id.clone()))?;
        if self.execution_outputs.contains_key(execution_id) {
            return Err(ConductorError::InvalidExecutionData {
                execution_id: execution_id.clone(),
                message: "output was already recorded".to_owned(),
            });
        }
        let (secret_names, secret_values) = secret_material(&execution.authority);
        redact_value(&mut output, &secret_names, &secret_values);
        if let Some(callable) = execution.summary.callable.as_ref() {
            let descriptor = self
                .configuration_for_execution(execution_id)?
                .callables
                .descriptor(callable)?;
            validate_json_schema(&descriptor.output_schema, &output).map_err(|message| {
                ConductorError::InvalidExecutionData {
                    execution_id: execution_id.clone(),
                    message: format!("output: {message}"),
                }
            })?;
        }
        self.record_domain_event(DomainEvent::ExecutionOutputRecorded {
            execution_id: execution_id.clone(),
            output,
        })
    }

    #[must_use]
    pub fn execution_output(&self, execution_id: &ExecutionId) -> Option<&Value> {
        self.execution_outputs.get(execution_id)
    }

    fn new_execution_id(&self) -> ExecutionId {
        ExecutionId::parse(format!("execution-{}", self.next_execution + 1)).expect("generated id")
    }
}
