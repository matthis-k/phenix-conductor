impl ConductorRuntime {
    fn dispatch_lifecycle_hooks(
        &mut self,
        execution_id: &ExecutionId,
        event: LifecycleEvent,
    ) -> Result<(), ConductorError> {
        let root_dispatch = self.active_lifecycle_hooks.is_empty();
        let result = self.dispatch_lifecycle_hooks_inner(execution_id, event);
        if root_dispatch {
            self.active_lifecycle_hooks.clear();
        }
        result
    }

    fn dispatch_lifecycle_hooks_inner(
        &mut self,
        execution_id: &ExecutionId,
        event: LifecycleEvent,
    ) -> Result<(), ConductorError> {
        let hooks = self
            .configuration_for_execution(execution_id)?
            .lifecycle_hooks_for_event(&event)?
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();

        for hook in hooks {
            if self
                .active_lifecycle_hooks
                .iter()
                .any(|(_, active)| active == &hook.id)
            {
                continue;
            }
            let guard = (execution_id.clone(), hook.id.clone());
            self.active_lifecycle_hooks.insert(guard);
            let result = self.execute_lifecycle_hook_action(execution_id, &event, &hook);

            if let Err(error) = result {
                match hook.failure_policy {
                    HookFailurePolicy::Ignore => {}
                    HookFailurePolicy::Warn => {
                        self.push_event(
                            execution_id,
                            ExecutionEventKind::Error {
                                code: "lifecycle_hook_warning".to_owned(),
                                message: error.to_string(),
                            },
                        )?;
                    }
                    HookFailurePolicy::FailOperation => return Err(error.into()),
                }
            }
        }
        Ok(())
    }
    fn execute_lifecycle_hook_action(
        &mut self,
        execution_id: &ExecutionId,
        event: &LifecycleEvent,
        hook: &LifecycleHookDefinition,
    ) -> Result<(), LifecycleHookError> {
        let failure = |message: String| LifecycleHookError::ActionFailed {
            hook: hook.id.clone(),
            event: event.clone(),
            message,
        };
        match &hook.action {
            HookAction::Observe => Ok(()),
            HookAction::RequestContext { resource_id } => {
                let descriptor = self
                    .context_descriptors_for_execution(execution_id)
                    .map_err(|error| failure(error.to_string()))?
                    .into_iter()
                    .find(|descriptor| &descriptor.id == resource_id)
                    .ok_or_else(|| {
                        failure(format!("unknown context resource for hook: {resource_id}"))
                    })?;
                self.load_context_for_execution(
                    execution_id,
                    resource_id,
                    &descriptor.revision,
                    phenix_core::ContextInjectionRequester::Hook,
                    phenix_core::ContextInjectionLifetime::SingleRequest,
                    format!("lifecycle hook {} for {event:?}", hook.id.0),
                )
                .map(|_| ())
                .map_err(|error| failure(error.to_string()))
            }
            HookAction::InvokeCallable { callable_id } => {
                let descriptor = self
                    .configuration_for_execution(execution_id)
                    .and_then(|configuration| {
                        configuration
                            .callables
                            .descriptor(callable_id)
                            .cloned()
                            .map_err(Into::into)
                    })
                    .map_err(|error| failure(error.to_string()))?;
                match descriptor.kind {
                    CallableKind::Tool => {
                        let allowed_tools = self
                            .permitted_tool_descriptors(execution_id)
                            .map_err(|error| failure(error.to_string()))?
                            .into_iter()
                            .map(|descriptor| descriptor.id)
                            .collect();
                        let result = self
                            .invoke_tool(
                                execution_id,
                                &allowed_tools,
                                ToolInvocation {
                                    callable: callable_id.clone(),
                                    arguments_json: "{}".to_owned(),
                                },
                            )
                            .map_err(|error| failure(error.to_string()))?;
                        if result.success {
                            Ok(())
                        } else {
                            Err(failure(result.output))
                        }
                    }
                    CallableKind::Agent => self
                        .start_agent(
                            execution_id,
                            callable_id,
                            format!("Lifecycle hook {} for {event:?}", hook.id.0),
                        )
                        .map(|_| ())
                        .map_err(|error| failure(error.to_string())),
                    CallableKind::Orchestration => self
                        .start_orchestration(
                            execution_id,
                            callable_id,
                            json!({
                                "lifecycle_hook": hook.id.0,
                                "event": format!("{event:?}"),
                            }),
                        )
                        .map(|_| ())
                        .map_err(|error| failure(error.to_string())),
                }
            }
            HookAction::InvokeOrchestration { callable_id } => self
                .start_orchestration(
                    execution_id,
                    callable_id,
                    json!({
                        "lifecycle_hook": hook.id.0,
                        "event": format!("{event:?}"),
                    }),
                )
                .map(|_| ())
                .map_err(|error| failure(error.to_string())),
            HookAction::Veto => Err(LifecycleHookError::Vetoed {
                hook: hook.id.clone(),
                event: event.clone(),
            }),
            HookAction::EmitMetadata { key, value } => self
                .push_event(
                    execution_id,
                    ExecutionEventKind::LifecycleHookMetadata {
                        hook_id: hook.id.0.clone(),
                        key: key.clone(),
                        value: value.clone(),
                    },
                )
                .map(|_| ())
                .map_err(|error| failure(error.to_string())),
        }
    }
}

#[cfg(test)]
mod lifecycle_hook_runtime_tests {
    use super::*;
    use phenix_core::{
        BackendId, CallablePolicy, CapabilitySet, InferenceOptions, ModelId, ProviderId,
    };

    fn target() -> ExecutionTarget {
        ExecutionTarget::Fixed(ModelTarget {
            backend: BackendId::parse("mock").unwrap(),
            provider: ProviderId::parse("mock").unwrap(),
            model: ModelId::parse("test").unwrap(),
            inference: InferenceOptions::default(),
        })
    }

    fn hook(
        id: &str,
        event: LifecycleEvent,
        action: HookAction,
        failure_policy: HookFailurePolicy,
    ) -> LifecycleHookDefinition {
        LifecycleHookDefinition {
            id: LifecycleHookId(id.to_owned()),
            event,
            after: BTreeSet::new(),
            action,
            failure_policy,
        }
    }

    fn agent_definition(id: &str) -> AgentDefinition {
        AgentDefinition {
            descriptor: CallableDescriptor {
                id: CallableId::parse(id).unwrap(),
                kind: CallableKind::Agent,
                description: "hook test agent".to_owned(),
                input_schema: json!({"type": "object"}),
                output_schema: json!({"type": "object"}),
                capabilities: CapabilitySet::default(),
                policy: CallablePolicy::default(),
            },
            authority: ExecutionAuthority::read_only(),
        }
    }

    #[test]
    fn execution_uses_hook_definition_from_its_pinned_configuration() {
        let mut first = CompiledConfiguration::default();
        first
            .register_lifecycle_hook(hook(
                "record",
                LifecycleEvent::ExecutionCompleted,
                HookAction::EmitMetadata {
                    key: "version".to_owned(),
                    value: json!("first"),
                },
                HookFailurePolicy::FailOperation,
            ))
            .unwrap();
        let mut runtime = ConductorRuntime::new();
        let first_revision = runtime.reload_configuration(first).unwrap();
        let session = runtime.create_session(None, None, target()).unwrap();
        let execution = runtime.submit(&session.id, "pinned").unwrap();

        let mut second = CompiledConfiguration::default();
        second
            .register_lifecycle_hook(hook(
                "record",
                LifecycleEvent::ExecutionCompleted,
                HookAction::EmitMetadata {
                    key: "version".to_owned(),
                    value: json!("second"),
                },
                HookFailurePolicy::FailOperation,
            ))
            .unwrap();
        runtime.reload_configuration(second).unwrap();

        runtime
            .set_state(&execution.id, ExecutionState::Completed)
            .unwrap();
        assert_eq!(
            runtime.execution_config_revision(&execution.id).unwrap(),
            first_revision
        );
        assert!(runtime.events.iter().any(|event| matches!(
            &event.kind,
            ExecutionEventKind::LifecycleHookMetadata { key, value, .. }
                if key == "version" && value == &json!("first")
        )));
        assert!(!runtime.events.iter().any(|event| matches!(
            &event.kind,
            ExecutionEventKind::LifecycleHookMetadata { value, .. }
                if value == &json!("second")
        )));
    }

    #[test]
    fn same_hook_recursive_reentry_is_blocked_across_child_creation() {
        let callable = CallableId::parse("worker").unwrap();
        let mut configuration = CompiledConfiguration::default();
        configuration.register_agent(agent_definition("worker")).unwrap();
        configuration
            .register_lifecycle_hook(hook(
                "spawn-on-create",
                LifecycleEvent::ExecutionCreated,
                HookAction::InvokeCallable {
                    callable_id: callable,
                },
                HookFailurePolicy::FailOperation,
            ))
            .unwrap();

        let mut runtime = ConductorRuntime::new();
        runtime.reload_configuration(configuration).unwrap();
        let session = runtime.create_session(None, None, target()).unwrap();
        runtime.submit(&session.id, "root").unwrap();

        assert_eq!(runtime.executions.len(), 2);
        assert_eq!(
            runtime
                .executions
                .values()
                .filter(|execution| execution.summary.parent_execution.is_some())
                .count(),
            1
        );
    }

    #[test]
    fn fail_operation_veto_prevents_supported_state_transition() {
        let mut configuration = CompiledConfiguration::default();
        configuration
            .register_lifecycle_hook(hook(
                "veto-completion",
                LifecycleEvent::ExecutionCompleted,
                HookAction::Veto,
                HookFailurePolicy::FailOperation,
            ))
            .unwrap();
        let mut runtime = ConductorRuntime::new();
        runtime.reload_configuration(configuration).unwrap();
        let session = runtime.create_session(None, None, target()).unwrap();
        let execution = runtime.submit(&session.id, "root").unwrap();

        assert!(matches!(
            runtime.set_state(&execution.id, ExecutionState::Completed),
            Err(ConductorError::LifecycleHook(LifecycleHookError::Vetoed { .. }))
        ));
        assert_eq!(
            runtime.executions[&execution.id].summary.state,
            ExecutionState::Pending
        );
    }

    #[test]
    fn ignore_and_warn_policies_do_not_fail_the_operation() {
        for (policy, expect_warning) in [
            (HookFailurePolicy::Ignore, false),
            (HookFailurePolicy::Warn, true),
        ] {
            let mut configuration = CompiledConfiguration::default();
            configuration
                .register_lifecycle_hook(hook(
                    "veto-completion",
                    LifecycleEvent::ExecutionCompleted,
                    HookAction::Veto,
                    policy,
                ))
                .unwrap();
            let mut runtime = ConductorRuntime::new();
            runtime.reload_configuration(configuration).unwrap();
            let session = runtime.create_session(None, None, target()).unwrap();
            let execution = runtime.submit(&session.id, "root").unwrap();

            runtime
                .set_state(&execution.id, ExecutionState::Completed)
                .unwrap();
            assert_eq!(
                runtime.executions[&execution.id].summary.state,
                ExecutionState::Completed
            );
            assert_eq!(
                runtime.events.iter().any(|event| matches!(
                    &event.kind,
                    ExecutionEventKind::Error { code, .. } if code == "lifecycle_hook_warning"
                )),
                expect_warning
            );
        }
    }

    #[test]
    fn hook_callable_cannot_bypass_execution_authority() {
        let callable = CallableId::parse("worker").unwrap();
        let mut configuration = CompiledConfiguration::default();
        configuration.register_agent(agent_definition("worker")).unwrap();
        configuration
            .register_lifecycle_hook(hook(
                "spawn-on-create",
                LifecycleEvent::ExecutionCreated,
                HookAction::InvokeCallable {
                    callable_id: callable,
                },
                HookFailurePolicy::FailOperation,
            ))
            .unwrap();

        let mut runtime = ConductorRuntime::new();
        runtime.reload_configuration(configuration).unwrap();
        let session = runtime.create_session(None, None, target()).unwrap();
        let restricted = ExecutionAuthority::read_only();
        assert!(matches!(
            runtime.submit_with_restrictions(&session.id, "root", Some(&restricted)),
            Err(ConductorError::LifecycleHook(LifecycleHookError::ActionFailed { .. }))
        ));
        let root = ExecutionId::parse("execution-1").unwrap();
        assert_eq!(runtime.executions.len(), 1);
        assert_eq!(runtime.executions[&root].summary.state, ExecutionState::Failed);
    }
}
