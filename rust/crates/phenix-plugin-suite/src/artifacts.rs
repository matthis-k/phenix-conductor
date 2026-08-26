use phenix_kernel::{
    Authority, CapabilityId, DurableSchema, PluginExecution, PluginHost, PluginId, PluginInstance,
    PluginManifest, ResourceNamespace, ServiceContribution, ServiceId, TransactionOp,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const ARTIFACT_SERVICE: &str = "phenix.artifacts@1";
const ARTIFACT_PLUGIN: &str = "phenix.artifacts";
const ARTIFACT_NAMESPACE: &str = "phenix.artifacts.state";
const PERSISTENCE_SCHEMA: &str = "kernel.persistence.schema";
const PERSISTENCE_READ: &str = "kernel.persistence.read";
const PERSISTENCE_WRITE: &str = "kernel.persistence.write";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ArtifactProvenance {
    pub producer: String,
    pub provider_identity: Option<String>,
    pub configuration_identity: Option<String>,
    pub source_observations: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ArtifactRecord {
    pub id: String,
    pub content_identity: String,
    pub content: Vec<u8>,
    pub provenance: ArtifactProvenance,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NormalizedReadRequest {
    pub resource: String,
    pub parameters: BTreeMap<String, String>,
    pub presentation_hint: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReadProviderIdentity {
    pub provider: String,
    pub contract_version: String,
    pub implementation_identity: String,
    pub configuration_identity: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReadResultRecord {
    pub id: String,
    pub request_identity: String,
    pub provider: ReadProviderIdentity,
    pub artifact_id: String,
    pub content_identity: String,
    pub dependencies: BTreeMap<String, String>,
    pub invocation_provenance: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RevalidationVerdict {
    StillValid,
    Invalid,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RevalidationRecord {
    pub result_id: String,
    pub provider: ReadProviderIdentity,
    pub current_dependencies: BTreeMap<String, String>,
    pub verdict: RevalidationVerdict,
    pub provenance: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum ArtifactCommand {
    Store {
        content: Vec<u8>,
        provenance: ArtifactProvenance,
    },
    Get {
        id: String,
        content_identity: String,
    },
    RecordRead {
        request: NormalizedReadRequest,
        provider: ReadProviderIdentity,
        invocation_provenance: String,
        content: Vec<u8>,
        dependencies: BTreeMap<String, String>,
    },
    LookupRead {
        request: NormalizedReadRequest,
        provider: ReadProviderIdentity,
        dependencies: BTreeMap<String, String>,
    },
    Revalidate {
        result_id: String,
        provider: ReadProviderIdentity,
        current_dependencies: BTreeMap<String, String>,
        verdict: RevalidationVerdict,
        provenance: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum ArtifactResponse {
    Stored {
        artifact: ArtifactRecord,
        reused: bool,
    },
    Artifact {
        artifact: Option<ArtifactRecord>,
    },
    ReadRecorded {
        result: ReadResultRecord,
        artifact: ArtifactRecord,
        reused: bool,
    },
    ReadLookup {
        result: Option<ReadResultRecord>,
    },
    Revalidated {
        record: RevalidationRecord,
        reusable: bool,
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
                content,
                provenance,
            } => store(host, content, provenance)?,
            ArtifactCommand::Get {
                id,
                content_identity,
            } => ArtifactResponse::Artifact {
                artifact: read_exact(host, &id, &content_identity)?,
            },
            ArtifactCommand::RecordRead {
                request,
                provider,
                invocation_provenance,
                content,
                dependencies,
            } => record_read(
                host,
                request,
                provider,
                invocation_provenance,
                content,
                dependencies,
            )?,
            ArtifactCommand::LookupRead {
                request,
                provider,
                dependencies,
            } => ArtifactResponse::ReadLookup {
                result: lookup_read(host, &request, &provider, &dependencies)?,
            },
            ArtifactCommand::Revalidate {
                result_id,
                provider,
                current_dependencies,
                verdict,
                provenance,
            } => revalidate(
                host,
                result_id,
                provider,
                current_dependencies,
                verdict,
                provenance,
            )?,
        };
        serde_json::to_vec(&response).map_err(|error| error.to_string())
    }
}

fn store(
    host: &PluginHost<'_>,
    content: Vec<u8>,
    provenance: ArtifactProvenance,
) -> Result<ArtifactResponse, String> {
    let content_identity = exact_content_identity(&content);
    let id = format!("artifact:{content_identity}");
    if let Some(existing) = read_record(host, &id)? {
        if existing.content_identity != content_identity || existing.content != content {
            return Err(format!("artifact identity collision: {id}"));
        }
        return Ok(ArtifactResponse::Stored {
            artifact: existing,
            reused: true,
        });
    }

    let artifact = ArtifactRecord {
        id,
        content_identity,
        content,
        provenance,
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

fn record_read(
    host: &PluginHost<'_>,
    request: NormalizedReadRequest,
    provider: ReadProviderIdentity,
    invocation_provenance: String,
    content: Vec<u8>,
    dependencies: BTreeMap<String, String>,
) -> Result<ArtifactResponse, String> {
    if let Some(result) = lookup_read(host, &request, &provider, &dependencies)? {
        let artifact = read_exact(host, &result.artifact_id, &result.content_identity)?
            .ok_or_else(|| format!("missing artifact for reusable read result: {}", result.id))?;
        return Ok(ArtifactResponse::ReadRecorded {
            result,
            artifact,
            reused: true,
        });
    }

    let artifact = match store(
        host,
        content,
        ArtifactProvenance {
            producer: invocation_provenance.clone(),
            provider_identity: Some(provider.provider.clone()),
            configuration_identity: Some(provider.configuration_identity.clone()),
            source_observations: dependencies.clone(),
        },
    )? {
        ArtifactResponse::Stored { artifact, .. } => artifact,
        _ => unreachable!("store returns stored artifact"),
    };
    let request_identity = normalized_request_identity(&request)?;
    let result = ReadResultRecord {
        id: read_result_identity(
            &request_identity,
            &provider,
            &artifact.content_identity,
            &dependencies,
        )?,
        request_identity: request_identity.clone(),
        provider: provider.clone(),
        artifact_id: artifact.id.clone(),
        content_identity: artifact.content_identity.clone(),
        dependencies,
        invocation_provenance,
    };
    let result_key = read_result_key(&result.id);
    let existing = host
        .read_durable(&artifact_namespace(), &result_key)
        .map_err(|error| error.to_string())?;
    if let Some(existing) = existing {
        let existing: ReadResultRecord =
            serde_json::from_slice(&existing).map_err(|error| error.to_string())?;
        if existing != result {
            return Err(format!("read result identity collision: {}", result.id));
        }
    } else {
        host.transact_durable(
            &artifact_namespace(),
            &[
                TransactionOp::AssertValue {
                    key: result_key.clone(),
                    expected: None,
                },
                TransactionOp::Put {
                    key: result_key,
                    value: serde_json::to_vec(&result).map_err(|error| error.to_string())?,
                },
                TransactionOp::Put {
                    key: read_index_key(&request_identity, &provider)?,
                    value: serde_json::to_vec(&result.id).map_err(|error| error.to_string())?,
                },
            ],
        )
        .map_err(|error| error.to_string())?;
    }

    Ok(ArtifactResponse::ReadRecorded {
        result,
        artifact,
        reused: false,
    })
}

fn lookup_read(
    host: &PluginHost<'_>,
    request: &NormalizedReadRequest,
    provider: &ReadProviderIdentity,
    dependencies: &BTreeMap<String, String>,
) -> Result<Option<ReadResultRecord>, String> {
    let request_identity = normalized_request_identity(request)?;
    let Some(result_id) = host
        .read_durable(
            &artifact_namespace(),
            &read_index_key(&request_identity, provider)?,
        )
        .map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };
    let result_id: String =
        serde_json::from_slice(&result_id).map_err(|error| error.to_string())?;
    let Some(result) = host
        .read_durable(&artifact_namespace(), &read_result_key(&result_id))
        .map_err(|error| error.to_string())?
    else {
        return Err(format!("missing indexed read result: {result_id}"));
    };
    let result: ReadResultRecord =
        serde_json::from_slice(&result).map_err(|error| error.to_string())?;
    if result.request_identity != request_identity
        || &result.provider != provider
        || &result.dependencies != dependencies
        || read_exact(host, &result.artifact_id, &result.content_identity)?.is_none()
    {
        return Ok(None);
    }
    Ok(Some(result))
}

fn revalidate(
    host: &PluginHost<'_>,
    result_id: String,
    provider: ReadProviderIdentity,
    current_dependencies: BTreeMap<String, String>,
    verdict: RevalidationVerdict,
    provenance: String,
) -> Result<ArtifactResponse, String> {
    let result = host
        .read_durable(&artifact_namespace(), &read_result_key(&result_id))
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("unknown read result: {result_id}"))?;
    let result: ReadResultRecord =
        serde_json::from_slice(&result).map_err(|error| error.to_string())?;
    if read_exact(host, &result.artifact_id, &result.content_identity)?.is_none() {
        return Err(format!("missing artifact for read result: {result_id}"));
    }
    let record = RevalidationRecord {
        result_id,
        provider,
        current_dependencies,
        verdict,
        provenance,
    };
    host.transact_durable(
        &artifact_namespace(),
        &[TransactionOp::Put {
            key: revalidation_key(&record)?,
            value: serde_json::to_vec(&record).map_err(|error| error.to_string())?,
        }],
    )
    .map_err(|error| error.to_string())?;
    Ok(ArtifactResponse::Revalidated {
        reusable: verdict == RevalidationVerdict::StillValid,
        record,
    })
}

fn normalized_request_identity(request: &NormalizedReadRequest) -> Result<String, String> {
    if request.resource.trim().is_empty() {
        return Err("read resource must not be empty".into());
    }
    serde_json::to_string(&(&request.resource, &request.parameters))
        .map(|identity| format!("request:{identity}"))
        .map_err(|error| error.to_string())
}

fn read_result_identity(
    request_identity: &str,
    provider: &ReadProviderIdentity,
    content_identity: &str,
    dependencies: &BTreeMap<String, String>,
) -> Result<String, String> {
    serde_json::to_string(&(request_identity, provider, content_identity, dependencies))
        .map(|identity| format!("read:{identity}"))
        .map_err(|error| error.to_string())
}

fn read_index_key(
    request_identity: &str,
    provider: &ReadProviderIdentity,
) -> Result<String, String> {
    serde_json::to_string(&(request_identity, provider))
        .map(|identity| format!("read-index/{identity}"))
        .map_err(|error| error.to_string())
}

fn revalidation_key(record: &RevalidationRecord) -> Result<String, String> {
    serde_json::to_string(record)
        .map(|identity| format!("revalidation/{identity}"))
        .map_err(|error| error.to_string())
}

fn exact_content_identity(content: &[u8]) -> String {
    let mut encoded = String::with_capacity(content.len() * 2);
    for byte in content {
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("writing to string cannot fail");
    }
    format!("exact:{}:{encoded}", content.len())
}

fn artifact_key(id: &str) -> String {
    format!("artifact/{id}")
}

fn read_result_key(id: &str) -> String {
    format!("read-result/{id}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_kernel::{Kernel, KernelConfig, LocalPersistence};
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

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

    fn kernel_with(path: &PathBuf) -> Kernel {
        let manifest = artifact_manifest();
        let plugin = manifest.id.clone();
        let persistence = LocalPersistence::open(path).unwrap();
        let mut kernel = Kernel::with_persistence(KernelConfig::new([manifest]).unwrap(), persistence);
        kernel
            .register_embedded_factory(plugin, artifact_factory)
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

    fn invoke(kernel: &mut Kernel, command: ArtifactCommand) -> ArtifactResponse {
        let input = serde_json::to_vec(&command).unwrap();
        let output = kernel
            .invoke(&artifact_service(), &input, &authority(), None)
            .unwrap();
        serde_json::from_slice(&output).unwrap()
    }

    fn provenance() -> ArtifactProvenance {
        ArtifactProvenance {
            producer: "fixture".into(),
            provider_identity: Some("reader".into()),
            configuration_identity: Some("config-1".into()),
            source_observations: BTreeMap::new(),
        }
    }

    fn request(presentation_hint: &str, mode: &str) -> NormalizedReadRequest {
        NormalizedReadRequest {
            resource: "src/lib.rs".into(),
            parameters: BTreeMap::from([("mode".into(), mode.into())]),
            presentation_hint: Some(presentation_hint.into()),
        }
    }

    fn provider() -> ReadProviderIdentity {
        ReadProviderIdentity {
            provider: "phenix.reader".into(),
            contract_version: "1".into(),
            implementation_identity: "reader-v1".into(),
            configuration_identity: "config-1".into(),
        }
    }

    fn dependencies(version: &str) -> BTreeMap<String, String> {
        BTreeMap::from([("file:src/lib.rs".into(), version.into())])
    }

    #[test]
    fn content_addressed_artifact_is_immutable_and_survives_restart() {
        let path = temp_db("artifacts");
        let stored = {
            let mut kernel = kernel_with(&path);
            invoke(
                &mut kernel,
                ArtifactCommand::Store {
                    content: b"content".to_vec(),
                    provenance: provenance(),
                },
            )
        };
        let ArtifactResponse::Stored {
            artifact,
            reused: false,
        } = stored
        else {
            panic!("unexpected artifact response")
        };

        let mut restored = kernel_with(&path);
        assert_eq!(
            invoke(
                &mut restored,
                ArtifactCommand::Get {
                    id: artifact.id.clone(),
                    content_identity: artifact.content_identity.clone(),
                },
            ),
            ArtifactResponse::Artifact {
                artifact: Some(artifact),
            }
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn equivalent_read_reuses_result_and_ignores_presentation_wording() {
        let mut kernel = kernel();
        let first = invoke(
            &mut kernel,
            ArtifactCommand::RecordRead {
                request: request("please inspect this", "text"),
                provider: provider(),
                invocation_provenance: "invocation-1".into(),
                content: b"first".to_vec(),
                dependencies: dependencies("v1"),
            },
        );
        let ArtifactResponse::ReadRecorded {
            result: first_result,
            artifact: first_artifact,
            reused: false,
        } = first
        else {
            panic!("unexpected first read response")
        };

        let second = invoke(
            &mut kernel,
            ArtifactCommand::RecordRead {
                request: request("different prompt wording", "text"),
                provider: provider(),
                invocation_provenance: "invocation-2".into(),
                content: b"should not replace prior content".to_vec(),
                dependencies: dependencies("v1"),
            },
        );
        assert!(matches!(
            second,
            ArtifactResponse::ReadRecorded {
                result,
                artifact,
                reused: true,
            } if result.id == first_result.id && artifact.id == first_artifact.id
        ));
    }

    #[test]
    fn semantic_request_change_or_dependency_change_prevents_conservative_reuse() {
        let mut kernel = kernel();
        invoke(
            &mut kernel,
            ArtifactCommand::RecordRead {
                request: request("prompt", "text"),
                provider: provider(),
                invocation_provenance: "invocation-1".into(),
                content: b"first".to_vec(),
                dependencies: dependencies("v1"),
            },
        );

        assert_eq!(
            invoke(
                &mut kernel,
                ArtifactCommand::LookupRead {
                    request: request("prompt", "structured"),
                    provider: provider(),
                    dependencies: dependencies("v1"),
                },
            ),
            ArtifactResponse::ReadLookup { result: None }
        );
        assert_eq!(
            invoke(
                &mut kernel,
                ArtifactCommand::LookupRead {
                    request: request("prompt", "text"),
                    provider: provider(),
                    dependencies: dependencies("v2"),
                },
            ),
            ArtifactResponse::ReadLookup { result: None }
        );
    }

    #[test]
    fn semantic_revalidation_is_explicit_and_unknown_does_not_reuse() {
        let mut kernel = kernel();
        let recorded = invoke(
            &mut kernel,
            ArtifactCommand::RecordRead {
                request: request("prompt", "text"),
                provider: provider(),
                invocation_provenance: "invocation-1".into(),
                content: b"first".to_vec(),
                dependencies: dependencies("v1"),
            },
        );
        let ArtifactResponse::ReadRecorded { result, .. } = recorded else {
            panic!("unexpected read response")
        };

        let still_valid = invoke(
            &mut kernel,
            ArtifactCommand::Revalidate {
                result_id: result.id.clone(),
                provider: provider(),
                current_dependencies: dependencies("v2"),
                verdict: RevalidationVerdict::StillValid,
                provenance: "semantic-check-1".into(),
            },
        );
        assert!(matches!(
            still_valid,
            ArtifactResponse::Revalidated { reusable: true, .. }
        ));

        let unknown = invoke(
            &mut kernel,
            ArtifactCommand::Revalidate {
                result_id: result.id,
                provider: provider(),
                current_dependencies: dependencies("v3"),
                verdict: RevalidationVerdict::Unknown,
                provenance: "semantic-check-2".into(),
            },
        );
        assert!(matches!(
            unknown,
            ArtifactResponse::Revalidated {
                reusable: false,
                ..
            }
        ));
    }
}
