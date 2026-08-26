use phenix_core::{
    Authority, CapabilityId, DurableSchema, PluginExecution, PluginHost, PluginId, PluginInstance,
    PluginManifest, ResourceNamespace, ServiceContribution, ServiceId, TransactionOp,
};
use serde::{Deserialize, Serialize};

pub const SESSION_SERVICE: &str = "phenix.sessions@1";
const SESSION_PLUGIN: &str = "phenix.sessions";
const SESSION_NAMESPACE: &str = "phenix.sessions.state";
const PERSISTENCE_SCHEMA: &str = "kernel.persistence.schema";
const PERSISTENCE_READ: &str = "kernel.persistence.read";
const PERSISTENCE_WRITE: &str = "kernel.persistence.write";
const ALL_SESSIONS_KEY: &str = "sessions/@all";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: String,
    pub parent: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionInputKind {
    User,
    Root,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionInput {
    pub sequence: u64,
    pub kind: SessionInputKind,
    pub content: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum SessionCommand {
    Create {
        id: String,
        parent: Option<String>,
    },
    Get {
        id: String,
    },
    List,
    Children {
        parent: Option<String>,
    },
    Continue {
        id: String,
        kind: SessionInputKind,
        content: Vec<u8>,
    },
    Inputs {
        id: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum SessionResponse {
    Created {
        session: SessionRecord,
    },
    Session {
        session: Option<SessionRecord>,
    },
    Sessions {
        sessions: Vec<SessionRecord>,
    },
    Children {
        sessions: Vec<SessionRecord>,
    },
    Continued {
        session: SessionRecord,
        input: SessionInput,
    },
    Inputs {
        inputs: Vec<SessionInput>,
    },
}

#[must_use]
pub fn session_manifest() -> PluginManifest {
    PluginManifest {
        id: PluginId::parse(SESSION_PLUGIN).expect("static plugin id is valid"),
        version: 1,
        execution: PluginExecution::Embedded,
        dependencies: Vec::new(),
        services: vec![ServiceContribution {
            service: session_service(),
            priority: 100,
            required_authority: Authority::default(),
        }],
        resource_namespaces: vec![session_namespace()],
        maximum_authority: Authority::new([
            capability(PERSISTENCE_SCHEMA),
            capability(PERSISTENCE_READ),
            capability(PERSISTENCE_WRITE),
        ]),
    }
}

#[must_use]
pub fn session_factory() -> Box<dyn PluginInstance> {
    Box::new(SessionPlugin)
}

#[must_use]
pub fn session_service() -> ServiceId {
    ServiceId::parse(SESSION_SERVICE).expect("static service id is valid")
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
        if service != &session_service() {
            return Err(format!("unsupported session service: {service}"));
        }
        let command: SessionCommand =
            serde_json::from_slice(input).map_err(|error| error.to_string())?;
        let response = match command {
            SessionCommand::Create { id, parent } => create_session(host, id, parent)?,
            SessionCommand::Get { id } => SessionResponse::Session {
                session: read_session(host, &id)?,
            },
            SessionCommand::List => SessionResponse::Sessions {
                sessions: read_sessions(host)?,
            },
            SessionCommand::Children { parent } => SessionResponse::Children {
                sessions: read_children(host, parent.as_deref())?,
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
        serde_json::to_vec(&response).map_err(|error| error.to_string())
    }
}

fn create_session(
    host: &PluginHost<'_>,
    id: String,
    parent: Option<String>,
) -> Result<SessionResponse, String> {
    if id.trim().is_empty() {
        return Err("session id must not be empty".into());
    }
    if read_session(host, &id)?.is_some() {
        return Err(format!("session already exists: {id}"));
    }
    if let Some(parent_id) = parent.as_deref() {
        if read_session(host, parent_id)?.is_none() {
            return Err(format!("unknown parent session: {parent_id}"));
        }
    }

    let session = SessionRecord { id, parent };
    let session_key = session_key(&session.id);
    let children_key = children_key(session.parent.as_deref());
    let old_children = read_raw(host, &children_key)?;
    let mut children = decode_ids(old_children.as_deref())?;
    children.push(session.id.clone());
    children.sort();
    children.dedup();

    let old_sessions = read_raw(host, ALL_SESSIONS_KEY)?;
    let mut sessions = decode_ids(old_sessions.as_deref())?;
    sessions.push(session.id.clone());
    sessions.sort();
    sessions.dedup();

    host.transact_durable(
        &session_namespace(),
        &[
            TransactionOp::AssertValue {
                key: session_key.clone(),
                expected: None,
            },
            TransactionOp::AssertValue {
                key: children_key.clone(),
                expected: old_children,
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
                key: children_key,
                value: serde_json::to_vec(&children).map_err(|error| error.to_string())?,
            },
            TransactionOp::Put {
                key: ALL_SESSIONS_KEY.into(),
                value: serde_json::to_vec(&sessions).map_err(|error| error.to_string())?,
            },
        ],
    )
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

fn read_children(
    host: &PluginHost<'_>,
    parent: Option<&str>,
) -> Result<Vec<SessionRecord>, String> {
    let ids = decode_ids(read_raw(host, &children_key(parent))?.as_deref())?;
    ids.into_iter()
        .map(|id| {
            read_session(host, &id)?.ok_or_else(|| format!("missing durable child session: {id}"))
        })
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

fn children_key(parent: Option<&str>) -> String {
    match parent {
        Some(parent) => format!("children/{parent}"),
        None => "children/@root".into(),
    }
}

fn inputs_key(id: &str) -> String {
    format!("inputs/{id}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_core::{Kernel, KernelConfig, LocalPersistence, PersistenceBackend};
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
    fn session_tree_and_ordered_inputs_are_durable_across_plugin_restart() {
        let path = temp_db("sessions");
        {
            let mut kernel = kernel_with(&path);
            invoke(
                &mut kernel,
                &SessionCommand::Create {
                    id: "root".into(),
                    parent: None,
                },
            )
            .unwrap();
            invoke(
                &mut kernel,
                &SessionCommand::Create {
                    id: "child".into(),
                    parent: Some("root".into()),
                },
            )
            .unwrap();
            for (kind, content) in [
                (SessionInputKind::Root, b"system".to_vec()),
                (SessionInputKind::User, b"hello".to_vec()),
            ] {
                invoke(
                    &mut kernel,
                    &SessionCommand::Continue {
                        id: "child".into(),
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
                sessions: vec![
                    SessionRecord {
                        id: "child".into(),
                        parent: Some("root".into()),
                    },
                    SessionRecord {
                        id: "root".into(),
                        parent: None,
                    },
                ],
            }
        );
        assert_eq!(
            invoke(
                &mut restored,
                &SessionCommand::Children {
                    parent: Some("root".into()),
                },
            )
            .unwrap(),
            SessionResponse::Children {
                sessions: vec![SessionRecord {
                    id: "child".into(),
                    parent: Some("root".into()),
                }],
            }
        );
        assert_eq!(
            invoke(
                &mut restored,
                &SessionCommand::Inputs { id: "child".into() },
            )
            .unwrap(),
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

    #[test]
    fn child_session_and_lineage_edge_commit_in_one_namespace_transaction() {
        let path = temp_db("session-lineage");
        let mut kernel = kernel_with(&path);
        invoke(
            &mut kernel,
            &SessionCommand::Create {
                id: "root".into(),
                parent: None,
            },
        )
        .unwrap();
        invoke(
            &mut kernel,
            &SessionCommand::Create {
                id: "child".into(),
                parent: Some("root".into()),
            },
        )
        .unwrap();
        assert!(matches!(
            invoke(&mut kernel, &SessionCommand::Get { id: "child".into() },).unwrap(),
            SessionResponse::Session { session: Some(_) }
        ));
        assert!(matches!(
            invoke(
                &mut kernel,
                &SessionCommand::Children {
                    parent: Some("root".into()),
                },
            )
            .unwrap(),
            SessionResponse::Children { sessions } if sessions.len() == 1
        ));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn incompatible_session_schema_fails_activation() {
        let path = temp_db("session-schema");
        let manifest = session_manifest();
        let plugin = manifest.id.clone();
        let namespace = session_namespace();
        let mut persistence = LocalPersistence::open(&path).unwrap();
        persistence
            .register_schema(&plugin, &DurableSchema::new(namespace, 2))
            .unwrap();
        let mut kernel =
            Kernel::with_persistence(KernelConfig::new([manifest]).unwrap(), persistence);
        kernel
            .register_embedded_factory(plugin, session_factory)
            .unwrap();
        assert!(kernel.activate_all().is_err());
        let _ = fs::remove_file(path);
    }
}
