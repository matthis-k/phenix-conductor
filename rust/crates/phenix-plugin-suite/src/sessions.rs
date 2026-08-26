use phenix_kernel::{
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: String,
    pub parent: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum SessionCommand {
    Create { id: String, parent: Option<String> },
    Get { id: String },
    Children { parent: Option<String> },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum SessionResponse {
    Created { session: SessionRecord },
    Session { session: Option<SessionRecord> },
    Children { sessions: Vec<SessionRecord> },
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
            SessionCommand::Children { parent } => SessionResponse::Children {
                sessions: read_children(host, parent.as_deref())?,
            },
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
    let old_children = host
        .read_durable(&session_namespace(), &children_key)
        .map_err(|error| error.to_string())?;
    let mut children = decode_children(old_children.as_deref())?;
    children.push(session.id.clone());
    children.sort();
    children.dedup();

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
            TransactionOp::Put {
                key: session_key,
                value: serde_json::to_vec(&session).map_err(|error| error.to_string())?,
            },
            TransactionOp::Put {
                key: children_key,
                value: serde_json::to_vec(&children).map_err(|error| error.to_string())?,
            },
        ],
    )
    .map_err(|error| error.to_string())?;

    Ok(SessionResponse::Created { session })
}

fn read_session(host: &PluginHost<'_>, id: &str) -> Result<Option<SessionRecord>, String> {
    let value = host
        .read_durable(&session_namespace(), &session_key(id))
        .map_err(|error| error.to_string())?;
    value
        .map(|value| serde_json::from_slice(&value).map_err(|error| error.to_string()))
        .transpose()
}

fn read_children(
    host: &PluginHost<'_>,
    parent: Option<&str>,
) -> Result<Vec<SessionRecord>, String> {
    let value = host
        .read_durable(&session_namespace(), &children_key(parent))
        .map_err(|error| error.to_string())?;
    let ids = decode_children(value.as_deref())?;
    ids.into_iter()
        .map(|id| {
            read_session(host, &id)?.ok_or_else(|| format!("missing durable child session: {id}"))
        })
        .collect()
}

fn decode_children(value: Option<&[u8]>) -> Result<Vec<String>, String> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_kernel::{Kernel, KernelConfig, LocalPersistence, PersistenceBackend};
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
    fn session_tree_is_durable_across_plugin_restart() {
        let path = temp_db("sessions");
        {
            let mut kernel = kernel_with(&path);
            assert!(matches!(
                invoke(
                    &mut kernel,
                    &SessionCommand::Create {
                        id: "root".into(),
                        parent: None,
                    },
                )
                .unwrap(),
                SessionResponse::Created { .. }
            ));
            invoke(
                &mut kernel,
                &SessionCommand::Create {
                    id: "child".into(),
                    parent: Some("root".into()),
                },
            )
            .unwrap();
        }

        let mut restored = kernel_with(&path);
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
