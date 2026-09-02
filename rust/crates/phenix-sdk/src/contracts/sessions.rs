pub use phenix_core::SessionId;
use phenix_core::{Bytes, ComponentInterface, InterfaceId, NamespaceTransaction, ServiceId};
use serde::{Deserialize, Serialize};

pub const SESSION_SERVICE: &str = "phenix.sessions@1";
pub const SESSION_MUTATION_SERVICE: &str = "phenix.sessions.mutation@1";

#[derive(
    Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, phenix_sdk_macros::PhenixValue,
)]
#[serde(rename_all = "snake_case")]
pub enum SessionInputKind {
    User,
    Root,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, phenix_sdk_macros::PhenixValue)]
pub struct SessionInput {
    pub sequence: u64,
    pub kind: SessionInputKind,
    pub content: Bytes,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, phenix_sdk_macros::PhenixValue)]
#[serde(deny_unknown_fields)]
pub struct SessionRecord {
    pub id: SessionId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, phenix_sdk_macros::PhenixValue)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum SessionMutationCommand {
    PrepareCreate { id: SessionId },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, phenix_sdk_macros::PhenixValue)]
#[serde(tag = "result", rename_all = "snake_case", deny_unknown_fields)]
pub enum SessionMutationResponse {
    PreparedCreate {
        session: SessionRecord,
        transaction: NamespaceTransaction,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, phenix_sdk_macros::PhenixValue)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum SessionCommand {
    Create {
        id: SessionId,
    },
    Get {
        id: SessionId,
    },
    List,
    Continue {
        id: SessionId,
        kind: SessionInputKind,
        content: Bytes,
    },
    Inputs {
        id: SessionId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, phenix_sdk_macros::PhenixValue)]
#[serde(tag = "result", rename_all = "snake_case", deny_unknown_fields)]
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
    Continued {
        session: SessionRecord,
        input: SessionInput,
    },
    Inputs {
        inputs: Vec<SessionInput>,
    },
}

pub struct SessionInterface;

impl ComponentInterface for SessionInterface {
    fn interface_id() -> InterfaceId {
        InterfaceId::parse(SESSION_SERVICE).expect("static session interface id is valid")
    }

    fn schema() -> phenix_core::InterfaceSchema {
        phenix_core::InterfaceSchema::of::<SessionCommand, SessionResponse>()
    }
}

pub struct SessionMutationInterface;

impl ComponentInterface for SessionMutationInterface {
    fn interface_id() -> InterfaceId {
        InterfaceId::parse(SESSION_MUTATION_SERVICE)
            .expect("static session mutation interface id is valid")
    }

    fn schema() -> phenix_core::InterfaceSchema {
        phenix_core::InterfaceSchema::of::<SessionMutationCommand, SessionMutationResponse>()
    }
}

#[must_use]
pub fn session_service() -> ServiceId {
    ServiceId::parse(SESSION_SERVICE).expect("static session service id is valid")
}

#[must_use]
pub fn session_mutation_service() -> ServiceId {
    ServiceId::parse(SESSION_MUTATION_SERVICE).expect("static session mutation service id is valid")
}
