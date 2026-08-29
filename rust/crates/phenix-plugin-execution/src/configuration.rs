use phenix_core::{
    CallableId, CapabilityId, DurableSchema, PluginContext, PluginHost, PluginInstance,
    ResourceNamespace, ServiceId, TransactionOp,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{btree_map::Entry, BTreeMap, BTreeSet},
    fmt::Display,
};

pub const EXECUTION_CONFIGURATION_SERVICE: &str = "phenix.execution.configuration@1";
const EXECUTION_CONFIGURATION_NAMESPACE: &str = "phenix.execution.configuration";
const STATE_KEY: &str = "state";

type ExecutionConfigurationContext<'host, 'runtime> = PluginContext<'host, 'runtime, ()>;

fn context<'host, 'runtime>(
    host: &'host PluginHost<'runtime>,
) -> ExecutionConfigurationContext<'host, 'runtime> {
    PluginContext::new(host, (), (), ())
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String")]
struct NonEmptyText(String);

impl NonEmptyText {
    fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for NonEmptyText {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.trim().is_empty() {
            return Err("text must not be empty");
        }
        Ok(Self(value))
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct CallablePolicy {
    #[serde(default)]
    pub requires_permission: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct CallableDefinition {
    id: CallableId,
    description: NonEmptyText,
    input_schema: Value,
    output_schema: Value,
    #[serde(default)]
    capabilities: BTreeSet<CapabilityId>,
    #[serde(default)]
    policy: CallablePolicy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum AgentKind {
    Agent,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentDefinition {
    #[serde(flatten)]
    callable: CallableDefinition,
    kind: AgentKind,
}

impl AgentDefinition {
    pub fn id(&self) -> &CallableId {
        &self.callable.id
    }

    pub fn description(&self) -> &str {
        self.callable.description.as_str()
    }

    pub fn input_schema(&self) -> &Value {
        &self.callable.input_schema
    }

    pub fn output_schema(&self) -> &Value {
        &self.callable.output_schema
    }

    pub fn capabilities(&self) -> &BTreeSet<CapabilityId> {
        &self.callable.capabilities
    }

    pub fn policy(&self) -> &CallablePolicy {
        &self.callable.policy
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum OrchestrationKind {
    Orchestration,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct OrchestrationDescriptor {
    #[serde(flatten)]
    callable: CallableDefinition,
    kind: OrchestrationKind,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationNode {
    callable: CallableId,
    objective: NonEmptyText,
}

impl OrchestrationNode {
    pub fn callable(&self) -> &CallableId {
        &self.callable
    }

    pub fn objective(&self) -> &str {
        self.objective.as_str()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Vec<OrchestrationNode>")]
struct NonEmptyNodes(Vec<OrchestrationNode>);

impl NonEmptyNodes {
    fn as_slice(&self) -> &[OrchestrationNode] {
        &self.0
    }
}

impl TryFrom<Vec<OrchestrationNode>> for NonEmptyNodes {
    type Error = &'static str;

    fn try_from(nodes: Vec<OrchestrationNode>) -> Result<Self, Self::Error> {
        if nodes.is_empty() {
            return Err("orchestration must contain at least one node");
        }
        Ok(Self(nodes))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum OrchestrationPolicy {
    Sequential,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationDefinition {
    descriptor: OrchestrationDescriptor,
    policy: OrchestrationPolicy,
    nodes: NonEmptyNodes,
}

impl OrchestrationDefinition {
    pub fn id(&self) -> &CallableId {
        &self.descriptor.callable.id
    }

    pub fn nodes(&self) -> &[OrchestrationNode] {
        self.nodes.as_slice()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum ExecutionConfigurationCommand {
    RegisterAgent {
        agent: AgentDefinition,
    },
    GetAgent {
        id: CallableId,
    },
    ListAgents,
    RegisterOrchestration {
        orchestration: OrchestrationDefinition,
    },
    GetOrchestration {
        id: CallableId,
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
    agents: BTreeMap<CallableId, AgentDefinition>,
    orchestrations: BTreeMap<CallableId, OrchestrationDefinition>,
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
        context(host)
            .kernel
            .register_durable_schema(&DurableSchema::new(execution_configuration_namespace(), 1))
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
        let command = serde_json::from_slice(input).map_err(|error| error.to_string())?;
        let response = execute(&context(host), command)?;
        serde_json::to_vec(&response).map_err(|error| error.to_string())
    }
}

fn execute(
    context: &ExecutionConfigurationContext<'_, '_>,
    command: ExecutionConfigurationCommand,
) -> Result<ExecutionConfigurationResponse, String> {
    match command {
        ExecutionConfigurationCommand::GetAgent { id } => {
            let (_, state) = read_state(context)?;
            Ok(ExecutionConfigurationResponse::Agent {
                agent: state.agents.get(&id).cloned(),
            })
        }
        ExecutionConfigurationCommand::ListAgents => {
            let (_, state) = read_state(context)?;
            Ok(ExecutionConfigurationResponse::Agents {
                agents: state.agents.into_values().collect(),
            })
        }
        ExecutionConfigurationCommand::GetOrchestration { id } => {
            let (_, state) = read_state(context)?;
            Ok(ExecutionConfigurationResponse::Orchestration {
                orchestration: state.orchestrations.get(&id).cloned(),
            })
        }
        ExecutionConfigurationCommand::ListOrchestrations => {
            let (_, state) = read_state(context)?;
            Ok(ExecutionConfigurationResponse::Orchestrations {
                orchestrations: state.orchestrations.into_values().collect(),
            })
        }
        ExecutionConfigurationCommand::RegisterAgent { agent } => mutate_state(context, |state| {
            insert_immutable(
                &mut state.agents,
                agent.id().clone(),
                agent.clone(),
                "agent",
            )?;
            Ok(ExecutionConfigurationResponse::Agent { agent: Some(agent) })
        }),
        ExecutionConfigurationCommand::RegisterOrchestration { orchestration } => {
            mutate_state(context, |state| {
                validate_orchestration(&orchestration, &state.agents)?;
                insert_immutable(
                    &mut state.orchestrations,
                    orchestration.id().clone(),
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

fn insert_immutable<K, T>(
    records: &mut BTreeMap<K, T>,
    id: K,
    value: T,
    label: &str,
) -> Result<(), String>
where
    K: Ord + Display,
    T: Eq,
{
    match records.entry(id) {
        Entry::Vacant(entry) => {
            entry.insert(value);
            Ok(())
        }
        Entry::Occupied(entry) if entry.get() == &value => Ok(()),
        Entry::Occupied(entry) => Err(format!("{label} identity is immutable: {}", entry.key())),
    }
}

fn validate_orchestration(
    orchestration: &OrchestrationDefinition,
    agents: &BTreeMap<CallableId, AgentDefinition>,
) -> Result<(), String> {
    for node in orchestration.nodes() {
        if !agents.contains_key(node.callable()) {
            return Err(format!(
                "orchestration {} references unknown agent: {}",
                orchestration.id(),
                node.callable()
            ));
        }
    }
    Ok(())
}

fn mutate_state<F>(
    context: &ExecutionConfigurationContext<'_, '_>,
    mutation: F,
) -> Result<ExecutionConfigurationResponse, String>
where
    F: FnOnce(&mut ExecutionConfigurationState) -> Result<ExecutionConfigurationResponse, String>,
{
    let (old, mut state) = read_state(context)?;
    let response = mutation(&mut state)?;
    context
        .kernel
        .transact_durable(
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
    context: &ExecutionConfigurationContext<'_, '_>,
) -> Result<(Option<Vec<u8>>, ExecutionConfigurationState), String> {
    let old = context
        .kernel
        .read_durable(&execution_configuration_namespace(), STATE_KEY)
        .map_err(|error| error.to_string())?;
    let state = old
        .as_deref()
        .map(|bytes| serde_json::from_slice(bytes).map_err(|error| error.to_string()))
        .transpose()?
        .unwrap_or_default();
    Ok((old, state))
}
