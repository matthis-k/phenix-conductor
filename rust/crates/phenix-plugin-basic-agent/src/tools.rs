use phenix_core::{
    tool_service, Authority, CapabilityId, ComponentExport, ComponentId, ComponentInterface,
    ComponentManifest, DurableSchema, InterfaceId, PluginExecution, PluginHost, PluginId,
    PluginInstance, PluginManifest, ResourceNamespace, ServiceContribution, ServiceId, ServiceRole,
    ToolCommand, ToolDefinition, ToolResponse, TransactionOp, TOOL_SERVICE,
};

pub const BASIC_TOOLS_PLUGIN: &str = "phenix.basic-tools";
pub const BASIC_TOOLS_COMPONENT: &str = "phenix.basic-tools";
const BASIC_TOOLS_NAMESPACE: &str = "phenix.basic-tools.state";
const INDEX_KEY: &str = "tools/@all";

pub struct BasicToolsInterface;

impl ComponentInterface for BasicToolsInterface {
    type Request = ToolCommand;
    type Response = ToolResponse;

    fn interface_id() -> InterfaceId {
        InterfaceId::parse(TOOL_SERVICE).expect("static tool interface id is valid")
    }
}

#[must_use]
pub fn basic_tools_manifest() -> PluginManifest {
    PluginManifest {
        id: PluginId::parse(BASIC_TOOLS_PLUGIN).expect("static plugin id is valid"),
        version: 1,
        execution: PluginExecution::Embedded,
        dependencies: Vec::new(),
        services: vec![ServiceContribution {
            role: ServiceRole::Terminal,
            service: tool_service(),
            priority: 10,
            required_authority: Authority::default(),
        }],
        resource_namespaces: vec![namespace()],
        maximum_authority: persistence_authority(),
    }
}

#[must_use]
pub fn basic_tools_component_manifest() -> ComponentManifest {
    ComponentManifest {
        id: ComponentId::parse(BASIC_TOOLS_COMPONENT).expect("static component id is valid"),
        owner: basic_tools_manifest().id,
        imports: Vec::new(),
        exports: vec![ComponentExport {
            interface: BasicToolsInterface::interface_id(),
            priority: 10,
            required_authority: Authority::default(),
        }],
        maximum_authority: persistence_authority(),
    }
}

#[must_use]
pub fn basic_tools_factory() -> Box<dyn PluginInstance> {
    Box::new(BasicTools)
}

struct BasicTools;

impl PluginInstance for BasicTools {
    fn start(&mut self, host: &PluginHost<'_>) -> Result<(), String> {
        host.register_durable_schema(&DurableSchema::new(namespace(), 1))
            .map_err(|error| error.to_string())
    }

    fn invoke(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        if service != &tool_service() {
            return Err(format!("unsupported basic tool service: {service}"));
        }
        let command: ToolCommand =
            serde_json::from_slice(input).map_err(|error| error.to_string())?;
        let response = match command {
            ToolCommand::Register { tool } => {
                require_id(&tool.id)?;
                write_tool(host, &tool)?;
                ToolResponse::Tool { tool: Some(tool) }
            }
            ToolCommand::Get { id } => ToolResponse::Tool {
                tool: read_tool(host, &id)?,
            },
            ToolCommand::List => ToolResponse::Tools {
                tools: read_ids(host)?
                    .into_iter()
                    .map(|id| {
                        read_tool(host, &id)?.ok_or_else(|| format!("missing durable tool: {id}"))
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            },
            ToolCommand::Invoke { id, input } => {
                let tool = read_tool(host, &id)?.ok_or_else(|| format!("unknown tool: {id}"))?;
                let mut output = tool.output_prefix;
                output.extend(input);
                ToolResponse::Output { output }
            }
        };
        serde_json::to_vec(&response).map_err(|error| error.to_string())
    }
}

fn write_tool(host: &PluginHost<'_>, tool: &ToolDefinition) -> Result<(), String> {
    let mut ids = read_ids(host)?;
    if !ids.contains(&tool.id) {
        ids.push(tool.id.clone());
        ids.sort();
    }
    host.transact_durable(
        &namespace(),
        &[
            TransactionOp::Put {
                key: format!("tool/{}", tool.id),
                value: serde_json::to_vec(tool).map_err(|error| error.to_string())?,
            },
            TransactionOp::Put {
                key: INDEX_KEY.into(),
                value: serde_json::to_vec(&ids).map_err(|error| error.to_string())?,
            },
        ],
    )
    .map_err(|error| error.to_string())
}

fn read_tool(host: &PluginHost<'_>, id: &str) -> Result<Option<ToolDefinition>, String> {
    host.read_durable(&namespace(), &format!("tool/{id}"))
        .map_err(|error| error.to_string())?
        .map(|value| serde_json::from_slice(&value).map_err(|error| error.to_string()))
        .transpose()
}

fn read_ids(host: &PluginHost<'_>) -> Result<Vec<String>, String> {
    host.read_durable(&namespace(), INDEX_KEY)
        .map_err(|error| error.to_string())?
        .map(|value| serde_json::from_slice(&value).map_err(|error| error.to_string()))
        .unwrap_or_else(|| Ok(Vec::new()))
}

fn require_id(id: &str) -> Result<(), String> {
    if id.trim().is_empty() {
        Err("tool id must not be empty".into())
    } else {
        Ok(())
    }
}

fn namespace() -> ResourceNamespace {
    ResourceNamespace::parse(BASIC_TOOLS_NAMESPACE).expect("static namespace is valid")
}

fn persistence_authority() -> Authority {
    Authority::new([
        capability("kernel.persistence.schema"),
        capability("kernel.persistence.read"),
        capability("kernel.persistence.write"),
    ])
}

fn capability(value: &str) -> CapabilityId {
    CapabilityId::parse(value).expect("static capability is valid")
}
