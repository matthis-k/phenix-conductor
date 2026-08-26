use phenix_core::{
    DurableSchema, PluginHost, PluginInstance, ResourceNamespace, ServiceId, TransactionOp,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const EXECUTION_CONFIGURATION_SERVICE: &str = "phenix.execution.configuration@1";
const EXECUTION_CONFIGURATION_NAMESPACE: &str = "phenix.execution.configuration";
const STATE_KEY: &str = "state";

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct CallablePolicy {
    #[serde(default)]
    pub requires_permission: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub id: String,
    pub kind: String,
    pub description: String,
    pub input_schema: Value,
    pub output_schema: Value,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub policy: CallablePolicy,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationNode {
    pub callable: String,
    pub objective: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationDefinition {
    pub descriptor: AgentDefinition,
    pub policy: String,
    pub nodes: Vec<OrchestrationNode>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum ExecutionConfigurationCommand {
    RegisterAgent {
        agent: AgentDefinition,
    },
    GetAgent {
        id: String,
    },
    ListAgents,
    RegisterOrchestration {
        orchestration: OrchestrationDefinition,
    },
    GetOrchestration {
        id: String,
    },
    ListOrchestrations,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "response", rename_all = "snake_case")]
pub enum ExecutionConfigurationResponse {
    Agent {
        agent: Option<AgentDefinition>,
    },
    Agents {
        agents: Vec<AgentDefinition>,
    },
    Orchestration {
        orchestration: Option<OrchestrationDefinition>,
    },
    Orchestrations {
        orchestrations: Vec<OrchestrationDefinition>,
    },
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
struct ExecutionConfigurationState {
    agents: BTreeMap<String, AgentDefinition>,
    orchestrations: BTreeMap<String, OrchestrationDefinition>,
}

pub(crate) fn execution_configuration_namespace() -> ResourceNamespace {
    ResourceNamespace::parse(EXECUTION_CONFIGURATION_NAMESPACE)
        .expect("static execution configuration namespace is valid")
}

#[must_use]
pub fn execution_configuration_service() -> ServiceId {
    ServiceId::parse(EXECUTION_CONFIGURATION_SERVICE)
        .expect("static execution configuration service is valid")
}

pub(crate) fn configuration_factory() -> Box<dyn PluginInstance> {
    Box::new(ExecutionConfigurationPlugin)
}

struct ExecutionConfigurationPlugin;

impl PluginInstance for ExecutionConfigurationPlugin {
    fn start(&mut self, host: &PluginHost<'_>) -> Result<(), String> {
        host.register_durable_schema(&DurableSchema::new(execution_configuration_namespace(), 1))
            .map_err(|error| error.to_string())
    }

    fn invoke(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        if service != &execution_configuration_service() {
            return Err(format!(
                "unsupported execution configuration service: {service}"
            ));
        }
        let command: ExecutionConfigurationCommand =
            serde_json::from_slice(input).map_err(|error| error.to_string())?;
        let response = execute(command, host)?;
        serde_json::to_vec(&response).map_err(|error| error.to_string())
    }
}

fn execute(
    command: ExecutionConfigurationCommand,
    host: &PluginHost<'_>,
) -> Result<ExecutionConfigurationResponse, String> {
    match command {
        ExecutionConfigurationCommand::GetAgent { id } => {
            let (_, state) = read_state(host)?;
            Ok(ExecutionConfigurationResponse::Agent {
                agent: state.agents.get(&id).cloned(),
            })
        }
        ExecutionConfigurationCommand::ListAgents => {
            let (_, state) = read_state(host)?;
            Ok(ExecutionConfigurationResponse::Agents {
                agents: state.agents.into_values().collect(),
            })
        }
        ExecutionConfigurationCommand::GetOrchestration { id } => {
            let (_, state) = read_state(host)?;
            Ok(ExecutionConfigurationResponse::Orchestration {
                orchestration: state.orchestrations.get(&id).cloned(),
            })
        }
        ExecutionConfigurationCommand::ListOrchestrations => {
            let (_, state) = read_state(host)?;
            Ok(ExecutionConfigurationResponse::Orchestrations {
                orchestrations: state.orchestrations.into_values().collect(),
            })
        }
        ExecutionConfigurationCommand::RegisterAgent { agent } => mutate_state(host, |state| {
            validate_agent(&agent)?;
            insert_immutable(&mut state.agents, agent.id.clone(), agent.clone(), "agent")?;
            Ok(ExecutionConfigurationResponse::Agent { agent: Some(agent) })
        }),
        ExecutionConfigurationCommand::RegisterOrchestration { orchestration } => {
            mutate_state(host, |state| {
                validate_orchestration(&orchestration, &state.agents)?;
                insert_immutable(
                    &mut state.orchestrations,
                    orchestration.descriptor.id.clone(),
                    orchestration.clone(),
                    "orchestration",
                )?;
                Ok(ExecutionConfigurationResponse::Orchestration {
                    orchestration: Some(orchestration),
                })
            })
        }
    }
}

fn insert_immutable<T: Eq>(
    records: &mut BTreeMap<String, T>,
    id: String,
    value: T,
    label: &str,
) -> Result<(), String> {
    if let Some(existing) = records.get(&id) {
        if existing == &value {
            return Ok(());
        }
        return Err(format!("{label} identity is immutable: {id}"));
    }
    records.insert(id, value);
    Ok(())
}

fn validate_agent(agent: &AgentDefinition) -> Result<(), String> {
    validate_identity("agent id", &agent.id)?;
    if agent.kind != "agent" {
        return Err(format!("agent {} must use kind agent", agent.id));
    }
    validate_identity("agent description", &agent.description)?;
    for capability in &agent.capabilities {
        validate_identity("agent capability", capability)?;
    }
    Ok(())
}

fn validate_orchestration(
    orchestration: &OrchestrationDefinition,
    agents: &BTreeMap<String, AgentDefinition>,
) -> Result<(), String> {
    validate_identity("orchestration id", &orchestration.descriptor.id)?;
    if orchestration.descriptor.kind != "orchestration" {
        return Err(format!(
            "orchestration {} must use kind orchestration",
            orchestration.descriptor.id
        ));
    }
    if orchestration.policy != "sequential" {
        return Err(format!(
            "unsupported orchestration policy for {}: {}",
            orchestration.descriptor.id, orchestration.policy
        ));
    }
    if orchestration.nodes.is_empty() {
        return Err(format!(
            "orchestration {} must contain at least one node",
            orchestration.descriptor.id
        ));
    }
    for node in &orchestration.nodes {
        if !agents.contains_key(&node.callable) {
            return Err(format!(
                "orchestration {} references unknown agent: {}",
                orchestration.descriptor.id, node.callable
            ));
        }
        validate_identity("orchestration node objective", &node.objective)?;
    }
    Ok(())
}

fn validate_identity(label: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{label} must not be empty"))
    } else {
        Ok(())
    }
}

fn mutate_state<F>(
    host: &PluginHost<'_>,
    mutation: F,
) -> Result<ExecutionConfigurationResponse, String>
where
    F: FnOnce(&mut ExecutionConfigurationState) -> Result<ExecutionConfigurationResponse, String>,
{
    let (old, mut state) = read_state(host)?;
    let response = mutation(&mut state)?;
    host.transact_durable(
        &execution_configuration_namespace(),
        &[
            TransactionOp::AssertValue {
                key: STATE_KEY.into(),
                expected: old,
            },
            TransactionOp::Put {
                key: STATE_KEY.into(),
                value: serde_json::to_vec(&state).map_err(|error| error.to_string())?,
            },
        ],
    )
    .map_err(|error| error.to_string())?;
    Ok(response)
}

fn read_state(
    host: &PluginHost<'_>,
) -> Result<(Option<Vec<u8>>, ExecutionConfigurationState), String> {
    let old = host
        .read_durable(&execution_configuration_namespace(), STATE_KEY)
        .map_err(|error| error.to_string())?;
    let state = old
        .as_deref()
        .map(|bytes| serde_json::from_slice(bytes).map_err(|error| error.to_string()))
        .transpose()?
        .unwrap_or_default();
    Ok((old, state))
}
