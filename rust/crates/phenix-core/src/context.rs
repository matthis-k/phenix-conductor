use super::PlanId;
use crate::{ConfigRevisionId, ExecutionId, ObjectiveId, WorkspaceId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ContextResourceId(String);

impl ContextResourceId {
    pub fn parse(value: impl Into<String>) -> Result<Self, crate::InvalidId> {
        let value = value.into();
        if value.trim().is_empty() {
            Err(crate::InvalidId)
        } else {
            Ok(Self(value))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for ContextResourceId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ContextRevision(String);

impl ContextRevision {
    pub fn parse(value: impl Into<String>) -> Result<Self, crate::InvalidId> {
        let value = value.into();
        if value.trim().is_empty() {
            Err(crate::InvalidId)
        } else {
            Ok(Self(value))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for ContextRevision {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FileObservationId(String);

impl FileObservationId {
    pub fn parse(value: impl Into<String>) -> Result<Self, crate::InvalidId> {
        let value = value.into();
        if value.trim().is_empty() {
            Err(crate::InvalidId)
        } else {
            Ok(Self(value))
        }
    }
}

impl Display for FileObservationId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LanguageObservationId(String);

impl LanguageObservationId {
    pub fn parse(value: impl Into<String>) -> Result<Self, crate::InvalidId> {
        let value = value.into();
        if value.trim().is_empty() {
            Err(crate::InvalidId)
        } else {
            Ok(Self(value))
        }
    }
}

impl Display for LanguageObservationId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum ExactReference {
    Objective(ObjectiveId),
    Plan(PlanId),
    Execution(ExecutionId),
    Event(u64),
    FileObservation(FileObservationId),
    LanguageObservation(LanguageObservationId),
    Context(ContextResourceId),
}

impl Display for ExactReference {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Objective(id) => write!(f, "objective:{id}"),
            Self::Plan(id) => write!(f, "plan:{id}"),
            Self::Execution(id) => write!(f, "execution:{id}"),
            Self::Event(sequence) => write!(f, "event:{sequence}"),
            Self::FileObservation(id) => write!(f, "file-observation:{id}"),
            Self::LanguageObservation(id) => write!(f, "lsp-observation:{id}"),
            Self::Context(id) => write!(f, "context:{id}"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextResourceKind {
    Skill,
    ProjectDocument,
    Objective,
    Plan,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextTier {
    MandatoryContent,
    MandatoryMetadata,
    DiscoverableContent,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ContextScope {
    Workspace { workspace_id: WorkspaceId },
    Execution { execution_id: ExecutionId },
    Objective { objective_id: ObjectiveId },
    Path { path: PathBuf },
    Configuration { revision: ConfigRevisionId },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContextDescriptor {
    pub id: ContextResourceId,
    pub kind: ContextResourceKind,
    pub title: String,
    pub description: String,
    pub scope: ContextScope,
    pub revision: ContextRevision,
    pub estimated_cost: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContextResourceRevision {
    pub descriptor: ContextDescriptor,
    pub tier: ContextTier,
    pub source_ref: ExactReference,
    pub content_identity: ContextRevision,
    pub content: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ContextCatalog {
    current: BTreeMap<ContextResourceId, ContextRevision>,
    revisions: BTreeMap<(ContextResourceId, ContextRevision), ContextResourceRevision>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContextCatalogError {
    ConflictingRevision {
        id: ContextResourceId,
        revision: ContextRevision,
    },
    UnknownResource(ContextResourceId),
    UnknownRevision {
        id: ContextResourceId,
        revision: ContextRevision,
    },
}

impl Display for ContextCatalogError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConflictingRevision { id, revision } => {
                write!(f, "context resource {id} revision {revision} changed after registration")
            }
            Self::UnknownResource(id) => write!(f, "unknown context resource: {id}"),
            Self::UnknownRevision { id, revision } => {
                write!(f, "unknown context resource revision: {id}@{revision}")
            }
        }
    }
}

impl std::error::Error for ContextCatalogError {}

impl ContextCatalog {
    pub fn register_revision(
        &mut self,
        revision: ContextResourceRevision,
    ) -> Result<(), ContextCatalogError> {
        let id = revision.descriptor.id.clone();
        let descriptor_revision = revision.descriptor.revision.clone();
        let key = (id.clone(), descriptor_revision.clone());
        if let Some(existing) = self.revisions.get(&key) {
            if existing != &revision {
                return Err(ContextCatalogError::ConflictingRevision {
                    id,
                    revision: descriptor_revision,
                });
            }
            return Ok(());
        }
        self.revisions.insert(key, revision);
        self.current.insert(id, descriptor_revision);
        Ok(())
    }

    pub fn current_descriptor(
        &self,
        id: &ContextResourceId,
    ) -> Result<&ContextDescriptor, ContextCatalogError> {
        let revision = self
            .current
            .get(id)
            .ok_or_else(|| ContextCatalogError::UnknownResource(id.clone()))?;
        Ok(&self
            .revisions
            .get(&(id.clone(), revision.clone()))
            .expect("current context revision must exist")
            .descriptor)
    }

    pub fn resolve_revision(
        &self,
        id: &ContextResourceId,
        revision: &ContextRevision,
    ) -> Result<&ContextResourceRevision, ContextCatalogError> {
        self.revisions
            .get(&(id.clone(), revision.clone()))
            .ok_or_else(|| ContextCatalogError::UnknownRevision {
                id: id.clone(),
                revision: revision.clone(),
            })
    }

    pub fn descriptors(&self) -> impl Iterator<Item = &ContextDescriptor> {
        self.current.iter().map(|(id, revision)| {
            &self
                .revisions
                .get(&(id.clone(), revision.clone()))
                .expect("current context revision must exist")
                .descriptor
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextInjectionRequester {
    Agent,
    User,
    Orchestration,
    ContextPolicy,
    Hook,
    Frontend,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextInjectionLifetime {
    SingleRequest,
    Execution,
    Objective,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContextInjection {
    pub execution_id: ExecutionId,
    pub source_ref: ExactReference,
    pub source_revision: ContextRevision,
    pub requested_by: ContextInjectionRequester,
    pub reason: String,
    pub lifetime: ContextInjectionLifetime,
    pub content_identity: ContextRevision,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resource_revision(revision: &str, content: &str) -> ContextResourceRevision {
        let id = ContextResourceId::parse("project:development").unwrap();
        let revision = ContextRevision::parse(revision).unwrap();
        ContextResourceRevision {
            descriptor: ContextDescriptor {
                id: id.clone(),
                kind: ContextResourceKind::ProjectDocument,
                title: "Development".to_owned(),
                description: "Project development instructions".to_owned(),
                scope: ContextScope::Path {
                    path: PathBuf::from("DEVELOPMENT.md"),
                },
                revision: revision.clone(),
                estimated_cost: content.len() as u64,
            },
            tier: ContextTier::DiscoverableContent,
            source_ref: ExactReference::Context(id),
            content_identity: revision,
            content: Some(content.to_owned()),
        }
    }

    #[test]
    fn exact_reference_display_is_typed_and_stable() {
        assert_eq!(
            ExactReference::Objective(ObjectiveId::parse("objective-7").unwrap()).to_string(),
            "objective:objective-7"
        );
        assert_eq!(ExactReference::Event(42).to_string(), "event:42");
    }

    #[test]
    fn context_resource_identity_rejects_empty_values() {
        assert!(ContextResourceId::parse(" ").is_err());
        assert!(ContextRevision::parse("").is_err());
    }

    #[test]
    fn catalog_keeps_historical_revisions_exactly_resolvable() {
        let mut catalog = ContextCatalog::default();
        let first = resource_revision("sha256:first", "first bytes");
        let second = resource_revision("sha256:second", "second bytes");
        let id = first.descriptor.id.clone();
        let first_revision = first.descriptor.revision.clone();
        let second_revision = second.descriptor.revision.clone();

        catalog.register_revision(first.clone()).unwrap();
        catalog.register_revision(second.clone()).unwrap();

        assert_eq!(catalog.current_descriptor(&id).unwrap().revision, second_revision);
        assert_eq!(catalog.resolve_revision(&id, &first_revision).unwrap(), &first);
        assert_eq!(catalog.resolve_revision(&id, &second_revision).unwrap(), &second);
    }

    #[test]
    fn catalog_rejects_rewriting_an_immutable_revision() {
        let mut catalog = ContextCatalog::default();
        let original = resource_revision("sha256:same", "original bytes");
        let mut rewritten = original.clone();
        rewritten.content = Some("different bytes".to_owned());

        catalog.register_revision(original).unwrap();
        assert!(matches!(
            catalog.register_revision(rewritten),
            Err(ContextCatalogError::ConflictingRevision { .. })
        ));
    }

    #[test]
    fn catalog_does_not_substitute_current_revision_for_unknown_requested_revision() {
        let mut catalog = ContextCatalog::default();
        let current = resource_revision("sha256:current", "current bytes");
        let id = current.descriptor.id.clone();
        catalog.register_revision(current).unwrap();

        let requested = ContextRevision::parse("sha256:missing").unwrap();
        assert_eq!(
            catalog.resolve_revision(&id, &requested),
            Err(ContextCatalogError::UnknownRevision {
                id,
                revision: requested,
            })
        );
    }
}
