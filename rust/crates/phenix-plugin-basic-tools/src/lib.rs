use phenix_core::{
    Authority, CallableId, CapabilityId, ComponentId, ComponentInterface, ComponentManifest,
    InterfaceId, PluginContext, PluginInstance, PluginManifest, ResourceNamespace, ToolCommand,
    ToolDefinition, ToolResponse, TransactionOp, TOOL_SERVICE,
};
use phenix_sdk::{StaticPluginComponentDispatch, StaticPluginDefinition};

pub const BASIC_TOOLS_PLUGIN: &str = "phenix.basic-tools";
pub const BASIC_TOOLS_COMPONENT: &str = "phenix.basic-tools";
const BASIC_TOOLS_NAMESPACE: &str = "phenix.basic-tools.state";
const INDEX_KEY: &str = "tools/@all";

type BasicToolsContext<'host, 'runtime> = PluginContext<'host, 'runtime, ()>;

pub struct BasicToolsInterface;

impl ComponentInterface for BasicToolsInterface {
    fn interface_id() -> InterfaceId {
        InterfaceId::parse(TOOL_SERVICE).expect("static tool interface id is valid")
    }
}

struct ToolStore;

#[phenix_sdk::resource(schema = 1)]
impl ToolStore {}

#[phenix_sdk::component]
struct Api;

#[phenix_sdk::component]
impl Api {
    #[phenix(export("phenix.tools@1"), terminal, priority = 10)]
    fn handle(
        &mut self,
        context: &phenix_sdk::PluginContext<'_, '_, ()>,
        command: ToolCommand,
    ) -> Result<ToolResponse, String> {
        handle(context, command)
    }
}

#[phenix_sdk::plugin(id = "phenix.basic-tools", authority = persistence_authority())]
pub struct Plugin {
    #[phenix(component, id = "phenix.basic-tools")]
    api: Api,

    #[phenix(resource, id = "phenix.basic-tools.state")]
    _state: phenix_sdk::Durable<ToolStore>,
}

#[must_use]
pub fn basic_tools_manifest() -> PluginManifest {
    Plugin::manifest()
}

#[must_use]
pub fn basic_tools_component_manifest() -> ComponentManifest {
    Plugin::component_manifests()
        .into_iter()
        .next()
        .expect("basic tools plugin has one generated component")
}

#[must_use]
pub fn basic_tools_factory() -> Box<dyn PluginInstance> {
    StaticPluginComponentDispatch::into_plugin_instance(Plugin {
        api: Api,
        _state: phenix_sdk::Durable::new(),
    })
}

#[must_use]
pub fn basic_tools_component_id() -> ComponentId {
    basic_tools_component_manifest().id
}

fn handle(
    context: &BasicToolsContext<'_, '_>,
    command: ToolCommand,
) -> Result<ToolResponse, String> {
    match command {
        ToolCommand::Register { tool } => {
            write_tool(context, &tool)?;
            Ok(ToolResponse::Tool { tool: Some(tool) })
        }
        ToolCommand::Get { id } => Ok(ToolResponse::Tool {
            tool: read_tool(context, &id)?,
        }),
        ToolCommand::List => Ok(ToolResponse::Tools {
            tools: read_ids(context)?
                .into_iter()
                .map(|id| {
                    read_tool(context, &id)?.ok_or_else(|| format!("missing durable tool: {id}"))
                })
                .collect::<Result<Vec<_>, _>>()?,
        }),
        ToolCommand::Invoke { id, input } => {
            let tool = read_tool(context, &id)?.ok_or_else(|| format!("unknown tool: {id}"))?;
            let mut output = tool.output_prefix.into_vec();
            output.extend_from_slice(input.as_ref());
            Ok(ToolResponse::Output {
                output: output.into(),
            })
        }
    }
}

fn write_tool(context: &BasicToolsContext<'_, '_>, tool: &ToolDefinition) -> Result<(), String> {
    let mut ids = read_ids(context)?;
    if !ids.contains(&tool.id) {
        ids.push(tool.id.clone());
        ids.sort();
    }
    context
        .kernel
        .transact_durable(
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

fn read_tool(
    context: &BasicToolsContext<'_, '_>,
    id: &CallableId,
) -> Result<Option<ToolDefinition>, String> {
    context
        .kernel
        .read_durable(&namespace(), &format!("tool/{id}"))
        .map_err(|error| error.to_string())?
        .map(|value| serde_json::from_slice(&value).map_err(|error| error.to_string()))
        .transpose()
}

fn read_ids(context: &BasicToolsContext<'_, '_>) -> Result<Vec<CallableId>, String> {
    context
        .kernel
        .read_durable(&namespace(), INDEX_KEY)
        .map_err(|error| error.to_string())?
        .map(|value| serde_json::from_slice(&value).map_err(|error| error.to_string()))
        .unwrap_or_else(|| Ok(Vec::new()))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_authoring_preserves_stable_identity() {
        let manifest = basic_tools_manifest();
        assert_eq!(manifest.id.as_str(), BASIC_TOOLS_PLUGIN);
        assert!(manifest.services.is_empty());
        assert_eq!(manifest.resource_namespaces, vec![namespace()]);

        let component = basic_tools_component_manifest();
        assert_eq!(component.id.as_str(), BASIC_TOOLS_COMPONENT);
        assert_eq!(component.exports.len(), 1);
        assert_eq!(
            component.exports[0].interface,
            BasicToolsInterface::interface_id()
        );
    }
}
