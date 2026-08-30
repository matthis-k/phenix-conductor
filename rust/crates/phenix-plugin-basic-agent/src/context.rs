use phenix_core::{
    context_service, Authority, CapabilityId, ComponentExport, ComponentId, ComponentInterface,
    ComponentManifest, ContextCommand, ContextDescriptor, ContextResourceRevision, ContextResponse,
    DurableSchema, InterfaceId, PluginContext, PluginExecution, PluginHost, PluginId,
    PluginInstance, PluginManifest, ResourceNamespace, ServiceContribution, ServiceId, ServiceRole,
    TransactionOp, CONTEXT_SERVICE,
};
use sha2::{Digest, Sha256};

pub const BASIC_CONTEXT_PLUGIN: &str = "phenix.basic-context";
pub const BASIC_CONTEXT_COMPONENT: &str = "phenix.basic-context";
const BASIC_CONTEXT_NAMESPACE: &str = "phenix.basic-context.state";
const INDEX_KEY: &str = "context/@all";

type BasicContextContext<'host, 'runtime> = PluginContext<'host, 'runtime, ()>;

fn context<'host, 'runtime>(
    host: &'host PluginHost<'runtime>,
) -> BasicContextContext<'host, 'runtime> {
    PluginContext::new(host, (), (), ())
}

pub struct BasicContextInterface;

impl ComponentInterface for BasicContextInterface {
    fn interface_id() -> InterfaceId {
        InterfaceId::parse(CONTEXT_SERVICE).expect("static context interface id is valid")
    }
}

#[must_use]
pub fn basic_context_manifest() -> PluginManifest {
    PluginManifest {
        id: PluginId::parse(BASIC_CONTEXT_PLUGIN).expect("static plugin id is valid"),
        version: 1,
        execution: PluginExecution::Embedded,
        dependencies: Vec::new(),
        services: vec![ServiceContribution {
            role: ServiceRole::Terminal,
            service: context_service(),
            priority: 10,
            required_authority: Authority::default(),
        }],
        resource_namespaces: vec![namespace()],
        maximum_authority: persistence_authority(),
    }
}

#[must_use]
pub fn basic_context_component_manifest() -> ComponentManifest {
    ComponentManifest {
        id: ComponentId::parse(BASIC_CONTEXT_COMPONENT).expect("static component id is valid"),
        owner: basic_context_manifest().id,
        imports: Vec::new(),
        exports: vec![ComponentExport {
            interface: BasicContextInterface::interface_id(),
            schema: BasicContextInterface::schema(),
            priority: 10,
            required_authority: Authority::default(),
        }],
        maximum_authority: persistence_authority(),
    }
}

#[must_use]
pub fn basic_context_factory() -> Box<dyn PluginInstance> {
    Box::new(BasicContext)
}

struct BasicContext;

impl PluginInstance for BasicContext {
    fn start(&mut self, host: &PluginHost<'_>) -> Result<(), String> {
        context(host)
            .kernel
            .register_durable_schema(&DurableSchema::new(namespace(), 1))
            .map_err(|error| error.to_string())
    }

    fn invoke(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        if service != &context_service() {
            return Err(format!("unsupported basic context service: {service}"));
        }
        let command = serde_json::from_slice(input).map_err(|error| error.to_string())?;
        let response = handle(&context(host), command)?;
        serde_json::to_vec(&response).map_err(|error| error.to_string())
    }
}

fn handle(
    context: &BasicContextContext<'_, '_>,
    command: ContextCommand,
) -> Result<ContextResponse, String> {
    match command {
        ContextCommand::Register {
            resource_id,
            kind,
            source,
            scope,
            content,
        } => {
            if resource_id.trim().is_empty() {
                return Err("context resource id must not be empty".into());
            }
            let content_identity = content_identity(content.as_ref());
            let revision = content_identity.clone();
            let resource = ContextResourceRevision {
                descriptor: ContextDescriptor {
                    resource_id: resource_id.clone(),
                    revision: revision.clone(),
                    kind,
                    source,
                    scope,
                    content_identity,
                    estimated_bytes: u64::try_from(content.as_ref().len())
                        .map_err(|_| "context resource byte length exceeds u64".to_owned())?,
                },
                content,
            };
            write_resource(context, &resource)?;
            Ok(ContextResponse::Registered { resource })
        }
        ContextCommand::Get {
            resource_id,
            revision,
        } => Ok(ContextResponse::Resource {
            resource: read_resource(context, &resource_id, &revision)?,
        }),
        ContextCommand::List => Ok(ContextResponse::Resources {
            descriptors: read_index(context)?
                .into_iter()
                .map(|(id, revision)| {
                    read_resource(context, &id, &revision)?
                        .map(|resource| resource.descriptor)
                        .ok_or_else(|| format!("missing durable context revision: {id}@{revision}"))
                })
                .collect::<Result<Vec<_>, _>>()?,
        }),
    }
}

fn write_resource(
    context: &BasicContextContext<'_, '_>,
    resource: &ContextResourceRevision,
) -> Result<(), String> {
    let identity = (
        resource.descriptor.resource_id.clone(),
        resource.descriptor.revision.clone(),
    );
    let mut index = read_index(context)?;
    if !index.contains(&identity) {
        index.push(identity.clone());
        index.sort();
    }
    context
        .kernel
        .transact_durable(
            &namespace(),
            &[
                TransactionOp::Put {
                    key: resource_key(&identity.0, &identity.1),
                    value: serde_json::to_vec(resource).map_err(|error| error.to_string())?,
                },
                TransactionOp::Put {
                    key: INDEX_KEY.into(),
                    value: serde_json::to_vec(&index).map_err(|error| error.to_string())?,
                },
            ],
        )
        .map_err(|error| error.to_string())
}

fn read_resource(
    context: &BasicContextContext<'_, '_>,
    id: &str,
    revision: &str,
) -> Result<Option<ContextResourceRevision>, String> {
    context
        .kernel
        .read_durable(&namespace(), &resource_key(id, revision))
        .map_err(|error| error.to_string())?
        .map(|value| serde_json::from_slice(&value).map_err(|error| error.to_string()))
        .transpose()
}

fn read_index(context: &BasicContextContext<'_, '_>) -> Result<Vec<(String, String)>, String> {
    context
        .kernel
        .read_durable(&namespace(), INDEX_KEY)
        .map_err(|error| error.to_string())?
        .map(|value| serde_json::from_slice(&value).map_err(|error| error.to_string()))
        .unwrap_or_else(|| Ok(Vec::new()))
}

fn resource_key(id: &str, revision: &str) -> String {
    format!("resource/{id}/{revision}")
}

fn content_identity(content: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(content))
}

fn namespace() -> ResourceNamespace {
    ResourceNamespace::parse(BASIC_CONTEXT_NAMESPACE).expect("static namespace is valid")
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
