impl ConductorRuntime {
    pub(crate) fn failed_child_for_interface(
        &self,
        interface_execution: &ExecutionId,
    ) -> Option<ExecutionId> {
        self.orchestration_interfaces
            .iter()
            .find_map(|(failed_child, interface)| {
                (interface == interface_execution).then(|| failed_child.clone())
            })
    }

    fn cancel_execution_set(
        &mut self,
        executions: BTreeSet<ExecutionId>,
        cause: ExecutionTerminationCause,
    ) -> Result<(), ConductorError> {
        for id in executions {
            let state = self
                .executions
                .get(&id)
                .expect("collected execution")
                .summary
                .state
                .clone();
            if !is_terminal(&state) {
                self.push_event(
                    &id,
                    ExecutionEventKind::ExecutionTerminated {
                        cause: cause.clone(),
                    },
                )?;
                self.set_state(&id, ExecutionState::Cancelled)?;
            }
        }
        Ok(())
    }

    fn cancel_descendants(&mut self, root: &ExecutionId) -> Result<(), ConductorError> {
        let mut descendants = self.execution_subtree(root)?;
        descendants.remove(root);
        self.cancel_execution_set(
            descendants,
            ExecutionTerminationCause::AncestorFailure {
                failed_ancestor: root.clone(),
            },
        )
    }

    pub fn cancel_execution(&mut self, root: &ExecutionId) -> Result<(), ConductorError> {
        let executions = self.execution_subtree(root)?;
        self.cancel_execution_set(
            executions,
            ExecutionTerminationCause::ExplicitCancellation {
                requested_execution: root.clone(),
            },
        )
    }

    pub fn set_state(
        &mut self,
        execution_id: &ExecutionId,
        state: ExecutionState,
    ) -> Result<(), ConductorError> {
        let (current, parent, has_callable) = {
            let execution = self
                .executions
                .get(execution_id)
                .ok_or_else(|| ConductorError::UnknownExecution(execution_id.clone()))?;
            (
                execution.summary.state.clone(),
                execution.summary.parent_execution.clone(),
                execution.summary.callable.is_some(),
            )
        };
        if is_terminal(&current) {
            return Err(ConductorError::InvalidLifecycle(execution_id.clone()));
        }

        if state == ExecutionState::Running && has_callable {
            self.dispatch_lifecycle_hooks(execution_id, LifecycleEvent::CallableStarted)?;
        }
        if state == ExecutionState::Completed {
            self.ensure_orchestration_child_output(execution_id)?;
            if has_callable {
                self.dispatch_lifecycle_hooks(execution_id, LifecycleEvent::CallableCompleted)?;
            }
            self.dispatch_lifecycle_hooks(execution_id, LifecycleEvent::ExecutionCompleted)?;
        } else if state == ExecutionState::Failed {
            self.dispatch_lifecycle_hooks(execution_id, LifecycleEvent::ExecutionFailed)?;
        }

        self.record_domain_event(DomainEvent::ExecutionStateChanged {
            execution_id: execution_id.clone(),
            state: state.clone(),
        })?;
        self.push_event(
            execution_id,
            ExecutionEventKind::ExecutionStateChanged {
                state: state.clone(),
            },
        )?;
        if state == ExecutionState::Failed {
            self.cancel_descendants(execution_id)?;
        }
        if is_terminal(&state) {
            self.skill_activations.remove(execution_id);
            self.sandbox_states.remove(execution_id);
            if let Some(parent) = parent {
                self.push_event(
                    &parent,
                    ExecutionEventKind::ChildExecutionFinished {
                        child: execution_id.clone(),
                        state,
                    },
                )?;
                self.refresh_orchestration(&parent)?;
            }
        }
        Ok(())
    }

    fn execution_sandbox_state(
        &mut self,
        execution_id: &ExecutionId,
    ) -> Result<std::sync::Arc<sandbox::ExecutionSandboxState>, std::io::Error> {
        if let Some(state) = self.sandbox_states.get(execution_id) {
            return Ok(std::sync::Arc::clone(state));
        }
        let state = sandbox::ExecutionSandboxState::create()?;
        self.sandbox_states
            .insert(execution_id.clone(), std::sync::Arc::clone(&state));
        Ok(state)
    }
}
