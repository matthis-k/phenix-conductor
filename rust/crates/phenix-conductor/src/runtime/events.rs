impl ConductorRuntime {
    fn record_domain_event(&mut self, mut event: DomainEvent) -> Result<(), ConductorError> {
        redact_domain_event(&mut event, &self.executions, &self.attempt_groups);
        let frontend_event = match &event {
            DomainEvent::FrontendEvent { event } => Some(event.clone()),
            _ => None,
        };
        let sequence = u64::try_from(self.journal.entries.len())
            .map_err(|_| JournalError::InvalidFormat("journal is too large".to_owned()))?
            + 1;
        self.journal.entries.push(JournalEntry {
            sequence,
            event: event.clone(),
        });
        let result = {
            let mut projection = DurableProjection {
                config_revisions: &mut self.config_revisions,
                current_config_revision: &mut self.config_revision,
                sessions: &mut self.sessions,
                executions: &mut self.executions,
                root_ingress: &mut self.root_ingress,
                next_root_ingress: &mut self.next_root_ingress,
                attempt_groups: &mut self.attempt_groups,
                orchestration_decisions: &mut self.orchestration_decisions,
                orchestration_interfaces: &mut self.orchestration_interfaces,
                orchestration_nodes: &mut self.orchestration_nodes,
                orchestration_node_inputs: &mut self.orchestration_node_inputs,
                orchestration_synthesis: &mut self.orchestration_synthesis,
                execution_outputs: &mut self.execution_outputs,
                diagnostic_write_patches: &mut self.diagnostic_write_patches,
                resolved_routes: &mut self.resolved_routes,
                read_sets: &mut self.read_sets,
                events: &mut self.events,
                next_config_revision: &mut self.next_config_revision,
                next_session: &mut self.next_session,
                next_execution: &mut self.next_execution,
                next_attempt_group: &mut self.next_attempt_group,
                next_event: &mut self.next_event,
                next_tool_call: &mut self.next_tool_call,
            };
            apply_domain_event(&mut projection, &event)
        };
        if let Err(error) = result {
            self.journal.entries.pop();
            return Err(error.into());
        }
        if let Some(event) = frontend_event {
            self.event_sinks
                .retain(|_, sink| sink.send(event.clone()).is_ok());
        }
        Ok(())
    }

    pub fn push_event(
        &mut self,
        execution_id: &ExecutionId,
        kind: ExecutionEventKind,
    ) -> Result<ExecutionEvent, ConductorError> {
        let session_id = self
            .executions
            .get(execution_id)
            .ok_or_else(|| ConductorError::UnknownExecution(execution_id.clone()))?
            .summary
            .session_id
            .clone();
        let mut event = ExecutionEvent {
            sequence: self.next_event + 1,
            session_id,
            execution_id: execution_id.clone(),
            kind,
        };
        let authority = &self
            .executions
            .get(execution_id)
            .expect("event execution was resolved above")
            .authority;
        let (secret_names, secret_values) = secret_material(authority);
        redact_event(&mut event, &secret_names, &secret_values);
        self.record_domain_event(DomainEvent::FrontendEvent {
            event: event.clone(),
        })?;
        Ok(event)
    }

    pub fn subscribe_events(
        &mut self,
        capacity: usize,
    ) -> std::sync::mpsc::Receiver<ExecutionEvent> {
        self.subscribe_events_with_id(capacity).1
    }

    pub fn subscribe_events_with_id(
        &mut self,
        _capacity: usize,
    ) -> (u64, std::sync::mpsc::Receiver<ExecutionEvent>) {
        let (sender, receiver) = std::sync::mpsc::channel();
        self.next_event_subscription = self.next_event_subscription.saturating_add(1);
        let subscription = self.next_event_subscription;
        self.event_sinks.insert(subscription, sender);
        (subscription, receiver)
    }

    pub fn unsubscribe_event_subscription(&mut self, subscription: u64) {
        self.event_sinks.remove(&subscription);
    }

    pub fn unsubscribe_events(&mut self) {
        self.event_sinks.clear();
    }

    #[must_use]
    pub fn event_subscription_count(&self) -> usize {
        self.event_sinks.len()
    }

    #[must_use]
    pub fn events_since(&self, sequence: u64) -> Vec<ExecutionEvent> {
        self.events
            .iter()
            .filter(|event| event.sequence > sequence)
            .cloned()
            .collect()
    }
}
