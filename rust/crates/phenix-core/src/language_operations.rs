use super::LanguageObservationId;
use crate::{ExecutionId, FileVersion, LanguageProviderId, LanguageServiceKind, WorkspaceId};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LanguageDocumentProvenance {
    WorkspaceBacked,
    FrontendUnsaved,
    MixedOrUnknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LanguageDocumentIdentity {
    pub path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_version: Option<FileVersion>,
    pub provenance: LanguageDocumentProvenance,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LanguagePosition {
    pub line: u32,
    pub character: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LanguageRange {
    pub start: LanguagePosition,
    pub end: LanguagePosition,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LanguageLocation {
    pub document: PathBuf,
    pub range: LanguageRange,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum LanguageOperation {
    Definition {
        document: PathBuf,
        position: LanguagePosition,
    },
    References {
        document: PathBuf,
        position: LanguagePosition,
    },
    Implementations {
        document: PathBuf,
        position: LanguagePosition,
    },
    Hover {
        document: PathBuf,
        position: LanguagePosition,
    },
    DocumentSymbols {
        document: PathBuf,
    },
    WorkspaceSymbols {
        query: String,
    },
    Diagnostics {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        document: Option<PathBuf>,
    },
    CallHierarchy {
        document: PathBuf,
        position: LanguagePosition,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LanguageOperationResult {
    pub value: Value,
    #[serde(default)]
    pub documents: Vec<LanguageDocumentIdentity>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LanguageObservationInput {
    pub execution: ExecutionId,
    pub workspace: WorkspaceId,
    pub service: LanguageServiceKind,
    pub provider: LanguageProviderId,
    pub provider_epoch: u64,
    pub operation: LanguageOperation,
    pub result: LanguageOperationResult,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LanguageObservation {
    pub id: LanguageObservationId,
    pub execution: ExecutionId,
    pub workspace: WorkspaceId,
    pub service: LanguageServiceKind,
    pub provider: LanguageProviderId,
    pub provider_epoch: u64,
    pub operation: LanguageOperation,
    pub result: LanguageOperationResult,
}

impl From<LanguageObservation> for LanguageObservationInput {
    fn from(observation: LanguageObservation) -> Self {
        Self {
            execution: observation.execution,
            workspace: observation.workspace,
            service: observation.service,
            provider: observation.provider,
            provider_epoch: observation.provider_epoch,
            operation: observation.operation,
            result: observation.result,
        }
    }
}

impl LanguageObservationInput {
    #[must_use]
    pub fn is_workspace_backed(&self) -> bool {
        !self.result.documents.is_empty()
            && self.result.documents.iter().all(|document| {
                document.provenance == LanguageDocumentProvenance::WorkspaceBacked
                    && document.workspace_version.is_some()
            })
    }
}

impl LanguageObservation {
    #[must_use]
    pub fn is_workspace_backed(&self) -> bool {
        !self.result.documents.is_empty()
            && self.result.documents.iter().all(|document| {
                document.provenance == LanguageDocumentProvenance::WorkspaceBacked
                    && document.workspace_version.is_some()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FileKind;

    fn workspace_document(path: &str) -> LanguageDocumentIdentity {
        LanguageDocumentIdentity {
            path: PathBuf::from(path),
            workspace_version: Some(FileVersion::Present {
                content_hash: format!("sha256:{path}"),
                kind: FileKind::Regular,
            }),
            provenance: LanguageDocumentProvenance::WorkspaceBacked,
        }
    }

    #[test]
    fn consumed_language_results_preserve_execution_provider_and_document_provenance() {
        let observation = LanguageObservationInput {
            execution: ExecutionId::parse("execution-1").unwrap(),
            workspace: WorkspaceId::parse("workspace:test").unwrap(),
            service: LanguageServiceKind::parse("rust").unwrap(),
            provider: LanguageProviderId::parse("rust-analyzer").unwrap(),
            provider_epoch: 7,
            operation: LanguageOperation::Definition {
                document: PathBuf::from("src/lib.rs"),
                position: LanguagePosition {
                    line: 10,
                    character: 4,
                },
            },
            result: LanguageOperationResult {
                value: serde_json::json!({"locations": []}),
                documents: vec![workspace_document("src/lib.rs")],
            },
        };

        assert_eq!(observation.execution.as_str(), "execution-1");
        assert_eq!(observation.provider_epoch, 7);
        assert!(observation.is_workspace_backed());
    }

    #[test]
    fn unsaved_frontend_state_is_not_authoritative_workspace_evidence() {
        let observation = LanguageObservationInput {
            execution: ExecutionId::parse("execution-1").unwrap(),
            workspace: WorkspaceId::parse("workspace:test").unwrap(),
            service: LanguageServiceKind::parse("rust").unwrap(),
            provider: LanguageProviderId::parse("editor-lsp").unwrap(),
            provider_epoch: 3,
            operation: LanguageOperation::Hover {
                document: PathBuf::from("src/lib.rs"),
                position: LanguagePosition {
                    line: 2,
                    character: 1,
                },
            },
            result: LanguageOperationResult {
                value: serde_json::json!({"contents": "draft"}),
                documents: vec![LanguageDocumentIdentity {
                    path: PathBuf::from("src/lib.rs"),
                    workspace_version: None,
                    provenance: LanguageDocumentProvenance::FrontendUnsaved,
                }],
            },
        };

        assert!(!observation.is_workspace_backed());
    }

    #[test]
    fn workspace_provenance_without_exact_version_is_not_authoritative() {
        let observation = LanguageObservationInput {
            execution: ExecutionId::parse("execution-1").unwrap(),
            workspace: WorkspaceId::parse("workspace:test").unwrap(),
            service: LanguageServiceKind::parse("rust").unwrap(),
            provider: LanguageProviderId::parse("editor-lsp").unwrap(),
            provider_epoch: 5,
            operation: LanguageOperation::DocumentSymbols {
                document: PathBuf::from("src/lib.rs"),
            },
            result: LanguageOperationResult {
                value: serde_json::json!([]),
                documents: vec![LanguageDocumentIdentity {
                    path: PathBuf::from("src/lib.rs"),
                    workspace_version: None,
                    provenance: LanguageDocumentProvenance::WorkspaceBacked,
                }],
            },
        };

        assert!(!observation.is_workspace_backed());
    }

    #[test]
    fn mixed_result_is_not_authoritative_workspace_evidence() {
        let observation = LanguageObservationInput {
            execution: ExecutionId::parse("execution-1").unwrap(),
            workspace: WorkspaceId::parse("workspace:test").unwrap(),
            service: LanguageServiceKind::parse("rust").unwrap(),
            provider: LanguageProviderId::parse("editor-lsp").unwrap(),
            provider_epoch: 4,
            operation: LanguageOperation::References {
                document: PathBuf::from("src/lib.rs"),
                position: LanguagePosition {
                    line: 1,
                    character: 0,
                },
            },
            result: LanguageOperationResult {
                value: serde_json::json!({"locations": []}),
                documents: vec![
                    workspace_document("src/lib.rs"),
                    LanguageDocumentIdentity {
                        path: PathBuf::from("src/draft.rs"),
                        workspace_version: None,
                        provenance: LanguageDocumentProvenance::MixedOrUnknown,
                    },
                ],
            },
        };

        assert!(!observation.is_workspace_backed());
    }
}
