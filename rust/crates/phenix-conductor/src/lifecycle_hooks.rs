use phenix_core::{CallableId, ContextResourceId};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LifecycleHookId(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleEvent {
    ExecutionCreated,
    ExecutionCompleted,
    ExecutionFailed,
    ContextLoaded,
    CallableStarted,
    CallableCompleted,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookFailurePolicy {
    Ignore,
    Warn,
    FailOperation,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HookAction {
    Observe,
    RequestContext { resource_id: ContextResourceId },
    InvokeCallable { callable_id: CallableId },
    InvokeOrchestration { callable_id: CallableId },
    Veto,
    EmitMetadata { key: String, value: Value },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LifecycleHookDefinition {
    pub id: LifecycleHookId,
    pub event: LifecycleEvent,
    #[serde(default)]
    pub after: BTreeSet<LifecycleHookId>,
    pub action: HookAction,
    pub failure_policy: HookFailurePolicy,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LifecycleHookError {
    Duplicate(LifecycleHookId),
    UnknownDependency {
        hook: LifecycleHookId,
        dependency: LifecycleHookId,
    },
    CrossEventDependency {
        hook: LifecycleHookId,
        dependency: LifecycleHookId,
    },
    DependencyCycle {
        event: LifecycleEvent,
    },
    ActionFailed {
        hook: LifecycleHookId,
        event: LifecycleEvent,
        message: String,
    },
    Vetoed {
        hook: LifecycleHookId,
        event: LifecycleEvent,
    },
}

impl Display for LifecycleHookError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Duplicate(id) => write!(formatter, "duplicate lifecycle hook {}", id.0),
            Self::UnknownDependency { hook, dependency } => write!(
                formatter,
                "lifecycle hook {} depends on unknown hook {}",
                hook.0, dependency.0
            ),
            Self::CrossEventDependency { hook, dependency } => write!(
                formatter,
                "lifecycle hook {} depends on hook {} from another event",
                hook.0, dependency.0
            ),
            Self::DependencyCycle { event } => {
                write!(formatter, "lifecycle hook dependency cycle for {event:?}")
            }
            Self::ActionFailed {
                hook,
                event,
                message,
            } => write!(
                formatter,
                "lifecycle hook {} failed during {event:?}: {message}",
                hook.0
            ),
            Self::Vetoed { hook, event } => {
                write!(formatter, "lifecycle hook {} vetoed {event:?}", hook.0)
            }
        }
    }
}

impl Error for LifecycleHookError {}

#[derive(Clone, Debug, Default)]
pub(crate) struct LifecycleHookRegistry {
    hooks: BTreeMap<LifecycleHookId, LifecycleHookDefinition>,
}

impl LifecycleHookRegistry {
    pub fn register(&mut self, hook: LifecycleHookDefinition) -> Result<(), LifecycleHookError> {
        if self.hooks.contains_key(&hook.id) {
            return Err(LifecycleHookError::Duplicate(hook.id));
        }
        self.hooks.insert(hook.id.clone(), hook);
        Ok(())
    }

    pub fn ordered_for_event(
        &self,
        event: &LifecycleEvent,
    ) -> Result<Vec<&LifecycleHookDefinition>, LifecycleHookError> {
        self.validate()?;
        let mut remaining = self
            .hooks
            .values()
            .filter(|hook| &hook.event == event)
            .map(|hook| (hook.id.clone(), hook))
            .collect::<BTreeMap<_, _>>();
        let mut emitted = BTreeSet::new();
        let mut ordered = Vec::with_capacity(remaining.len());
        while !remaining.is_empty() {
            let ready = remaining
                .iter()
                .find(|(_, hook)| {
                    hook.after
                        .iter()
                        .all(|dependency| emitted.contains(dependency))
                })
                .map(|(id, _)| id.clone())
                .ok_or_else(|| LifecycleHookError::DependencyCycle {
                    event: event.clone(),
                })?;
            let hook = remaining
                .remove(&ready)
                .expect("ready hook must remain present");
            emitted.insert(ready);
            ordered.push(hook);
        }
        Ok(ordered)
    }

    pub fn semantic_manifest(&self) -> Value {
        let hooks = self.hooks.values().collect::<Vec<_>>();
        json!(hooks)
    }

    fn validate(&self) -> Result<(), LifecycleHookError> {
        for hook in self.hooks.values() {
            for dependency in &hook.after {
                let Some(target) = self.hooks.get(dependency) else {
                    return Err(LifecycleHookError::UnknownDependency {
                        hook: hook.id.clone(),
                        dependency: dependency.clone(),
                    });
                };
                if target.event != hook.event {
                    return Err(LifecycleHookError::CrossEventDependency {
                        hook: hook.id.clone(),
                        dependency: dependency.clone(),
                    });
                }
            }
        }
        for event in self
            .hooks
            .values()
            .map(|hook| hook.event.clone())
            .collect::<BTreeSet<_>>()
        {
            self.validate_event_dag(&event)?;
        }
        Ok(())
    }

    fn validate_event_dag(&self, event: &LifecycleEvent) -> Result<(), LifecycleHookError> {
        let mut remaining = self
            .hooks
            .values()
            .filter(|hook| &hook.event == event)
            .map(|hook| (hook.id.clone(), hook))
            .collect::<BTreeMap<_, _>>();
        let mut emitted = BTreeSet::new();
        while !remaining.is_empty() {
            let ready = remaining
                .iter()
                .find(|(_, hook)| {
                    hook.after
                        .iter()
                        .all(|dependency| emitted.contains(dependency))
                })
                .map(|(id, _)| id.clone())
                .ok_or_else(|| LifecycleHookError::DependencyCycle {
                    event: event.clone(),
                })?;
            remaining.remove(&ready);
            emitted.insert(ready);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CompiledConfiguration, ConductorRuntime, DomainEvent};
    use phenix_core::{
        AgentDefinition, BackendId, CallableDescriptor, CallableKind, CallablePolicy,
        CapabilitySet, ExecutionAuthority, ExecutionKind, ExecutionTarget, InferenceOptions,
        ModelId, ModelTarget, OrchestrationDefinition, OrchestrationNode, OrchestrationNodeId,
        ProviderId,
    };

    fn hook(id: &str, event: LifecycleEvent, after: &[&str]) -> LifecycleHookDefinition {
        LifecycleHookDefinition {
            id: LifecycleHookId(id.to_owned()),
            event,
            after: after
                .iter()
                .map(|dependency| LifecycleHookId((*dependency).to_owned()))
                .collect(),
            action: HookAction::Observe,
            failure_policy: HookFailurePolicy::FailOperation,
        }
    }

    fn target() -> ExecutionTarget {
        ExecutionTarget::Fixed(ModelTarget {
            backend: BackendId::parse("mock").unwrap(),
            provider: ProviderId::parse("mock").unwrap(),
            model: ModelId::parse("test").unwrap(),
            inference: InferenceOptions::default(),
        })
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
    fn forward_dependency_registration_is_order_independent() {
        let mut registry = LifecycleHookRegistry::default();
        registry
            .register(hook("second", LifecycleEvent::ExecutionCreated, &["first"]))
            .unwrap();
        registry
            .register(hook("first", LifecycleEvent::ExecutionCreated, &[]))
            .unwrap();

        let ordered = registry
            .ordered_for_event(&LifecycleEvent::ExecutionCreated)
            .unwrap();
        assert_eq!(
            ordered
                .iter()
                .map(|definition| definition.id.0.as_str())
                .collect::<Vec<_>>(),
            vec!["first", "second"]
        );
    }

    #[test]
    fn dependency_cycles_are_rejected_when_event_order_is_resolved() {
        let mut registry = LifecycleHookRegistry::default();
        registry
            .register(hook("first", LifecycleEvent::ExecutionCreated, &["second"]))
            .unwrap();
        registry
            .register(hook("second", LifecycleEvent::ExecutionCreated, &["first"]))
            .unwrap();

        assert!(matches!(
            registry.ordered_for_event(&LifecycleEvent::ExecutionCreated),
            Err(LifecycleHookError::DependencyCycle {
                event: LifecycleEvent::ExecutionCreated
            })
        ));
    }

    #[test]
    fn cross_event_dependencies_are_rejected_when_event_order_is_resolved() {
        let mut registry = LifecycleHookRegistry::default();
        registry
            .register(hook("created", LifecycleEvent::ExecutionCreated, &[]))
            .unwrap();
        registry
            .register(hook(
                "completed",
                LifecycleEvent::ExecutionCompleted,
                &["created"],
            ))
            .unwrap();

        assert!(matches!(
            registry.ordered_for_event(&LifecycleEvent::ExecutionCompleted),
            Err(LifecycleHookError::CrossEventDependency { .. })
        ));
    }

    #[test]
    fn orchestration_action_creates_canonical_orchestration_execution() {
        let worker = CallableId::parse("worker").unwrap();
        let pipeline = CallableId::parse("pipeline").unwrap();
        let mut configuration = CompiledConfiguration::default();
        configuration
            .register_agent(agent_definition("worker"))
            .unwrap();
        configuration
            .register_orchestration(OrchestrationDefinition {
                descriptor: CallableDescriptor {
                    id: pipeline.clone(),
                    kind: CallableKind::Orchestration,
                    description: "hook test orchestration".to_owned(),
                    input_schema: json!({"type": "object"}),
                    output_schema: json!({"type": "object"}),
                    capabilities: CapabilitySet::default(),
                    policy: CallablePolicy::default(),
                },
                interface_agent: None,
                nodes: vec![OrchestrationNode {
                    id: OrchestrationNodeId::parse("step").unwrap(),
                    callable: worker,
                    depends_on: Vec::new(),
                    objective: Some("run hook step".to_owned()),
                    input_bindings: BTreeMap::new(),
                }],
                output_bindings: BTreeMap::new(),
            })
            .unwrap();
        configuration
            .register_lifecycle_hook(LifecycleHookDefinition {
                id: LifecycleHookId("run-pipeline".to_owned()),
                event: LifecycleEvent::ExecutionCreated,
                after: BTreeSet::new(),
                action: HookAction::InvokeOrchestration {
                    callable_id: pipeline.clone(),
                },
                failure_policy: HookFailurePolicy::FailOperation,
            })
            .unwrap();

        let mut runtime = ConductorRuntime::new();
        runtime.reload_configuration(configuration).unwrap();
        let session = runtime.create_session(None, None, target()).unwrap();
        let root = runtime.submit(&session.id, "root").unwrap();

        let created = runtime
            .journal()
            .entries
            .iter()
            .filter_map(|entry| match &entry.event {
                DomainEvent::ExecutionCreated { execution, .. } => Some(execution),
                _ => None,
            })
            .collect::<Vec<_>>();
        let orchestration = created
            .iter()
            .find(|execution| {
                execution.parent_execution.as_ref() == Some(&root.id)
                    && execution.kind == ExecutionKind::Orchestration
                    && execution.callable.as_ref() == Some(&pipeline)
            })
            .expect("hook must use the canonical orchestration execution path");
        assert!(created.iter().any(|execution| {
            execution.parent_execution.as_ref() == Some(&orchestration.id)
                && execution.kind == ExecutionKind::Agent
        }));
    }
}
