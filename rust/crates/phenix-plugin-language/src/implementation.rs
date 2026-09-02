use phenix_core::{
    Authority, CapabilityId, ComponentInterface, DurableSchema, Exact, PhenixValue, PluginContext,
    PluginExecution, PluginHost, PluginId, PluginInstance, PluginManifest, Project,
    ResourceNamespace, ServiceContribution, ServiceId, TransactionOp, Type, ValueCodec, ValueError,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, num::NonZeroU64};

pub const LANGUAGE_SERVICE: &str = "phenix.language@1";
const LANGUAGE_PLUGIN: &str = "phenix.language";
const LANGUAGE_NAMESPACE: &str = "phenix.language.state";
const PERSISTENCE_SCHEMA: &str = "kernel.persistence.schema";
const PERSISTENCE_READ: &str = "kernel.persistence.read";
const PERSISTENCE_WRITE: &str = "kernel.persistence.write";

#[derive(Default)]
struct LanguageState {
    providers: BTreeMap<String, LanguageProviderEpoch>,
    diagnostics: BTreeMap<String, DiagnosticsResult>,
}

type LanguageContext<'host, 'runtime, 'state> =
    PluginContext<'host, 'runtime, (), (), &'state mut LanguageState>;

fn context<'host, 'runtime, 'state>(
    host: &'host PluginHost<'runtime>,
    state: &'state mut LanguageState,
) -> LanguageContext<'host, 'runtime, 'state> {
    PluginContext::new(host, (), (), state)
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, phenix_sdk_macros::PhenixValue)]
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, phenix_sdk_macros::PhenixValue)]
#[serde(rename_all = "snake_case")]
pub enum DocumentProvenance {
    WorkspaceBacked,
    FrontendUnsaved,
    MixedOrUnknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, phenix_sdk_macros::PhenixValue)]
pub struct LanguageDocumentIdentity {
    pub path: String,
    pub file_version: Option<String>,
    pub provenance: DocumentProvenance,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "u64", into = "u64")]
pub struct ProviderEpoch(NonZeroU64);

impl ProviderEpoch {
    pub fn new(value: u64) -> Result<Self, &'static str> {
        value.try_into()
    }

    #[must_use]
    pub fn get(self) -> u64 {
        self.0.get()
    }
}

impl TryFrom<u64> for ProviderEpoch {
    type Error = &'static str;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        NonZeroU64::new(value)
            .map(Self)
            .ok_or("language provider epoch must be non-zero")
    }
}

impl From<ProviderEpoch> for u64 {
    fn from(value: ProviderEpoch) -> Self {
        value.get()
    }
}

impl ValueCodec for ProviderEpoch {
    fn phenix_type() -> Type {
        <u64 as ValueCodec>::phenix_type()
    }

    fn to_value(&self) -> PhenixValue {
        <u64 as ValueCodec>::to_value(&self.get())
    }

    fn from_value(value: &PhenixValue) -> Result<Self, ValueError> {
        let value = <u64 as ValueCodec>::from_value(value)?;
        Self::try_from(value).map_err(|error| ValueError::InvalidValue(error.into()))
    }

    fn project_from_value(value: &PhenixValue) -> Result<Self, ValueError> {
        let value = <u64 as ValueCodec>::project_from_value(value)?;
        Self::try_from(value).map_err(|error| ValueError::InvalidValue(error.into()))
    }
}

impl From<&ProviderEpoch> for PhenixValue {
    fn from(value: &ProviderEpoch) -> Self {
        value.to_value()
    }
}

impl<'value> TryFrom<Exact<&'value PhenixValue>> for ProviderEpoch {
    type Error = ValueError;

    fn try_from(value: Exact<&'value PhenixValue>) -> Result<Self, Self::Error> {
        Self::from_value(value.0)
    }
}

impl<'value> TryFrom<Project<&'value PhenixValue>> for ProviderEpoch {
    type Error = ValueError;

    fn try_from(value: Project<&'value PhenixValue>) -> Result<Self, Self::Error> {
        Self::project_from_value(value.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, phenix_sdk_macros::PhenixValue)]
pub struct LanguageProviderEpoch {
    pub workspace_id: String,
    pub provider_id: String,
    pub epoch: ProviderEpoch,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, phenix_sdk_macros::PhenixValue)]
pub struct LanguageOperationResult {
    pub operation: LanguageOperationKind,
    pub payload: PhenixValue,
    pub documents: Vec<LanguageDocumentIdentity>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, phenix_sdk_macros::PhenixValue)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum DiagnosticsResult {
    Diagnostics {
        payload: PhenixValue,
        documents: Vec<LanguageDocumentIdentity>,
    },
}

impl DiagnosticsResult {
    fn documents(&self) -> &[LanguageDocumentIdentity] {
        match self {
            Self::Diagnostics { documents, .. } => documents,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, phenix_sdk_macros::PhenixValue)]
pub struct LanguageObservation {
    pub id: String,
    pub execution_id: String,
    pub workspace_id: String,
    pub provider_id: String,
    pub provider_epoch: ProviderEpoch,
    pub result: LanguageOperationResult,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, phenix_sdk_macros::PhenixValue)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum LanguageCommand {
    ActivateProvider {
        workspace_id: String,
        provider_id: String,
        epoch: ProviderEpoch,
    },
    EndProvider {
        workspace_id: String,
        provider_id: String,
        epoch: ProviderEpoch,
    },
    PublishDiagnostics {
        workspace_id: String,
        provider_id: String,
        epoch: ProviderEpoch,
        result: DiagnosticsResult,
    },
    CurrentDiagnostics {
        workspace_id: String,
    },
    Consume {
        observation_id: String,
        execution_id: String,
        workspace_id: String,
        provider_id: String,
        epoch: ProviderEpoch,
        result: LanguageOperationResult,
    },
    GetObservation {
        observation_id: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, phenix_sdk_macros::PhenixValue)]
#[serde(tag = "response", rename_all = "snake_case")]
pub enum LanguageResponse {
    Provider {
        epoch: Option<LanguageProviderEpoch>,
    },
    Diagnostics {
        result: Option<DiagnosticsResult>,
    },
    Observation {
        observation: Option<LanguageObservation>,
    },
}

#[must_use]
pub fn language_manifest() -> PluginManifest {
    PluginManifest {
        id: PluginId::parse(LANGUAGE_PLUGIN).expect("static plugin id is valid"),
        version: 1,
        execution: PluginExecution::Embedded,
        dependencies: Vec::new(),
        services: vec![ServiceContribution {
            role: phenix_core::ServiceRole::Terminal,
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
    state: LanguageState,
}

impl PluginInstance for LanguagePlugin {
    fn start(&mut self, host: &PluginHost<'_>) -> Result<(), String> {
        context(host, &mut self.state)
            .kernel
            .register_durable_schema(&DurableSchema::new(language_namespace(), 1))
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
        let mut context = context(host, &mut self.state);
        let interface = crate::LanguageInterface::interface_id();
        let command = context
            .kernel
            .decode_projected::<LanguageCommand>(&interface, input)
            .map_err(|error| error.to_string())?;
        let response = handle(&mut context, command)?;
        context
            .kernel
            .encode_value(&response)
            .map_err(|error| error.to_string())
    }
}

fn handle(
    context: &mut LanguageContext<'_, '_, '_>,
    command: LanguageCommand,
) -> Result<LanguageResponse, String> {
    match command {
        LanguageCommand::ActivateProvider {
            workspace_id,
            provider_id,
            epoch,
        } => Ok(LanguageResponse::Provider {
            epoch: Some(activate_provider(
                context,
                workspace_id,
                provider_id,
                epoch,
            )?),
        }),
        LanguageCommand::EndProvider {
            workspace_id,
            provider_id,
            epoch,
        } => {
            require_epoch(context, &workspace_id, &provider_id, epoch)?;
            context.plugin.state.providers.remove(&workspace_id);
            context.plugin.state.diagnostics.remove(&workspace_id);
            Ok(LanguageResponse::Provider { epoch: None })
        }
        LanguageCommand::PublishDiagnostics {
            workspace_id,
            provider_id,
            epoch,
            result,
        } => {
            require_epoch(context, &workspace_id, &provider_id, epoch)?;
            validate_documents(result.documents())?;
            context
                .plugin
                .state
                .diagnostics
                .insert(workspace_id, result.clone());
            Ok(LanguageResponse::Diagnostics {
                result: Some(result),
            })
        }
        LanguageCommand::CurrentDiagnostics { workspace_id } => {
            validate_identity("workspace id", &workspace_id)?;
            Ok(LanguageResponse::Diagnostics {
                result: context.plugin.state.diagnostics.get(&workspace_id).cloned(),
            })
        }
        LanguageCommand::Consume {
            observation_id,
            execution_id,
            workspace_id,
            provider_id,
            epoch,
            result,
        } => {
            require_epoch(context, &workspace_id, &provider_id, epoch)?;
            validate_identity("language observation id", &observation_id)?;
            validate_identity("consuming execution id", &execution_id)?;
            validate_documents(&result.documents)?;
            let observation = LanguageObservation {
                id: observation_id,
                execution_id,
                workspace_id,
                provider_id,
                provider_epoch: epoch,
                result,
            };
            store_observation(context, &observation)?;
            Ok(LanguageResponse::Observation {
                observation: Some(observation),
            })
        }
        LanguageCommand::GetObservation { observation_id } => {
            validate_identity("language observation id", &observation_id)?;
            Ok(LanguageResponse::Observation {
                observation: read_observation(context, &observation_id)?,
            })
        }
    }
}

fn activate_provider(
    context: &mut LanguageContext<'_, '_, '_>,
    workspace_id: String,
    provider_id: String,
    epoch: ProviderEpoch,
) -> Result<LanguageProviderEpoch, String> {
    validate_identity("workspace id", &workspace_id)?;
    validate_identity("language provider id", &provider_id)?;
    if let Some(current) = context.plugin.state.providers.get(&workspace_id) {
        if epoch <= current.epoch {
            return Err(format!(
                "language provider epoch must advance beyond {}",
                current.epoch.get()
            ));
        }
    }
    let active = LanguageProviderEpoch {
        workspace_id: workspace_id.clone(),
        provider_id,
        epoch,
    };
    context
        .plugin
        .state
        .providers
        .insert(workspace_id.clone(), active.clone());
    context.plugin.state.diagnostics.remove(&workspace_id);
    Ok(active)
}

fn require_epoch(
    context: &LanguageContext<'_, '_, '_>,
    workspace_id: &str,
    provider_id: &str,
    epoch: ProviderEpoch,
) -> Result<(), String> {
    validate_identity("workspace id", workspace_id)?;
    validate_identity("language provider id", provider_id)?;
    match context.plugin.state.providers.get(workspace_id) {
        Some(active) if active.provider_id == provider_id && active.epoch == epoch => Ok(()),
        Some(active) => Err(format!(
            "ProviderChanged: active provider is {} epoch {}",
            active.provider_id,
            active.epoch.get()
        )),
        None => Err("ProviderChanged: no active provider".into()),
    }
}

fn store_observation(
    context: &LanguageContext<'_, '_, '_>,
    observation: &LanguageObservation,
) -> Result<(), String> {
    let key = observation_key(&observation.id);
    let encoded = serde_json::to_vec(observation).map_err(|error| error.to_string())?;
    context
        .kernel
        .transact_durable(
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
    context: &LanguageContext<'_, '_, '_>,
    observation_id: &str,
) -> Result<Option<LanguageObservation>, String> {
    context
        .kernel
        .read_durable(&language_namespace(), &observation_key(observation_id))
        .map_err(|error| error.to_string())?
        .map(|value| serde_json::from_slice(&value).map_err(|error| error.to_string()))
        .transpose()
}

fn validate_documents(documents: &[LanguageDocumentIdentity]) -> Result<(), String> {
    for document in documents {
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
    use phenix_core::{Kernel, KernelConfig, LocalPersistence};
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
        let input = serde_json::to_vec(&phenix_core::PhenixValue::from(&command)).unwrap();
        let output = kernel
            .invoke(
                &language_service(),
                &input,
                &language_manifest().maximum_authority,
                None,
            )
            .map_err(|error| error.to_string())?;
        let output: phenix_core::PhenixValue =
            serde_json::from_slice(&output).map_err(|error| error.to_string())?;
        output.project().map_err(|error| error.to_string())
    }

    fn epoch(value: u64) -> ProviderEpoch {
        ProviderEpoch::new(value).unwrap()
    }

    fn activate(kernel: &mut Kernel, value: u64) {
        invoke(
            kernel,
            LanguageCommand::ActivateProvider {
                workspace_id: "workspace".into(),
                provider_id: "rust-analyzer".into(),
                epoch: epoch(value),
            },
        )
        .unwrap();
    }

    fn workspace_result(operation: LanguageOperationKind) -> LanguageOperationResult {
        LanguageOperationResult {
            operation,
            payload: serde_json::json!({"items": ["result"]}).into(),
            documents: vec![LanguageDocumentIdentity {
                path: "src/lib.rs".into(),
                file_version: Some("sha256:abc".into()),
                provenance: DocumentProvenance::WorkspaceBacked,
            }],
        }
    }

    fn diagnostics_result() -> DiagnosticsResult {
        DiagnosticsResult::Diagnostics {
            payload: serde_json::json!({"items": ["result"]}).into(),
            documents: vec![LanguageDocumentIdentity {
                path: "src/lib.rs".into(),
                file_version: Some("sha256:abc".into()),
                provenance: DocumentProvenance::WorkspaceBacked,
            }],
        }
    }

    #[test]
    fn zero_provider_epoch_is_rejected_at_decode_boundary() {
        assert!(ProviderEpoch::new(0).is_err());
        assert!(serde_json::from_value::<ProviderEpoch>(serde_json::json!(0)).is_err());
        let value = PhenixValue::U64(0);
        assert!(ProviderEpoch::try_from(Project(&value)).is_err());
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
                    epoch: epoch(1),
                    result: diagnostics_result(),
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
                    epoch: epoch(1),
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
                epoch: epoch(1),
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
            payload: serde_json::json!({"text": "hover"}).into(),
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
                epoch: epoch(1),
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
            payload: serde_json::json!({}).into(),
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
                epoch: epoch(1),
                result: invalid,
            },
        )
        .unwrap_err();
        assert!(error.contains("exact file version"));
        let _ = fs::remove_file(path);
    }
}
