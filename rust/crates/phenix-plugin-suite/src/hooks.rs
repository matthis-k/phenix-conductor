use crate::{
    context_service, execution_service, ContextCommand, ContextInjectionLifetime,
    ContextInjectionRequester, ExecutionCommand, ExecutionResponse,
};
use phenix_kernel::{
    Authority, CapabilityId, DurableSchema, PluginExecution, PluginHost, PluginId, PluginInstance,
    PluginManifest, ResourceNamespace, ServiceContribution, ServiceId, TransactionOp,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const HOOK_SERVICE: &str = "phenix.hooks@1";
const HOOK_PLUGIN: &str = "phenix.hooks";
const HOOK_NAMESPACE: &str = "phenix.hooks.state";
const PERSISTENCE_SCHEMA: &str = "kernel.persistence.schema";
const PERSISTENCE_READ: &str = "kernel.persistence.read";
const PERSISTENCE_WRITE: &str = "kernel.persistence.write";

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleEvent {
    ExecutionCreated,
    ExecutionCompleted,
    ExecutionFailed,
    ContextLoading,
    CallableStart,
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
#[serde(tag = "action", rename_all = "snake_case")]
pub enum HookAction {
    Observe,
    InjectContext {
        resource_id: String,
        revision: String,
        reason: String,
    },
    InvokeCallable {
        callable_id: String,
        input: Vec<u8>,
    },
    EmitMetadata {
        key: String,
        value: serde_json::Value,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HookDefinition {
    pub id: String,
    pub event: LifecycleEvent,
    #[serde(default)]
    pub depends_on: BTreeSet<String>,
    pub action: HookAction,
    pub failure_policy: HookFailurePolicy,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HookConfiguration {
    pub revision: String,
    pub hooks: Vec<HookDefinition>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HookWarning {
    pub hook_id: String,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HookDispatch {
    pub executed: Vec<String>,
    pub warnings: Vec<HookWarning>,
    pub metadata: BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum HookCommand {
    RegisterConfiguration {
        configuration: HookConfiguration,
    },
    GetConfiguration {
        revision: String,
    },
    Trigger {
        revision: String,
        event: LifecycleEvent,
        execution_id: String,
        causality_id: u64,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "response", rename_all = "snake_case")]
pub enum HookResponse {
    Configuration {
        configuration: Option<HookConfiguration>,
    },
    Dispatch {
        dispatch: HookDispatch,
    },
}

#[must_use]
pub fn hook_manifest(maximum_authority: Authority) -> PluginManifest {
    let persistence = Authority::new([
        capability(PERSISTENCE_SCHEMA),
        capability(PERSISTENCE_READ),
        capability(PERSISTENCE_WRITE),
    ]);
    PluginManifest {
        id: PluginId::parse(HOOK_PLUGIN).expect("static plugin id is valid"),
        version: 1,
        execution: PluginExecution::Embedded,
        dependencies: Vec::new(),
        services: vec![ServiceContribution {
            service: hook_service(),
            priority: 100,
            required_authority: Authority::default(),
        }],
        resource_namespaces: vec![hook_namespace()],
        maximum_authority: Authority::new(
            maximum_authority
                .capabilities()
                .cloned()
                .chain(persistence.capabilities().cloned()),
        ),
    }
}

#[must_use]
pub fn hook_factory() -> Box<dyn PluginInstance> {
    Box::new(HookPlugin::default())
}

#[must_use]
pub fn hook_service() -> ServiceId {
    ServiceId::parse(HOOK_SERVICE).expect("static service id is valid")
}

fn hook_namespace() -> ResourceNamespace {
    ResourceNamespace::parse(HOOK_NAMESPACE).expect("static namespace is valid")
}

fn capability(value: &str) -> CapabilityId {
    CapabilityId::parse(value).expect("static capability is valid")
}

#[derive(Default)]
struct HookPlugin {
    active: BTreeSet<(u64, String)>,
}

impl PluginInstance for HookPlugin {
    fn start(&mut self, host: &PluginHost<'_>) -> Result<(), String> {
        host.register_durable_schema(&DurableSchema::new(hook_namespace(), 1))
            .map_err(|error| error.to_string())
    }

    fn invoke(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        if service != &hook_service() {
            return Err(format!("unsupported hook service: {service}"));
        }
        let command: HookCommand =
            serde_json::from_slice(input).map_err(|error| error.to_string())?;
        let response = match command {
            HookCommand::RegisterConfiguration { configuration } => {
                validate_configuration(&configuration)?;
                insert_configuration(host, &configuration)?;
                HookResponse::Configuration {
                    configuration: Some(configuration),
                }
            }
            HookCommand::GetConfiguration { revision } => HookResponse::Configuration {
                configuration: read_configuration(host, &revision)?,
            },
            HookCommand::Trigger {
                revision,
                event,
                execution_id,
                causality_id,
            } => HookResponse::Dispatch {
                dispatch: self.dispatch(host, &revision, event, &execution_id, causality_id)?,
            },
        };
        serde_json::to_vec(&response).map_err(|error| error.to_string())
    }
}

impl HookPlugin {
    fn dispatch(
        &mut self,
        host: &PluginHost<'_>,
        revision: &str,
        event: LifecycleEvent,
        execution_id: &str,
        causality_id: u64,
    ) -> Result<HookDispatch, String> {
        let configuration = read_configuration(host, revision)?
            .ok_or_else(|| format!("unknown hook configuration revision: {revision}"))?;
        let hooks = ordered_hooks(&configuration, &event)?;
        let mut dispatch = HookDispatch {
            executed: Vec::new(),
            warnings: Vec::new(),
            metadata: BTreeMap::new(),
        };
        for hook in hooks {
            let causal_key = (causality_id, hook.id.clone());
            if !self.active.insert(causal_key.clone()) {
                let message = format!("causal hook re-entry blocked: {}", hook.id);
                handle_failure(hook, message, &mut dispatch)?;
                continue;
            }
            let result = execute_action(host, hook, execution_id, &mut dispatch);
            self.active.remove(&causal_key);
            match result {
                Ok(()) => dispatch.executed.push(hook.id.clone()),
                Err(message) => handle_failure(hook, message, &mut dispatch)?,
            }
        }
        Ok(dispatch)
    }
}

fn execute_action(
    host: &PluginHost<'_>,
    hook: &HookDefinition,
    execution_id: &str,
    dispatch: &mut HookDispatch,
) -> Result<(), String> {
    match &hook.action {
        HookAction::Observe => Ok(()),
        HookAction::EmitMetadata { key, value } => {
            if key.trim().is_empty() {
                return Err("hook metadata key must not be empty".into());
            }
            dispatch.metadata.insert(key.clone(), value.clone());
            Ok(())
        }
        HookAction::InjectContext {
            resource_id,
            revision,
            reason,
        } => {
            let command = ContextCommand::Load {
                execution_id: execution_id.to_owned(),
                resource_id: resource_id.clone(),
                revision: revision.clone(),
                requester: ContextInjectionRequester::Hook,
                lifetime: ContextInjectionLifetime::Execution,
                reason: reason.clone(),
            };
            host.invoke_service(
                &context_service(),
                &serde_json::to_vec(&command).map_err(|error| error.to_string())?,
                host.authority(),
                None,
            )
            .map(|_| ())
            .map_err(|error| error.to_string())
        }
        HookAction::InvokeCallable { callable_id, input } => {
            let command = ExecutionCommand::InvokeCallable {
                execution_id: execution_id.to_owned(),
                callable_id: callable_id.clone(),
                input: input.clone(),
            };
            let output = host
                .invoke_service(
                    &execution_service(),
                    &serde_json::to_vec(&command).map_err(|error| error.to_string())?,
                    host.authority(),
                    None,
                )
                .map_err(|error| error.to_string())?;
            match serde_json::from_slice::<ExecutionResponse>(&output)
                .map_err(|error| error.to_string())?
            {
                ExecutionResponse::Invocation { .. } => Ok(()),
                other => Err(format!("unexpected callable response: {other:?}")),
            }
        }
    }
}

fn handle_failure(
    hook: &HookDefinition,
    message: String,
    dispatch: &mut HookDispatch,
) -> Result<(), String> {
    match hook.failure_policy {
        HookFailurePolicy::Ignore => Ok(()),
        HookFailurePolicy::Warn => {
            dispatch.warnings.push(HookWarning {
                hook_id: hook.id.clone(),
                message,
            });
            Ok(())
        }
        HookFailurePolicy::FailOperation => Err(format!("hook {} failed: {message}", hook.id)),
    }
}

fn validate_configuration(configuration: &HookConfiguration) -> Result<(), String> {
    validate_id("hook configuration revision", &configuration.revision)?;
    let mut ids = BTreeSet::new();
    for hook in &configuration.hooks {
        validate_id("hook id", &hook.id)?;
        if !ids.insert(hook.id.clone()) {
            return Err(format!("duplicate hook id: {}", hook.id));
        }
    }
    for event in [
        LifecycleEvent::ExecutionCreated,
        LifecycleEvent::ExecutionCompleted,
        LifecycleEvent::ExecutionFailed,
        LifecycleEvent::ContextLoading,
        LifecycleEvent::CallableStart,
        LifecycleEvent::CallableCompleted,
    ] {
        ordered_hooks(configuration, &event)?;
    }
    Ok(())
}

fn ordered_hooks<'a>(
    configuration: &'a HookConfiguration,
    event: &LifecycleEvent,
) -> Result<Vec<&'a HookDefinition>, String> {
    let by_id: BTreeMap<&str, &HookDefinition> = configuration
        .hooks
        .iter()
        .filter(|hook| &hook.event == event)
        .map(|hook| (hook.id.as_str(), hook))
        .collect();
    for hook in by_id.values() {
        for dependency in &hook.depends_on {
            if !by_id.contains_key(dependency.as_str()) {
                return Err(format!(
                    "hook {} depends on unknown hook {dependency} for event {event:?}",
                    hook.id
                ));
            }
        }
    }
    let mut remaining: BTreeSet<&str> = by_id.keys().copied().collect();
    let mut done = BTreeSet::new();
    let mut ordered = Vec::new();
    while !remaining.is_empty() {
        let ready = remaining
            .iter()
            .copied()
            .find(|id| {
                by_id[id]
                    .depends_on
                    .iter()
                    .all(|dependency| done.contains(dependency.as_str()))
            })
            .ok_or_else(|| format!("hook dependency cycle detected for event {event:?}"))?;
        remaining.remove(ready);
        done.insert(ready);
        ordered.push(by_id[ready]);
    }
    Ok(ordered)
}

fn insert_configuration(
    host: &PluginHost<'_>,
    configuration: &HookConfiguration,
) -> Result<(), String> {
    let key = configuration_key(&configuration.revision);
    host.transact_durable(
        &hook_namespace(),
        &[
            TransactionOp::AssertValue {
                key: key.clone(),
                expected: None,
            },
            TransactionOp::Put {
                key,
                value: serde_json::to_vec(configuration).map_err(|error| error.to_string())?,
            },
        ],
    )
    .map_err(|error| error.to_string())
}

fn read_configuration(
    host: &PluginHost<'_>,
    revision: &str,
) -> Result<Option<HookConfiguration>, String> {
    validate_id("hook configuration revision", revision)?;
    host.read_durable(&hook_namespace(), &configuration_key(revision))
        .map_err(|error| error.to_string())?
        .map(|value| serde_json::from_slice(&value).map_err(|error| error.to_string()))
        .transpose()
}

fn configuration_key(revision: &str) -> String {
    format!("configuration/{revision}")
}

fn validate_id(label: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() || value.contains('/') {
        Err(format!(
            "{label} must be non-empty and must not contain '/'"
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_kernel::{Kernel, KernelConfig, LocalPersistence};
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_db(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "phenix-{name}-{}-{nonce}.sqlite",
            std::process::id()
        ))
    }

    fn kernel(path: &PathBuf) -> Kernel {
        let manifest = hook_manifest(Authority::default());
        let plugin = manifest.id.clone();
        let persistence = LocalPersistence::open(path).unwrap();
        let mut kernel =
            Kernel::with_persistence(KernelConfig::new([manifest]).unwrap(), persistence);
        kernel
            .register_embedded_factory(plugin, hook_factory)
            .unwrap();
        kernel.activate_all().unwrap();
        kernel
    }

    fn invoke(kernel: &mut Kernel, command: HookCommand) -> Result<HookResponse, String> {
        let output = kernel
            .invoke(
                &hook_service(),
                &serde_json::to_vec(&command).unwrap(),
                &hook_manifest(Authority::default()).maximum_authority,
                None,
            )
            .map_err(|error| error.to_string())?;
        serde_json::from_slice(&output).map_err(|error| error.to_string())
    }

    fn observe(id: &str, depends_on: &[&str]) -> HookDefinition {
        HookDefinition {
            id: id.into(),
            event: LifecycleEvent::ExecutionCompleted,
            depends_on: depends_on.iter().map(|value| (*value).to_owned()).collect(),
            action: HookAction::Observe,
            failure_policy: HookFailurePolicy::FailOperation,
        }
    }

    #[test]
    fn hook_configuration_is_immutable_durable_and_ordered_by_dependency() {
        let path = temp_db("hooks-order");
        let configuration = HookConfiguration {
            revision: "config-1".into(),
            hooks: vec![observe("second", &["first"]), observe("first", &[])],
        };
        {
            let mut kernel = kernel(&path);
            invoke(
                &mut kernel,
                HookCommand::RegisterConfiguration {
                    configuration: configuration.clone(),
                },
            )
            .unwrap();
            let duplicate_error = invoke(
                &mut kernel,
                HookCommand::RegisterConfiguration {
                    configuration: configuration.clone(),
                },
            )
            .unwrap_err();
            assert!(duplicate_error.contains("transaction assertion failed"));
            assert!(duplicate_error.contains("configuration/config-1"));
        }
        let mut restored = kernel(&path);
        let dispatch = invoke(
            &mut restored,
            HookCommand::Trigger {
                revision: "config-1".into(),
                event: LifecycleEvent::ExecutionCompleted,
                execution_id: "execution-1".into(),
                causality_id: 1,
            },
        )
        .unwrap();
        match dispatch {
            HookResponse::Dispatch { dispatch } => {
                assert_eq!(dispatch.executed, vec!["first", "second"])
            }
            other => panic!("unexpected response: {other:?}"),
        }
        let _ = fs::remove_file(path);
    }

    #[test]
    fn hook_cycle_is_rejected_independent_of_registration_order() {
        let path = temp_db("hooks-cycle");
        let mut kernel = kernel(&path);
        let error = invoke(
            &mut kernel,
            HookCommand::RegisterConfiguration {
                configuration: HookConfiguration {
                    revision: "config-cycle".into(),
                    hooks: vec![observe("a", &["b"]), observe("b", &["a"])],
                },
            },
        )
        .unwrap_err();
        assert!(error.contains("cycle"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn warning_policy_records_warning_without_failing_dispatch() {
        let path = temp_db("hooks-warn");
        let mut kernel = kernel(&path);
        invoke(
            &mut kernel,
            HookCommand::RegisterConfiguration {
                configuration: HookConfiguration {
                    revision: "config-1".into(),
                    hooks: vec![HookDefinition {
                        id: "warn".into(),
                        event: LifecycleEvent::ExecutionCompleted,
                        depends_on: BTreeSet::new(),
                        action: HookAction::InvokeCallable {
                            callable_id: "missing".into(),
                            input: vec![],
                        },
                        failure_policy: HookFailurePolicy::Warn,
                    }],
                },
            },
        )
        .unwrap();
        let response = invoke(
            &mut kernel,
            HookCommand::Trigger {
                revision: "config-1".into(),
                event: LifecycleEvent::ExecutionCompleted,
                execution_id: "execution-1".into(),
                causality_id: 2,
            },
        )
        .unwrap();
        match response {
            HookResponse::Dispatch { dispatch } => {
                assert_eq!(dispatch.warnings.len(), 1);
                assert!(dispatch.executed.is_empty());
            }
            other => panic!("unexpected response: {other:?}"),
        }
        let _ = fs::remove_file(path);
    }
}
