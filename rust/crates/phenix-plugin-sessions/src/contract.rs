use phenix_core::{Bytes, ServiceId, SessionId};
use phenix_sdk_macros::PhenixValue;
use serde::{Deserialize, Serialize};

pub const SESSION_SERVICE: &str = "phenix.sessions@1";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, PhenixValue)]
#[serde(rename_all = "snake_case")]
pub enum SessionInputKind {
    User,
    Root,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, PhenixValue)]
pub struct SessionInput {
    pub sequence: u64,
    pub kind: SessionInputKind,
    pub content: Bytes,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, PhenixValue)]
#[serde(deny_unknown_fields)]
pub struct SessionRecord {
    pub id: SessionId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, PhenixValue)]
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, PhenixValue)]
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

#[must_use]
pub fn session_service() -> ServiceId {
    ServiceId::parse(SESSION_SERVICE).expect("static session service id is valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_session_contract_rejects_tree_operations_and_fields() {
        let create = serde_json::to_value(SessionCommand::Create {
            id: SessionId::parse("root").unwrap(),
        })
        .unwrap();
        assert_eq!(
            create,
            serde_json::json!({"operation":"create","id":"root"})
        );

        assert!(serde_json::from_value::<SessionCommand>(serde_json::json!({
            "operation": "create",
            "id": "child",
            "parent": "root"
        }))
        .is_err());
        assert!(serde_json::from_value::<SessionCommand>(serde_json::json!({
            "operation": "children",
            "parent": "root"
        }))
        .is_err());
        assert!(serde_json::from_value::<SessionRecord>(serde_json::json!({
            "id": "child",
            "parent": "root"
        }))
        .is_err());
        assert!(serde_json::from_value::<SessionCommand>(serde_json::json!({
            "operation": "get",
            "id": ""
        }))
        .is_err());
    }
}
