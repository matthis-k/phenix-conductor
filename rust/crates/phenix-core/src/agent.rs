use crate::ServiceId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SESSION_SERVICE: &str = "phenix.sessions@1";
pub const MODEL_INFERENCE_SERVICE: &str = "phenix.models.inference@1";
pub const TOOL_SERVICE: &str = "phenix.tools@1";
pub const SKILL_SERVICE: &str = "phenix.skills@1";
pub const CONTEXT_SERVICE: &str = "phenix.context@1";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionInputKind {
    User,
    Root,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionInput {
    pub sequence: u64,
    pub kind: SessionInputKind,
    pub content: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionRecord {
    pub id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum SessionCommand {
    Create {
        id: String,
    },
    Get {
        id: String,
    },
    List,
    Continue {
        id: String,
        kind: SessionInputKind,
        content: Vec<u8>,
    },
    Inputs {
        id: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelInferenceRequest {
    pub model: String,
    pub input: Vec<u8>,
    #[serde(default)]
    pub options: BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelInferenceResponse {
    pub output: Vec<u8>,
    pub provider_metadata: BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub id: String,
    #[serde(default)]
    pub input_schema: serde_json::Value,
    #[serde(default)]
    pub output_schema: serde_json::Value,
    #[serde(default)]
    pub output_prefix: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum ToolCommand {
    Register { tool: ToolDefinition },
    Get { id: String },
    List,
    Invoke { id: String, input: Vec<u8> },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum ToolResponse {
    Tool { tool: Option<ToolDefinition> },
    Tools { tools: Vec<ToolDefinition> },
    Output { output: Vec<u8> },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub id: String,
    pub content: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum SkillCommand {
    Register { skill: SkillDefinition },
    Get { id: String },
    List,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum SkillResponse {
    Skill { skill: Option<SkillDefinition> },
    Skills { skills: Vec<SkillDefinition> },
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextResourceKind {
    ProjectInstruction,
    ProjectDocument,
    Skill,
    External,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextScope {
    Workspace,
    PathPrefix(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContextDescriptor {
    pub resource_id: String,
    pub revision: String,
    pub kind: ContextResourceKind,
    pub source: String,
    pub scope: ContextScope,
    pub content_identity: String,
    pub estimated_bytes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContextResourceRevision {
    pub descriptor: ContextDescriptor,
    pub content: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum ContextCommand {
    Register {
        resource_id: String,
        kind: ContextResourceKind,
        source: String,
        scope: ContextScope,
        content: Vec<u8>,
    },
    Get {
        resource_id: String,
        revision: String,
    },
    List,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
}

pub fn session_service() -> ServiceId {
    ServiceId::parse(SESSION_SERVICE).expect("static service id")
}

pub fn model_inference_service() -> ServiceId {
    ServiceId::parse(MODEL_INFERENCE_SERVICE).expect("static service id")
}

pub fn tool_service() -> ServiceId {
    ServiceId::parse(TOOL_SERVICE).expect("static service id")
}

pub fn skill_service() -> ServiceId {
    ServiceId::parse(SKILL_SERVICE).expect("static service id")
}

pub fn context_service() -> ServiceId {
    ServiceId::parse(CONTEXT_SERVICE).expect("static service id")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_session_contract_rejects_tree_operations_and_fields() {
        let create = serde_json::to_value(SessionCommand::Create { id: "root".into() }).unwrap();
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
    }
}
