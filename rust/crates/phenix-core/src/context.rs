use super::PlanId;
use crate::{ConfigRevisionId, ExecutionId, ObjectiveId, WorkspaceId};
use serde::{Deserialize, Serialize};
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
}
