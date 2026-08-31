use phenix_core::{
    Bytes, ComponentInterface, ContextResourceId, ContextRevisionId, InterfaceId, ServiceId,
};
pub use phenix_core::{
    ContextDescriptor, ContextResourceKind, ContextResourceRevision, ContextScope,
};
use serde::{Deserialize, Serialize};

pub const CONTEXT_SERVICE: &str = "phenix.context@1";

#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Serialize,
    Deserialize,
    phenix_sdk_macros::PhenixValue,
)]
pub struct ExactContextReference {
    pub resource_id: ContextResourceId,
    pub revision: ContextRevisionId,
}

#[derive(
    Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, phenix_sdk_macros::PhenixValue,
)]
#[serde(rename_all = "snake_case")]
pub enum ContextInjectionRequester {
    User,
    Agent,
    Orchestration,
    ContextPolicy,
    Hook,
    Frontend,
}

#[derive(
    Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, phenix_sdk_macros::PhenixValue,
)]
#[serde(rename_all = "snake_case")]
pub enum ContextInjectionLifetime {
    Execution,
    Objective,
    Session,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, phenix_sdk_macros::PhenixValue)]
pub struct ContextInjection {
    pub sequence: u64,
    pub execution_id: String,
    pub source: ExactContextReference,
    pub requester: ContextInjectionRequester,
    pub lifetime: ContextInjectionLifetime,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, phenix_sdk_macros::PhenixValue)]
pub struct ProjectedContextEntry {
    pub injection: ContextInjection,
    pub resource: ContextResourceRevision,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, phenix_sdk_macros::PhenixValue)]
pub struct ExecutionContextProjection {
    pub execution_id: String,
    pub entries: Vec<ProjectedContextEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, phenix_sdk_macros::PhenixValue)]
pub struct RepositoryContextSource {
    pub path: String,
    pub content: Bytes,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, phenix_sdk_macros::PhenixValue)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum ContextCommand {
    Register {
        resource_id: ContextResourceId,
        kind: ContextResourceKind,
        source: String,
        scope: ContextScope,
        content: Bytes,
    },
    Get {
        resource_id: ContextResourceId,
        revision: ContextRevisionId,
    },
    List,
    DiscoverRepository {
        workspace_id: String,
        sources: Vec<RepositoryContextSource>,
    },
    Load {
        execution_id: String,
        resource_id: ContextResourceId,
        revision: ContextRevisionId,
        requester: ContextInjectionRequester,
        lifetime: ContextInjectionLifetime,
        reason: String,
    },
    Project {
        execution_id: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, phenix_sdk_macros::PhenixValue)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum ContextResponse {
    Registered {
        resource: ContextResourceRevision,
    },
    Resource {
        resource: Option<ContextResourceRevision>,
    },
    Resources {
        descriptors: Vec<ContextDescriptor>,
    },
    Discovered {
        descriptors: Vec<ContextDescriptor>,
    },
    Loaded {
        injection: ContextInjection,
        resource: ContextResourceRevision,
    },
    Projection {
        projection: ExecutionContextProjection,
    },
}

pub struct ContextInterface;

impl ComponentInterface for ContextInterface {
    fn interface_id() -> InterfaceId {
        InterfaceId::parse(CONTEXT_SERVICE).expect("static context interface id is valid")
    }

    fn schema() -> phenix_core::InterfaceSchema {
        phenix_core::InterfaceSchema::of::<ContextCommand, ContextResponse>()
    }
}

#[must_use]
pub fn context_service() -> ServiceId {
    ServiceId::parse(CONTEXT_SERVICE).expect("static context service id is valid")
}
