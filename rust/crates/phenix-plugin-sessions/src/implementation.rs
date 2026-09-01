use crate::{
    session_service, SessionCommand, SessionInput, SessionInputKind, SessionInterface,
    SessionRecord, SessionResponse,
};
use phenix_core::{
    Authority, Bytes, CapabilityId, ComponentExport, ComponentId, ComponentInterface,
    ComponentManifest, DurableSchema, NamespaceTransaction, PluginContext, PluginExecution,
    PluginHost, PluginId, PluginInstance, PluginManifest, ResourceNamespace, ServiceContribution,
    ServiceId, SessionId, TransactionOp,
};
use phenix_sdk::{
    session_mutation_service, SessionMutationCommand, SessionMutationInterface,
    SessionMutationResponse,
};

const SESSION_PLUGIN: &str = "phenix.sessions";
const SESSION_COMPONENT: &str = "phenix.sessions";
const SESSION_NAMESPACE: &str = "phenix.sessions.state";
const PERSISTENCE_SCHEMA: &str = "kernel.persistence.schema";
const PERSISTENCE_READ: &str = "kernel.persistence.read";
const PERSISTENCE_WRITE: &str = "kernel.persistence.write";
const ALL_SESSIONS_KEY: &str = "sessions/@all";

type SessionContext<'host, 'runtime> = PluginContext<'host, 'runtime, ()>;

fn context<'host, 'runtime>(host: &'host PluginHost<'runtime>) -> SessionContext<'host, 'runtime> {
    PluginContext::new(host, (), (), ())
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
                schema: SessionInterface::schema(),
                priority: 100,
                required_authority: Authority::default(),
            },
            ComponentExport {
                interface: SessionMutationInterface::interface_id(),
                schema: SessionMutationInterface::schema(),
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
        context(host)
            .kernel
            .register_durable_schema(&DurableSchema::new(session_namespace(), 1))
            .map_err(|error| error.to_string())
    }

    fn invoke(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        let context = context(host);
        if service == &session_service() {
            let interface = SessionInterface::interface_id();
            let command = context
                .kernel
                .decode_projected::<SessionCommand>(&interface, input)
                .map_err(|error| error.to_string())?;
            let response = handle_session(&context, command)?;
            return context
                .kernel
                .encode_value(&response)
                .map_err(|error| error.to_string());
        }
        if service == &session_mutation_service() {
            let interface = SessionMutationInterface::interface_id();
            let command = context
                .kernel
                .decode_projected::<SessionMutationCommand>(&interface, input)
                .map_err(|error| error.to_string())?;
            let response = handle_mutation(&context, command)?;
            return context
                .kernel
                .encode_value(&response)
                .map_err(|error| error.to_string());
        }
        Err(format!("unsupported session service: {service}"))
    }
}

fn handle_session(
    context: &SessionContext<'_, '_>,
    command: SessionCommand,
) -> Result<SessionResponse, String> {
    match command {
        SessionCommand::Create { id } => create_session(context, id),
        SessionCommand::Get { id } => Ok(SessionResponse::Session {
            session: read_session(context, &id)?,
        }),
        SessionCommand::List => Ok(SessionResponse::Sessions {
            sessions: read_sessions(context)?,
        }),
        SessionCommand::Continue { id, kind, content } => {
            continue_session(context, &id, kind, content)
        }
        SessionCommand::Inputs { id } => {
            if read_session(context, &id)?.is_none() {
                return Err(format!("unknown session: {id}"));
            }
            Ok(SessionResponse::Inputs {
                inputs: read_inputs(context, &id)?,
            })
        }
    }
}

fn handle_mutation(
    context: &SessionContext<'_, '_>,
    command: SessionMutationCommand,
) -> Result<SessionMutationResponse, String> {
    let SessionMutationCommand::PrepareCreate { id } = command;
    let (session, transaction) = prepare_create(context, id)?;
    Ok(SessionMutationResponse::PreparedCreate {
        session,
        transaction,
    })
}

fn prepare_create(
    context: &SessionContext<'_, '_>,
    id: SessionId,
) -> Result<(SessionRecord, NamespaceTransaction), String> {
    if read_session(context, &id)?.is_some() {
        return Err(format!("session already exists: {id}"));
    }

    let session = SessionRecord { id };
    let session_key = session_key(&session.id);
    let old_sessions = read_raw(context, ALL_SESSIONS_KEY)?;
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

fn create_session(
    context: &SessionContext<'_, '_>,
    id: SessionId,
) -> Result<SessionResponse, String> {
    let (session, transaction) = prepare_create(context, id)?;
    context
        .kernel
        .transact_durable(&transaction.namespace, &transaction.operations)
        .map_err(|error| error.to_string())?;
    Ok(SessionResponse::Created { session })
}

fn continue_session(
    context: &SessionContext<'_, '_>,
    id: &SessionId,
    kind: SessionInputKind,
    content: Bytes,
) -> Result<SessionResponse, String> {
    let session = read_session(context, id)?.ok_or_else(|| format!("unknown session: {id}"))?;
    let key = inputs_key(id);
    let old_inputs = read_raw(context, &key)?;
    let mut inputs = decode_inputs(old_inputs.as_deref())?;
    let input = SessionInput {
        sequence: u64::try_from(inputs.len())
            .map_err(|_| "session input sequence overflow".to_owned())?
            + 1,
        kind,
        content,
    };
    inputs.push(input.clone());
    context
        .kernel
        .transact_durable(
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

fn read_session(
    context: &SessionContext<'_, '_>,
    id: &SessionId,
) -> Result<Option<SessionRecord>, String> {
    read_raw(context, &session_key(id))?
        .map(|value| serde_json::from_slice(&value).map_err(|error| error.to_string()))
        .transpose()
}

fn read_sessions(context: &SessionContext<'_, '_>) -> Result<Vec<SessionRecord>, String> {
    let ids = decode_ids(read_raw(context, ALL_SESSIONS_KEY)?.as_deref())?;
    ids.into_iter()
        .map(|id| {
            read_session(context, &id)?.ok_or_else(|| format!("missing durable session: {id}"))
        })
        .collect()
}

fn read_inputs(
    context: &SessionContext<'_, '_>,
    id: &SessionId,
) -> Result<Vec<SessionInput>, String> {
    decode_inputs(read_raw(context, &inputs_key(id))?.as_deref())
}

fn read_raw(context: &SessionContext<'_, '_>, key: &str) -> Result<Option<Vec<u8>>, String> {
    context
        .kernel
        .read_durable(&session_namespace(), key)
        .map_err(|error| error.to_string())
}

fn decode_ids(value: Option<&[u8]>) -> Result<Vec<SessionId>, String> {
    value
        .map(|value| serde_json::from_slice(value).map_err(|error| error.to_string()))
        .unwrap_or_else(|| Ok(Vec::new()))
}

fn decode_inputs(value: Option<&[u8]>) -> Result<Vec<SessionInput>, String> {
    value
        .map(|value| serde_json::from_slice(value).map_err(|error| error.to_string()))
        .unwrap_or_else(|| Ok(Vec::new()))
}

fn session_key(id: &SessionId) -> String {
    format!("session/{id}")
}

fn inputs_key(id: &SessionId) -> String {
    format!("inputs/{id}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_core::{Kernel, KernelConfig, LocalPersistence, PhenixValue, Project};
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn authority() -> Authority {
        session_manifest().maximum_authority
    }

    fn invoke(kernel: &mut Kernel, command: &SessionCommand) -> Result<SessionResponse, String> {
        let input = serde_json::to_vec(&PhenixValue::from(command)).unwrap();
        let output = kernel
            .invoke(&session_service(), &input, &authority(), None)
            .map_err(|error| error.to_string())?;
        let output: PhenixValue =
            serde_json::from_slice(&output).map_err(|error| error.to_string())?;
        SessionResponse::try_from(Project(&output)).map_err(|error| error.to_string())
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
        let root = SessionId::parse("root").unwrap();
        {
            let mut kernel = kernel_with(&path);
            invoke(&mut kernel, &SessionCommand::Create { id: root.clone() }).unwrap();
            for (kind, content) in [
                (SessionInputKind::Root, b"system".to_vec()),
                (SessionInputKind::User, b"hello".to_vec()),
            ] {
                invoke(
                    &mut kernel,
                    &SessionCommand::Continue {
                        id: root.clone(),
                        kind,
                        content: content.into(),
                    },
                )
                .unwrap();
            }
        }

        let mut restored = kernel_with(&path);
        assert_eq!(
            invoke(&mut restored, &SessionCommand::List).unwrap(),
            SessionResponse::Sessions {
                sessions: vec![SessionRecord { id: root.clone() }],
            }
        );
        assert_eq!(
            invoke(&mut restored, &SessionCommand::Inputs { id: root },).unwrap(),
            SessionResponse::Inputs {
                inputs: vec![
                    SessionInput {
                        sequence: 1,
                        kind: SessionInputKind::Root,
                        content: b"system".to_vec().into(),
                    },
                    SessionInput {
                        sequence: 2,
                        kind: SessionInputKind::User,
                        content: b"hello".to_vec().into(),
                    },
                ],
            }
        );
        let _ = fs::remove_file(path);
    }
}
