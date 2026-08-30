use crate::{Bytes, CallableId, ContextResourceId, ContextRevisionId, ModelId, ServiceId, SkillId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const MODEL_INFERENCE_SERVICE: &str = "phenix.models.inference@1";
pub const TOOL_SERVICE: &str = "phenix.tools@1";
pub const SKILL_SERVICE: &str = "phenix.skills@1";
pub const CONTEXT_SERVICE: &str = "phenix.context@1";

#[derive(phenix_sdk_macros::PhenixValue, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelInferenceRequest {
    pub model: ModelId,
    pub input: Bytes,
    #[serde(default)]
    pub options: BTreeMap<String, serde_json::Value>,
}

#[derive(phenix_sdk_macros::PhenixValue, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelInferenceResponse {
    pub output: Bytes,
    pub provider_metadata: BTreeMap<String, serde_json::Value>,
}

#[derive(phenix_sdk_macros::PhenixValue, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub id: CallableId,
    #[serde(default)]
    pub input_schema: serde_json::Value,
    #[serde(default)]
    pub output_schema: serde_json::Value,
    #[serde(default)]
    pub output_prefix: Bytes,
}

#[derive(phenix_sdk_macros::PhenixValue, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum ToolCommand {
    Register { tool: ToolDefinition },
    Get { id: CallableId },
    List,
    Invoke { id: CallableId, input: Bytes },
}

#[derive(phenix_sdk_macros::PhenixValue, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum ToolResponse {
    Tool { tool: Option<ToolDefinition> },
    Tools { tools: Vec<ToolDefinition> },
    Output { output: Bytes },
}

#[derive(phenix_sdk_macros::PhenixValue, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub id: SkillId,
    pub content: Bytes,
}

#[derive(phenix_sdk_macros::PhenixValue, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum SkillCommand {
    Register { skill: SkillDefinition },
    Get { id: SkillId },
    List,
}

#[derive(phenix_sdk_macros::PhenixValue, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum SkillResponse {
    Skill { skill: Option<SkillDefinition> },
    Skills { skills: Vec<SkillDefinition> },
}

#[derive(
    phenix_sdk_macros::PhenixValue,
    Clone,
    Debug,
    Eq,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ContextResourceKind {
    ProjectInstruction,
    ProjectDocument,
    Skill,
    External,
}

#[derive(
    phenix_sdk_macros::PhenixValue,
    Clone,
    Debug,
    Eq,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ContextScope {
    Workspace,
    PathPrefix(String),
}

#[derive(phenix_sdk_macros::PhenixValue, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContextDescriptor {
    pub resource_id: ContextResourceId,
    pub revision: ContextRevisionId,
    pub kind: ContextResourceKind,
    pub source: String,
    pub scope: ContextScope,
    pub content_identity: String,
    pub estimated_bytes: u64,
}

#[derive(phenix_sdk_macros::PhenixValue, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContextResourceRevision {
    pub descriptor: ContextDescriptor,
    pub content: Bytes,
}

#[derive(phenix_sdk_macros::PhenixValue, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
}

#[derive(phenix_sdk_macros::PhenixValue, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
