use phenix_kernel::{
    Authority, CapabilityId, DurableSchema, PluginExecution, PluginHost, PluginId, PluginInstance,
    PluginManifest, ResourceNamespace, ServiceContribution, ServiceId, TransactionOp,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const LANGUAGE_SERVICE: &str = "phenix.language@1";
const LANGUAGE_PLUGIN: &str = "phenix.language";
const LANGUAGE_NAMESPACE: &str = "phenix.language.state";
const PERSISTENCE_SCHEMA: &str = "kernel.persistence.schema";
const PERSISTENCE_READ: &str = "kernel.persistence.read";
const PERSISTENCE_WRITE: &str = "kernel.persistence.write";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LanguageOperationKind {
    Definition,
    References,
    Implementations,
    Hover,
    DocumentSymbols,
    WorkspaceSymbols,
    Diagnostics,
    CallHierarchy,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentProvenance {
    WorkspaceBacked,
    FrontendUnsaved,
    MixedOrUnknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LanguageDocumentIdentity {
    pub path: String,
    pub file_version: Option<String>,
    pub provenance: DocumentProvenance,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LanguageProviderEpoch {
    pub workspace_id: String,
    pub provider_id: String,
    pub epoch: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LanguageOperationResult {
    pub operation: LanguageOperationKind,
    pub payload: serde_json::Value,
    pub documents: Vec<LanguageDocumentIdentity>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LanguageObservation {
    pub id: String,
    pub execution_id: String,
    pub workspace_id: String,
    pub provider_id: String,
    pub provider_epoch: u64,
    pub result: LanguageOperationResult,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum LanguageCommand {
    ActivateProvider {
        workspace_id: String,
        provider_id: String,
        epoch: u64,
    },
    EndProvider {
        workspace_id: String,
        provider_id: String,
        epoch: u64,
    },
    PublishDiagnostics {
        workspace_id: String,
        provider_id: String,
        epoch: u64,
        result: LanguageOperationResult,
    },
    CurrentDiagnostics {
        workspace_id: String,
    },
    Consume {
        observation_id: String,
        execution_id: String,
        workspace_id: String,
        provider_id: String,
        epoch: u64,
        result: LanguageOperationResult,
    },
    GetObservation {
        observation_id: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "response", rename_all = "snake_case")]
pub enum LanguageResponse {
    Provider { epoch: Option<LanguageProviderEpoch> },
    Diagnostics { result: Option<LanguageOperationResult> },
    Observation { observation: Option<LanguageObservation> },
}

#[must_use]
pub fn language_manifest() -> PluginManifest {
    PluginManifest {
        id: PluginId::parse(LANGUAGE_PLUGIN).expect("static plugin id is valid"),
        version: 1,
        execution: PluginExecution::Embedded,
        dependencies: Vec::new(),
        services: vec![ServiceContribution {
            service: language_service(),
            priority: 100,
            required_authority: Authority::default(),
        }],
        resource_namespaces: vec![language_namespace()],
        maximum_authority: Authority::new([
            capability(PERSISTENCE_SCHEMA),
            capability(PERSISTENCE_READ),
            capability(PERSISTENCE_WRITE),
        ]),
    }
}

#[must_use]
pub fn language_factory() -> Box<dyn PluginInstance> {
    Box::new(LanguagePlugin::default())
}

#[must_use]
pub fn language_service() -> ServiceId {
    ServiceId::parse(LANGUAGE_SERVICE).expect("static service id is valid")
}

fn language_namespace() -> ResourceNamespace {
    ResourceNamespace::parse(LANGUAGE_NAMESPACE).expect("static namespace is valid")
}

fn capability(value: &str) -> CapabilityId {
    CapabilityId::parse(value).expect("static capability is valid")
}

#[derive(Default)]
struct LanguagePlugin {
    providers: BTreeMap<String, LanguageProviderEpoch>,
    diagnostics: BTreeMap<String, LanguageOperationResult>,
}

impl PluginInstance for LanguagePlugin {
    fn start(&mut self, host: &PluginHost<'_>) -> Result<(), String> {
        host.register_durable_schema(&DurableSchema::new(language_namespace(), 1))
            .map_err(|error| error.to_string())
    }

    fn invoke(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        if service != &language_service() {
            return Err(format!("unsupported language service: {service}"));
        }
        let command: LanguageCommand =
            serde_json::from_slice(input).map_err(|error| error.to_string())?;
        let response = match command {
            LanguageCommand::ActivateProvider {
                workspace_id,
                provider_id,
                epoch,
            } => {
                let epoch = self.activate_provider(workspace_id, provider_id, epoch)?;
                LanguageResponse::Provider { epoch: Some(epoch) }
            }
            LanguageCommand::EndProvider {
                workspace_id,
                provider_id,
                epoch,
            } => {
                self.require_epoch(&workspace_id, &provider_id, epoch)?;
                self.providers.remove(&workspace_id);
                self.diagnostics.remove(&workspace_id);
                LanguageResponse::Provider { epoch: None }
            }
            LanguageCommand::PublishDiagnostics {
                workspace_id,
                provider_id,
                epoch,
                result,
            } => {
                self.require_epoch(&workspace_id, &provider_id, epoch)?;
                if result.operation != LanguageOperationKind::Diagnostics {
                    return Err("diagnostic publication must carry a diagnostics operation".into());
                }
                validate_result(&result)?;
                self.diagnostics.insert(workspace_id, result.clone());
                LanguageResponse::Diagnostics {
                    result: Some(result),
                }
            }
            LanguageCommand::CurrentDiagnostics { workspace_id } => {
                validate_identity("workspace id", &workspace_id)?;
                LanguageResponse::Diagnostics {
                    result: self.diagnostics.get(&workspace_id).cloned(),
                }
            }
            LanguageCommand::Consume {
                observation_id,
                execution_id,
                workspace_id,
                provider_id,
                epoch,
                result,
            } => {
                self.require_epoch(&workspace_id, &provider_id, epoch)?;
                validate_identity("language observation id", &observation_id)?;
                validate_identity("consuming execution id", &execution_id)?;
                validate_result(&result)?;
                let observation = LanguageObservation {
                    id: observation_id,
                    execution_id,
                    workspace_id,
                    provider_id,
                    provider_epoch: epoch,
                    result,
                };
                store_observation(host, &observation)?;
                LanguageResponse::Observation {
                    observation: Some(observation),
                }
            }
            LanguageCommand::GetObservation { observation_id } => {
                validate_identity("language observation id", &observation_id)?;
                LanguageResponse::Observation {
                    observation: read_observation(host, &observation_id)?,
                }
            }
        };
        serde_json::to_vec(&response).map_err(|error| error.to_string())
    }
}

impl LanguagePlugin {
    fn activate_provider(
        &mut self,
        workspace_id: String,
        provider_id: String,
        epoch: u64,
    ) -> Result<LanguageProviderEpoch, String> {
        validate_identity("workspace id", &workspace_id)?;
        validate_identity("language provider id", &provider_id)?;
        if epoch == 0 {
            return Err("language provider epoch must be non-zero".into());
        }
        if let Some(current) = self.providers.get(&workspace_id) {
            if epoch <= current.epoch {
                return Err(format!(
                    "language provider epoch must advance beyond {}",
                    current.epoch
                ));
            }
        }
        let active = LanguageProviderEpoch {
            workspace_id: workspace_id.clone(),
            provider_id,
            epoch,
        };
        self.providers.insert(workspace_id.clone(), active.clone());
        self.diagnostics.remove(&workspace_id);
        Ok(active)
    }

    fn require_epoch(
        &self,
        workspace_id: &str,
        provider_id: &str,
        epoch: u64,
    ) -> Result<(), String> {
        validate_identity("workspace id", workspace_id)?;
        validate_identity("language provider id", provider_id)?;
        match self.providers.get(workspace_id) {
            Some(active) if active.provider_id == provider_id && active.epoch == epoch => Ok(()),
            Some(active) => Err(format!(
                "ProviderChanged: active provider is {} epoch {}",
                active.provider_id, active.epoch
            )),
            None => Err("ProviderChanged: no active provider".into()),
        }
    }
}

fn store_observation(host: &PluginHost<'_>, observation: &LanguageObservation) -> Result<(), String> {
    let key = observation_key(&observation.id);
    let encoded = serde_json::to_vec(observation).map_err(|error| error.to_string())?;
    host.transact_durable(
        &language_namespace(),
        &[
            TransactionOp::AssertValue {
                key: key.clone(),
                expected: None,
            },
            TransactionOp::Put {
                key,
                value: encoded,
            },
        ],
    )
    .map_err(|error| error.to_string())
}

fn read_observation(
    host: &PluginHost<'_>,
    observation_id: &str,
) -> Result<Option<LanguageObservation>, String> {
    host.read_durable(&language_namespace(), &observation_key(observation_id))
        .map_err(|error| error.to_string())?
        .map(|value| serde_json::from_slice(&value).map_err(|error| error.to_string()))
        .transpose()
}

fn validate_result(result: &LanguageOperationResult) -> Result<(), String> {
    for document in &result.documents {
        validate_identity("language document path", &document.path)?;
        if matches!(document.provenance, DocumentProvenance::WorkspaceBacked)
            && document.file_version.as_deref().is_none_or(str::is_empty)
        {
            return Err("workspace-backed language evidence requires an exact file version".into());
        }
    }
    Ok(())
}

fn validate_identity(label: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{label} must not be empty"))
    } else {
        Ok(())
    }
}

fn observation_key(id: &str) -> String {
    format!("observation/{id}")
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

    fn kernel_with(path: &PathBuf) -> Kernel {
        let manifest = language_manifest();
        let plugin = manifest.id.clone();
        let persistence = LocalPersistence::open(path).unwrap();
        let mut kernel =
            Kernel::with_persistence(KernelConfig::new([manifest]).unwrap(), persistence);
        kernel
            .register_embedded_factory(plugin, language_factory)
            .unwrap();
        kernel.activate_all().unwrap();
        kernel
    }

    fn invoke(kernel: &mut Kernel, command: LanguageCommand) -> Result<LanguageResponse, String> {
        let input = serde_json::to_vec(&command).unwrap();
        let output = kernel
            .invoke(
                &language_service(),
                &input,
                &language_manifest().maximum_authority,
                None,
            )
            .map_err(|error| error.to_string())?;
        serde_json::from_slice(&output).map_err(|error| error.to_string())
    }

    fn activate(kernel: &mut Kernel, epoch: u64) {
        invoke(
            kernel,
            LanguageCommand::ActivateProvider {
                workspace_id: "workspace".into(),
                provider_id: "rust-analyzer".into(),
                epoch,
            },
        )
        .unwrap();
    }

    fn workspace_result(operation: LanguageOperationKind) -> LanguageOperationResult {
        LanguageOperationResult {
            operation,
            payload: serde_json::json!({"items": ["result"]}),
            documents: vec![LanguageDocumentIdentity {
                path: "src/lib.rs".into(),
                file_version: Some("sha256:abc".into()),
                provenance: DocumentProvenance::WorkspaceBacked,
            }],
        }
    }

    #[test]
    fn consumed_observations_are_durable_but_provider_and_diagnostics_are_not() {
        let path = temp_db("language-observation");
        {
            let mut kernel = kernel_with(&path);
            activate(&mut kernel, 1);
            invoke(
                &mut kernel,
                LanguageCommand::PublishDiagnostics {
                    workspace_id: "workspace".into(),
                    provider_id: "rust-analyzer".into(),
                    epoch: 1,
                    result: workspace_result(LanguageOperationKind::Diagnostics),
                },
            )
            .unwrap();
            invoke(
                &mut kernel,
                LanguageCommand::Consume {
                    observation_id: "language-1".into(),
                    execution_id: "execution-1".into(),
                    workspace_id: "workspace".into(),
                    provider_id: "rust-analyzer".into(),
                    epoch: 1,
                    result: workspace_result(LanguageOperationKind::Definition),
                },
            )
            .unwrap();
        }

        let mut restored = kernel_with(&path);
        assert!(matches!(
            invoke(
                &mut restored,
                LanguageCommand::GetObservation {
                    observation_id: "language-1".into(),
                }
            )
            .unwrap(),
            LanguageResponse::Observation {
                observation: Some(_)
            }
        ));
        assert_eq!(
            invoke(
                &mut restored,
                LanguageCommand::CurrentDiagnostics {
                    workspace_id: "workspace".into(),
                }
            )
            .unwrap(),
            LanguageResponse::Diagnostics { result: None }
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn stale_provider_epoch_cannot_record_a_successful_observation() {
        let path = temp_db("language-provider-change");
        let mut kernel = kernel_with(&path);
        activate(&mut kernel, 1);
        activate(&mut kernel, 2);
        let error = invoke(
            &mut kernel,
            LanguageCommand::Consume {
                observation_id: "language-1".into(),
                execution_id: "execution-1".into(),
                workspace_id: "workspace".into(),
                provider_id: "rust-analyzer".into(),
                epoch: 1,
                result: workspace_result(LanguageOperationKind::Hover),
            },
        )
        .unwrap_err();
        assert!(error.contains("ProviderChanged"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn unsaved_frontend_provenance_is_preserved_and_workspace_evidence_requires_a_version() {
        let path = temp_db("language-provenance");
        let mut kernel = kernel_with(&path);
        activate(&mut kernel, 1);
        let unsaved = LanguageOperationResult {
            operation: LanguageOperationKind::Hover,
            payload: serde_json::json!({"text": "hover"}),
            documents: vec![LanguageDocumentIdentity {
                path: "src/lib.rs".into(),
                file_version: None,
                provenance: DocumentProvenance::FrontendUnsaved,
            }],
        };
        let response = invoke(
            &mut kernel,
            LanguageCommand::Consume {
                observation_id: "language-unsaved".into(),
                execution_id: "execution-1".into(),
                workspace_id: "workspace".into(),
                provider_id: "rust-analyzer".into(),
                epoch: 1,
                result: unsaved,
            },
        )
        .unwrap();
        match response {
            LanguageResponse::Observation {
                observation: Some(observation),
            } => assert_eq!(
                observation.result.documents[0].provenance,
                DocumentProvenance::FrontendUnsaved
            ),
            other => panic!("unexpected response: {other:?}"),
        }

        let invalid = LanguageOperationResult {
            operation: LanguageOperationKind::Definition,
            payload: serde_json::json!({}),
            documents: vec![LanguageDocumentIdentity {
                path: "src/lib.rs".into(),
                file_version: None,
                provenance: DocumentProvenance::WorkspaceBacked,
            }],
        };
        let error = invoke(
            &mut kernel,
            LanguageCommand::Consume {
                observation_id: "language-invalid".into(),
                execution_id: "execution-1".into(),
                workspace_id: "workspace".into(),
                provider_id: "rust-analyzer".into(),
                epoch: 1,
                result: invalid,
            },
        )
        .unwrap_err();
        assert!(error.contains("exact file version"));
        let _ = fs::remove_file(path);
    }
}
