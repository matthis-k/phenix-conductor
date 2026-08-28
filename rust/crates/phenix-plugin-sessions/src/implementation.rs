use phenix_core::{
    session_service, Authority, CapabilityId, ComponentExport, ComponentId, ComponentInterface,
    ComponentManifest, DurableSchema, InterfaceId, NamespaceTransaction, PluginExecution,
    PluginHost, PluginId, PluginInstance, PluginManifest, ResourceNamespace, ServiceContribution,
    ServiceId, TransactionOp,
};
pub use phenix_core::{
    SessionCommand, SessionInput, SessionInputKind, SessionRecord, SessionResponse, SESSION_SERVICE,
};
use serde::{Deserialize, Serialize};

const SESSION_PLUGIN: &str = "phenix.sessions";
const SESSION_COMPONENT: &str = "phenix.sessions";
const SESSION_NAMESPACE: &str = "phenix.sessions.state";
pub const SESSION_MUTATION_SERVICE: &str = "phenix.sessions.mutation@1";
const PERSISTENCE_SCHEMA: &str = "kernel.persistence.schema";
const PERSISTENCE_READ: &str = "kernel.persistence.read";
const PERSISTENCE_WRITE: &str = "kernel.persistence.write";
const ALL_SESSIONS_KEY: &str = "sessions/@all";

pub struct SessionInterface;

impl ComponentInterface for SessionInterface {
    type Request = SessionCommand;
    type Response = SessionResponse;

    fn interface_id() -> InterfaceId {
        InterfaceId::parse(SESSION_SERVICE).expect("static session interface id is valid")
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum SessionMutationCommand {
    PrepareCreate { id: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case", deny_unknown_fields)]
pub enum SessionMutationResponse {
    PreparedCreate {
        session: SessionRecord,
        transaction: NamespaceTransaction,
    },
}

pub struct SessionMutationInterface;

impl ComponentInterface for SessionMutationInterface {
    type Request = SessionMutationCommand;
    type Response = SessionMutationResponse;

    fn interface_id() -> InterfaceId {
        InterfaceId::parse(SESSION_MUTATION_SERVICE)
            .expect("static session mutation interface id is valid")
    }
}

#[must_use]
pub fn session_mutation_service() -> ServiceId {
    ServiceId::parse(SESSION_MUTATION_SERVICE).expect("static session mutation service id is valid")
}

#[must_use]
pub fn session_manifest() -> PluginManifest {
    PluginManifest {
        id: PluginId::parse(SESSION_PLUGIN).expect("static plugin id is valid"),
        version: 1,
        execution: PluginExecution::Embedded,
        dependencies: Vec::new(),
        services: vec![
            ServiceContribution {
                role: phenix_core::ServiceRole::Terminal,
                service: session_service(),
                priority: 100,
                required_authority: Authority::default(),
            },
            ServiceContribution {
                role: phenix_core::ServiceRole::Terminal,
                service: session_mutation_service(),
                priority: 100,
                required_authority: Authority::default(),
            },
        ],
        resource_namespaces: vec![session_namespace()],
        maximum_authority: Authority::new([
            capability(PERSISTENCE_SCHEMA),
            capability(PERSISTENCE_READ),
            capability(PERSISTENCE_WRITE),
        ]),
    }
}

#[must_use]
pub fn session_component_manifest() -> ComponentManifest {
    ComponentManifest {
        id: ComponentId::parse(SESSION_COMPONENT).expect("static component id is valid"),
        owner: PluginId::parse(SESSION_PLUGIN).expect("static plugin id is valid"),
        imports: Vec::new(),
        exports: vec![
            ComponentExport {
                interface: SessionInterface::interface_id(),
                priority: 100,
                required_authority: Authority::default(),
            },
            ComponentExport {
                interface: SessionMutationInterface::interface_id(),
                priority: 100,
                required_authority: Authority::default(),
            },
        ],
        maximum_authority: session_manifest().maximum_authority,
    }
}

#[must_use]
pub fn session_factory() -> Box<dyn PluginInstance> {
    Box::new(SessionPlugin)
}

fn session_namespace() -> ResourceNamespace {
    ResourceNamespace::parse(SESSION_NAMESPACE).expect("static namespace is valid")
}

fn capability(value: &str) -> CapabilityId {
    CapabilityId::parse(value).expect("static capability is valid")
}

struct SessionPlugin;

impl PluginInstance for SessionPlugin {
    fn start(&mut self, host: &PluginHost<'_>) -> Result<(), String> {
        host.register_durable_schema(&DurableSchema::new(session_namespace(), 1))
            .map_err(|error| error.to_string())
    }

    fn invoke(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        if service == &session_service() {
            let command: SessionCommand =
                serde_json::from_slice(input).map_err(|error| error.to_string())?;
            let response = match command {
                SessionCommand::Create { id } => create_session(host, id)?,
                SessionCommand::Get { id } => SessionResponse::Session {
                    session: read_session(host, &id)?,
                },
                SessionCommand::List => SessionResponse::Sessions {
                    sessions: read_sessions(host)?,
                },
                SessionCommand::Continue { id, kind, content } => {
                    continue_session(host, &id, kind, content)?
                }
                SessionCommand::Inputs { id } => {
                    if read_session(host, &id)?.is_none() {
                        return Err(format!("unknown session: {id}"));
                    }
                    SessionResponse::Inputs {
                        inputs: read_inputs(host, &id)?,
                    }
                }
            };
            return serde_json::to_vec(&response).map_err(|error| error.to_string());
        }
        if service == &session_mutation_service() {
            let command: SessionMutationCommand =
                serde_json::from_slice(input).map_err(|error| error.to_string())?;
            let response = match command {
                SessionMutationCommand::PrepareCreate { id } => {
                    let (session, transaction) = prepare_create(host, id)?;
                    SessionMutationResponse::PreparedCreate {
                        session,
                        transaction,
                    }
                }
            };
            return serde_json::to_vec(&response).map_err(|error| error.to_string());
        }
        Err(format!("unsupported session service: {service}"))
    }
}

fn prepare_create(
    host: &PluginHost<'_>,
    id: String,
) -> Result<(SessionRecord, NamespaceTransaction), String> {
    if id.trim().is_empty() {
        return Err("session id must not be empty".into());
    }
    if read_session(host, &id)?.is_some() {
        return Err(format!("session already exists: {id}"));
    }

    let session = SessionRecord { id };
    let session_key = session_key(&session.id);
    let old_sessions = read_raw(host, ALL_SESSIONS_KEY)?;
    let mut sessions = decode_ids(old_sessions.as_deref())?;
    sessions.push(session.id.clone());
    sessions.sort();
    sessions.dedup();
    let transaction = NamespaceTransaction {
        owner: PluginId::parse(SESSION_PLUGIN).expect("static plugin id is valid"),
        namespace: session_namespace(),
        operations: vec![
            TransactionOp::AssertValue {
                key: session_key.clone(),
                expected: None,
            },
            TransactionOp::AssertValue {
                key: ALL_SESSIONS_KEY.into(),
                expected: old_sessions,
            },
            TransactionOp::Put {
                key: session_key,
                value: serde_json::to_vec(&session).map_err(|error| error.to_string())?,
            },
            TransactionOp::Put {
                key: ALL_SESSIONS_KEY.into(),
                value: serde_json::to_vec(&sessions).map_err(|error| error.to_string())?,
            },
        ],
    };
    Ok((session, transaction))
}

fn create_session(host: &PluginHost<'_>, id: String) -> Result<SessionResponse, String> {
    let (session, transaction) = prepare_create(host, id)?;
    host.transact_durable(&transaction.namespace, &transaction.operations)
        .map_err(|error| error.to_string())?;
    Ok(SessionResponse::Created { session })
}

fn continue_session(
    host: &PluginHost<'_>,
    id: &str,
    kind: SessionInputKind,
    content: Vec<u8>,
) -> Result<SessionResponse, String> {
    let session = read_session(host, id)?.ok_or_else(|| format!("unknown session: {id}"))?;
    let key = inputs_key(id);
    let old_inputs = read_raw(host, &key)?;
    let mut inputs = decode_inputs(old_inputs.as_deref())?;
    let input = SessionInput {
        sequence: u64::try_from(inputs.len())
            .map_err(|_| "session input sequence overflow".to_owned())?
            + 1,
        kind,
        content,
    };
    inputs.push(input.clone());
    host.transact_durable(
        &session_namespace(),
        &[
            TransactionOp::AssertValue {
                key: key.clone(),
                expected: old_inputs,
            },
            TransactionOp::Put {
                key,
                value: serde_json::to_vec(&inputs).map_err(|error| error.to_string())?,
            },
        ],
    )
    .map_err(|error| error.to_string())?;
    Ok(SessionResponse::Continued { session, input })
}

fn read_session(host: &PluginHost<'_>, id: &str) -> Result<Option<SessionRecord>, String> {
    read_raw(host, &session_key(id))?
        .map(|value| serde_json::from_slice(&value).map_err(|error| error.to_string()))
        .transpose()
}

fn read_sessions(host: &PluginHost<'_>) -> Result<Vec<SessionRecord>, String> {
    let ids = decode_ids(read_raw(host, ALL_SESSIONS_KEY)?.as_deref())?;
    ids.into_iter()
        .map(|id| read_session(host, &id)?.ok_or_else(|| format!("missing durable session: {id}")))
        .collect()
}

fn read_inputs(host: &PluginHost<'_>, id: &str) -> Result<Vec<SessionInput>, String> {
    decode_inputs(read_raw(host, &inputs_key(id))?.as_deref())
}

fn read_raw(host: &PluginHost<'_>, key: &str) -> Result<Option<Vec<u8>>, String> {
    host.read_durable(&session_namespace(), key)
        .map_err(|error| error.to_string())
}

fn decode_ids(value: Option<&[u8]>) -> Result<Vec<String>, String> {
    value
        .map(|value| serde_json::from_slice(value).map_err(|error| error.to_string()))
        .unwrap_or_else(|| Ok(Vec::new()))
}

fn decode_inputs(value: Option<&[u8]>) -> Result<Vec<SessionInput>, String> {
    value
        .map(|value| serde_json::from_slice(value).map_err(|error| error.to_string()))
        .unwrap_or_else(|| Ok(Vec::new()))
}

fn session_key(id: &str) -> String {
    format!("session/{id}")
}

fn inputs_key(id: &str) -> String {
    format!("inputs/{id}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_core::{Kernel, KernelConfig, LocalPersistence};
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn authority() -> Authority {
        session_manifest().maximum_authority
    }

    fn invoke(kernel: &mut Kernel, command: &SessionCommand) -> Result<SessionResponse, String> {
        let input = serde_json::to_vec(command).unwrap();
        let output = kernel
            .invoke(&session_service(), &input, &authority(), None)
            .map_err(|error| error.to_string())?;
        serde_json::from_slice(&output).map_err(|error| error.to_string())
    }

    fn kernel_with(path: &PathBuf) -> Kernel {
        let manifest = session_manifest();
        let plugin = manifest.id.clone();
        let persistence = LocalPersistence::open(path).unwrap();
        let mut kernel =
            Kernel::with_persistence(KernelConfig::new([manifest]).unwrap(), persistence);
        kernel
            .register_embedded_factory(plugin, session_factory)
            .unwrap();
        kernel.activate_all().unwrap();
        kernel
    }

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

    #[test]
    fn flat_sessions_and_ordered_inputs_are_durable_across_plugin_restart() {
        let path = temp_db("sessions");
        {
            let mut kernel = kernel_with(&path);
            invoke(&mut kernel, &SessionCommand::Create { id: "root".into() }).unwrap();
            for (kind, content) in [
                (SessionInputKind::Root, b"system".to_vec()),
                (SessionInputKind::User, b"hello".to_vec()),
            ] {
                invoke(
                    &mut kernel,
                    &SessionCommand::Continue {
                        id: "root".into(),
                        kind,
                        content,
                    },
                )
                .unwrap();
            }
        }

        let mut restored = kernel_with(&path);
        assert_eq!(
            invoke(&mut restored, &SessionCommand::List).unwrap(),
            SessionResponse::Sessions {
                sessions: vec![SessionRecord { id: "root".into() }],
            }
        );
        assert_eq!(
            invoke(&mut restored, &SessionCommand::Inputs { id: "root".into() },).unwrap(),
            SessionResponse::Inputs {
                inputs: vec![
                    SessionInput {
                        sequence: 1,
                        kind: SessionInputKind::Root,
                        content: b"system".to_vec(),
                    },
                    SessionInput {
                        sequence: 2,
                        kind: SessionInputKind::User,
                        content: b"hello".to_vec(),
                    },
                ],
            }
        );
        let _ = fs::remove_file(path);
    }
}
