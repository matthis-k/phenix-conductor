impl ConductorRuntime {
    pub fn create_session(
        &mut self,
        parent_session: Option<SessionId>,
        name: Option<String>,
        target: ExecutionTarget,
    ) -> Result<SessionSummary, ConductorError> {
        let revision = self.config_revision.clone();
        self.create_session_at_revision(parent_session, name, target, revision)
    }

    pub fn fork_session(
        &mut self,
        source: &SessionId,
        name: Option<String>,
    ) -> Result<SessionSummary, ConductorError> {
        let source = self
            .sessions
            .get(source)
            .ok_or_else(|| ConductorError::UnknownSession(source.clone()))?
            .summary
            .clone();
        self.create_session_at_revision(
            Some(source.id),
            name,
            source.default_target,
            source.config_revision,
        )
    }

    pub fn rebase_session(
        &mut self,
        session_id: &SessionId,
        revision: &ConfigRevisionId,
    ) -> Result<SessionSummary, ConductorError> {
        self.ensure_session_active(session_id)?;
        let session = self
            .sessions
            .get(session_id)
            .expect("active session exists")
            .summary
            .clone();
        if session.config_revision == *revision {
            return Ok(session);
        }
        let configuration = self.configuration_revision(revision)?;
        let current_ordinal = self.config_revisions[&session.config_revision].ordinal;
        let target_ordinal = self.config_revisions[revision].ordinal;
        if target_ordinal <= current_ordinal {
            return Err(ConductorError::IncompatibleSessionRebase {
                session_id: session_id.clone(),
                revision: revision.clone(),
                reason: format!(
                    "target is not newer than current revision {}",
                    session.config_revision
                ),
            });
        }
        if let ExecutionTarget::Routed(profile) = &session.default_target {
            if !configuration.routing.contains(profile) {
                return Err(ConductorError::IncompatibleSessionRebase {
                    session_id: session_id.clone(),
                    revision: revision.clone(),
                    reason: format!("routing profile {profile} is unavailable"),
                });
            }
        }
        self.record_domain_event(DomainEvent::SessionConfigRebased {
            session_id: session_id.clone(),
            config_revision: revision.clone(),
        })?;
        Ok(self
            .sessions
            .get(session_id)
            .expect("rebased session remains present")
            .summary
            .clone())
    }

    pub fn validate_session_close(
        &self,
        session_id: &SessionId,
    ) -> Result<SessionSummary, ConductorError> {
        let session = self
            .sessions
            .get(session_id)
            .ok_or_else(|| ConductorError::UnknownSession(session_id.clone()))?
            .summary
            .clone();
        if session.state == SessionState::Closed {
            return Ok(session);
        }
        if self.executions.values().any(|execution| {
            execution.summary.session_id == *session_id && !is_terminal(&execution.summary.state)
        }) {
            return Err(ConductorError::SessionHasActiveExecutions(
                session_id.clone(),
            ));
        }
        Ok(session)
    }

    pub fn close_session(
        &mut self,
        session_id: &SessionId,
    ) -> Result<SessionSummary, ConductorError> {
        let session = self.validate_session_close(session_id)?;
        if session.state == SessionState::Closed {
            return Ok(session);
        }
        self.record_domain_event(DomainEvent::SessionClosed {
            session_id: session_id.clone(),
        })?;
        Ok(self
            .sessions
            .get(session_id)
            .expect("closed session remains present")
            .summary
            .clone())
    }

    pub fn session(&self, session_id: &SessionId) -> Result<SessionSummary, ConductorError> {
        self.sessions
            .get(session_id)
            .map(|record| record.summary.clone())
            .ok_or_else(|| ConductorError::UnknownSession(session_id.clone()))
    }

    pub fn build_session_debug_bundle(
        &self,
        session_id: &SessionId,
        workspace: WorkspaceDescriptor,
        current_versions: &BTreeMap<PathBuf, FileVersion>,
    ) -> Result<SessionDebugBundle, ConductorError> {
        let session = self
            .sessions
            .get(session_id)
            .ok_or_else(|| ConductorError::UnknownSession(session_id.clone()))?
            .summary
            .clone();
        if workspace.id != session.workspace_id {
            return Err(ConductorError::WorkspaceMismatch {
                expected: session.workspace_id,
                actual: workspace.id,
            });
        }
        let execution_ids = self
            .executions
            .values()
            .filter(|record| record.summary.session_id == *session_id)
            .map(|record| record.summary.id.clone())
            .collect::<BTreeSet<_>>();
        let secret_names = self
            .executions
            .values()
            .filter(|record| execution_ids.contains(&record.summary.id))
            .flat_map(|record| record.authority.secrets.iter())
            .cloned()
            .collect::<BTreeSet<_>>();
        let secret_values = secret_names
            .iter()
            .filter_map(|name| std::env::var(name).ok())
            .filter(|value| !value.is_empty())
            .collect::<BTreeSet<_>>();
        let mut events = self
            .events
            .iter()
            .filter(|event| event.session_id == *session_id)
            .cloned()
            .collect::<Vec<_>>();
        for event in &mut events {
            redact_event(event, &secret_names, &secret_values);
        }

        let mut bundle = SessionDebugBundle::new(session, workspace);
        bundle.executions = self
            .executions
            .values()
            .filter(|record| execution_ids.contains(&record.summary.id))
            .map(|record| record.summary.clone())
            .collect();
        bundle.events = events.clone();
        bundle.attempt_groups = self
            .attempt_groups
            .values()
            .filter(|group| execution_ids.contains(&group.parent_execution))
            .cloned()
            .map(|mut group| {
                redact_attempt_group(&mut group, &secret_names, &secret_values);
                group
            })
            .collect();
        for event in &events {
            match &event.kind {
                ExecutionEventKind::UserInput { text } => {
                    bundle.conversation.push(DebugConversationMessage {
                        execution_id: event.execution_id.clone(),
                        role: DebugConversationRole::User,
                        content: text.clone(),
                    })
                }
                ExecutionEventKind::AssistantContentDelta { text } => {
                    if let Some(last) = bundle.conversation.last_mut().filter(|message| {
                        message.execution_id == event.execution_id
                            && message.role == DebugConversationRole::Assistant
                    }) {
                        last.content.push_str(text);
                    } else {
                        bundle.conversation.push(DebugConversationMessage {
                            execution_id: event.execution_id.clone(),
                            role: DebugConversationRole::Assistant,
                            content: text.clone(),
                        });
                    }
                }
                ExecutionEventKind::ToolCallStarted { .. }
                | ExecutionEventKind::ToolCallArguments { .. }
                | ExecutionEventKind::ToolCallFinished { .. } => {
                    bundle.tool_activity.push(event.clone());
                }
                ExecutionEventKind::ExecutionTerminated { cause } => {
                    bundle
                        .termination_causes
                        .insert(event.execution_id.clone(), cause.clone());
                }
                _ => {}
            }
        }
        for execution_id in &execution_ids {
            let record = &self.executions[execution_id];
            let mut authority = record.authority.clone();
            authority.secrets.clear();
            bundle
                .workspace_authority
                .insert(execution_id.clone(), authority);
            let read_set = self
                .read_sets
                .get(execution_id)
                .cloned()
                .unwrap_or_else(|| ExecutionReadSet::new(execution_id.clone()));
            bundle.workspace_validity.insert(
                execution_id.clone(),
                read_set.validity_against(current_versions),
            );
            bundle.read_sets.push(read_set);
            if record.summary.kind == ExecutionKind::Orchestration {
                let callable = record
                    .summary
                    .callable
                    .as_ref()
                    .expect("orchestration callable invariant");
                let definition = self
                    .configuration_revision(&record.config_revision)?
                    .callables
                    .orchestration(callable)?
                    .clone();
                let node_bindings = self
                    .orchestration_nodes
                    .iter()
                    .filter(|(child_id, _)| {
                        self.executions.get(*child_id).is_some_and(|child| {
                            child.summary.parent_execution.as_ref() == Some(execution_id)
                        })
                    })
                    .map(|(child_id, node_id)| (node_id.clone(), child_id.clone()))
                    .collect();
                let mut node_inputs = self
                    .orchestration_node_inputs
                    .iter()
                    .filter(|((parent_id, _), _)| parent_id == execution_id)
                    .map(|((_, node_id), input)| (node_id.clone(), input.clone()))
                    .collect::<BTreeMap<_, _>>();
                for value in node_inputs.values_mut() {
                    redact_value(value, &secret_names, &secret_values);
                }
                bundle.orchestrations.push(DebugOrchestration {
                    execution_id: execution_id.clone(),
                    definition,
                    node_bindings,
                    node_inputs,
                    synthesis_execution: self.orchestration_synthesis.get(execution_id).cloned(),
                });
            }
        }
        bundle.resolved_routing = self
            .resolved_routes
            .iter()
            .filter(|(execution_id, _)| execution_ids.contains(*execution_id))
            .map(|(execution_id, route)| DebugResolvedRoute {
                execution_id: execution_id.clone(),
                requested_target: route.requested_target.clone(),
                model: route.model.clone(),
                config_revision: route.config_revision.clone(),
            })
            .collect();
        bundle.failure_decisions = self
            .orchestration_decisions
            .values()
            .filter(|decision| execution_ids.contains(&decision.parent_execution))
            .cloned()
            .collect();
        bundle.execution_outputs = self
            .execution_outputs
            .iter()
            .filter(|(execution_id, _)| execution_ids.contains(*execution_id))
            .map(|(execution_id, output)| {
                let mut output = output.clone();
                redact_value(&mut output, &secret_names, &secret_values);
                (execution_id.clone(), output)
            })
            .collect();
        bundle.checkpoints = self
            .journal
            .entries
            .iter()
            .filter_map(|entry| match &entry.event {
                DomainEvent::WorkspaceCheckpointCaptured {
                    execution_id,
                    workspace_id,
                    files,
                } if execution_ids.contains(execution_id) => Some(DebugWorkspaceCheckpoint {
                    sequence: entry.sequence,
                    execution_id: execution_id.clone(),
                    workspace_id: workspace_id.clone(),
                    files: files.clone(),
                }),
                _ => None,
            })
            .collect();
        bundle.diagnostic_write_patches = self
            .diagnostic_write_patches
            .iter()
            .filter(|patch| execution_ids.contains(&patch.execution_id))
            .cloned()
            .map(|mut patch| {
                redact_text(&mut patch.patch, &secret_values);
                patch
            })
            .collect();
        Ok(bundle)
    }

    fn ensure_session_active(&self, session_id: &SessionId) -> Result<(), ConductorError> {
        let session = self
            .sessions
            .get(session_id)
            .ok_or_else(|| ConductorError::UnknownSession(session_id.clone()))?;
        if session.summary.state == SessionState::Closed {
            Err(ConductorError::ClosedSession(session_id.clone()))
        } else {
            Ok(())
        }
    }

    fn new_session_id(&self) -> SessionId {
        SessionId::parse(format!("session-{}", self.next_session + 1)).expect("generated id")
    }
}
