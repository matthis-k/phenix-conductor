use phenix_kernel::{
    Authority, CapabilityId, DurableSchema, PluginExecution, PluginHost, PluginId, PluginInstance,
    PluginManifest, ResourceNamespace, ServiceContribution, ServiceId, TransactionOp,
};
use serde::{Deserialize, Serialize};

pub const ARTIFACT_SERVICE: &str = "phenix.artifacts@1";
const ARTIFACT_PLUGIN: &str = "phenix.artifacts";
const ARTIFACT_NAMESPACE: &str = "phenix.artifacts.state";
const PERSISTENCE_SCHEMA: &str = "kernel.persistence.schema";
const PERSISTENCE_READ: &str = "kernel.persistence.read";
const PERSISTENCE_WRITE: &str = "kernel.persistence.write";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ArtifactRecord {
    pub id: String,
    pub content_identity: String,
    pub content: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum ArtifactCommand {
    Store {
        id: String,
        content_identity: String,
        content: Vec<u8>,
    },
    Read {
        id: String,
        content_identity: String,
    },
    Invalidate {
        id: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum ArtifactResponse {
    Stored {
        artifact: ArtifactRecord,
        reused: bool,
    },
    Read {
        artifact: Option<ArtifactRecord>,
    },
    Invalidated {
        removed: bool,
    },
}

#[must_use]
pub fn artifact_manifest() -> PluginManifest {
    PluginManifest {
        id: PluginId::parse(ARTIFACT_PLUGIN).expect("static plugin id is valid"),
        version: 1,
        execution: PluginExecution::Embedded,
        dependencies: Vec::new(),
        services: vec![ServiceContribution {
            service: artifact_service(),
            priority: 100,
            required_authority: Authority::default(),
        }],
        resource_namespaces: vec![artifact_namespace()],
        maximum_authority: Authority::new([
            capability(PERSISTENCE_SCHEMA),
            capability(PERSISTENCE_READ),
            capability(PERSISTENCE_WRITE),
        ]),
    }
}

#[must_use]
pub fn artifact_factory() -> Box<dyn PluginInstance> {
    Box::new(ArtifactPlugin)
}

#[must_use]
pub fn artifact_service() -> ServiceId {
    ServiceId::parse(ARTIFACT_SERVICE).expect("static service id is valid")
}

fn artifact_namespace() -> ResourceNamespace {
    ResourceNamespace::parse(ARTIFACT_NAMESPACE).expect("static namespace is valid")
}

fn capability(value: &str) -> CapabilityId {
    CapabilityId::parse(value).expect("static capability is valid")
}

struct ArtifactPlugin;

impl PluginInstance for ArtifactPlugin {
    fn start(&mut self, host: &PluginHost<'_>) -> Result<(), String> {
        host.register_durable_schema(&DurableSchema::new(artifact_namespace(), 1))
            .map_err(|error| error.to_string())
    }

    fn invoke(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        if service != &artifact_service() {
            return Err(format!("unsupported artifact service: {service}"));
        }
        let command: ArtifactCommand =
            serde_json::from_slice(input).map_err(|error| error.to_string())?;
        let response = match command {
            ArtifactCommand::Store {
                id,
                content_identity,
                content,
            } => store(host, id, content_identity, content)?,
            ArtifactCommand::Read {
                id,
                content_identity,
            } => ArtifactResponse::Read {
                artifact: read_exact(host, &id, &content_identity)?,
            },
            ArtifactCommand::Invalidate { id } => invalidate(host, &id)?,
        };
        serde_json::to_vec(&response).map_err(|error| error.to_string())
    }
}

fn store(
    host: &PluginHost<'_>,
    id: String,
    content_identity: String,
    content: Vec<u8>,
) -> Result<ArtifactResponse, String> {
    if id.trim().is_empty() || content_identity.trim().is_empty() {
        return Err("artifact id and content identity must not be empty".into());
    }
    if let Some(existing) = read_record(host, &id)? {
        if existing.content_identity == content_identity {
            if existing.content != content {
                return Err(format!(
                    "artifact content identity changed bytes: {content_identity}"
                ));
            }
            return Ok(ArtifactResponse::Stored {
                artifact: existing,
                reused: true,
            });
        }
        return Err(format!(
            "artifact id {} is immutable at content identity {}",
            existing.id, existing.content_identity
        ));
    }

    let artifact = ArtifactRecord {
        id,
        content_identity,
        content,
    };
    host.transact_durable(
        &artifact_namespace(),
        &[
            TransactionOp::AssertValue {
                key: artifact_key(&artifact.id),
                expected: None,
            },
            TransactionOp::Put {
                key: artifact_key(&artifact.id),
                value: serde_json::to_vec(&artifact).map_err(|error| error.to_string())?,
            },
        ],
    )
    .map_err(|error| error.to_string())?;
    Ok(ArtifactResponse::Stored {
        artifact,
        reused: false,
    })
}

fn read_record(host: &PluginHost<'_>, id: &str) -> Result<Option<ArtifactRecord>, String> {
    host.read_durable(&artifact_namespace(), &artifact_key(id))
        .map_err(|error| error.to_string())?
        .map(|value| serde_json::from_slice(&value).map_err(|error| error.to_string()))
        .transpose()
}

fn read_exact(
    host: &PluginHost<'_>,
    id: &str,
    content_identity: &str,
) -> Result<Option<ArtifactRecord>, String> {
    Ok(read_record(host, id)?.filter(|artifact| artifact.content_identity == content_identity))
}

fn invalidate(host: &PluginHost<'_>, id: &str) -> Result<ArtifactResponse, String> {
    let removed = read_record(host, id)?.is_some();
    if removed {
        host.transact_durable(
            &artifact_namespace(),
            &[TransactionOp::Delete {
                key: artifact_key(id),
            }],
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(ArtifactResponse::Invalidated { removed })
}

fn artifact_key(id: &str) -> String {
    format!("artifact/{id}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_kernel::{Kernel, KernelConfig};

    fn authority() -> Authority {
        artifact_manifest().maximum_authority
    }

    fn kernel() -> Kernel {
        let manifest = artifact_manifest();
        let plugin = manifest.id.clone();
        let mut kernel = Kernel::new(KernelConfig::new([manifest]).unwrap());
        kernel
            .register_embedded_factory(plugin, artifact_factory)
            .unwrap();
        kernel.activate_all().unwrap();
        kernel
    }

    fn invoke(kernel: &mut Kernel, command: ArtifactCommand) -> ArtifactResponse {
        let input = serde_json::to_vec(&command).unwrap();
        let output = kernel
            .invoke(&artifact_service(), &input, &authority(), None)
            .unwrap();
        serde_json::from_slice(&output).unwrap()
    }

    #[test]
    fn exact_reader_output_is_reused_until_invalidated() {
        let mut kernel = kernel();
        let command = ArtifactCommand::Store {
            id: "read:src/lib.rs".into(),
            content_identity: "sha256:a".into(),
            content: b"first".to_vec(),
        };
        let first = invoke(&mut kernel, command.clone());
        assert!(matches!(
            first,
            ArtifactResponse::Stored { reused: false, .. }
        ));

        let reused = invoke(&mut kernel, command);
        assert!(matches!(
            reused,
            ArtifactResponse::Stored { reused: true, .. }
        ));

        assert_eq!(
            invoke(
                &mut kernel,
                ArtifactCommand::Read {
                    id: "read:src/lib.rs".into(),
                    content_identity: "sha256:b".into(),
                },
            ),
            ArtifactResponse::Read { artifact: None }
        );

        assert_eq!(
            invoke(
                &mut kernel,
                ArtifactCommand::Invalidate {
                    id: "read:src/lib.rs".into(),
                },
            ),
            ArtifactResponse::Invalidated { removed: true }
        );
    }

    #[test]
    fn immutable_artifact_identity_cannot_be_reused_for_other_bytes_or_revision() {
        let mut kernel = kernel();
        invoke(
            &mut kernel,
            ArtifactCommand::Store {
                id: "artifact-1".into(),
                content_identity: "sha256:a".into(),
                content: b"first".to_vec(),
            },
        );

        for command in [
            ArtifactCommand::Store {
                id: "artifact-1".into(),
                content_identity: "sha256:a".into(),
                content: b"different".to_vec(),
            },
            ArtifactCommand::Store {
                id: "artifact-1".into(),
                content_identity: "sha256:b".into(),
                content: b"first".to_vec(),
            },
        ] {
            let input = serde_json::to_vec(&command).unwrap();
            assert!(kernel
                .invoke(&artifact_service(), &input, &authority(), None)
                .is_err());
        }
    }
}
