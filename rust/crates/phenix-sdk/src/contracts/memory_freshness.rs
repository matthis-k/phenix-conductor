use phenix_core::{CallableId, ServiceId};
use serde::{Deserialize, Serialize};

pub const MEMORY_VALIDATE_CALLABLE: &str = "memory.validate";

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
    phenix_sdk_macros::PhenixValue,
)]
#[serde(rename_all = "snake_case")]
pub enum MemoryFreshness {
    #[default]
    Current,
    NeedsValidation,
    Historical,
}

#[derive(
    Clone,
    Debug,
    Eq,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
    phenix_sdk_macros::PhenixValue,
)]
#[serde(deny_unknown_fields)]
pub struct MemoryDependencyRevision {
    pub service: ServiceId,
    pub resource: String,
    pub revision: Option<String>,
}

#[derive(
    Clone,
    Debug,
    Eq,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
    phenix_sdk_macros::PhenixValue,
)]
#[serde(deny_unknown_fields)]
pub struct MemoryCanonicalReference {
    pub service: ServiceId,
    pub resource: String,
    pub revision: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, phenix_sdk_macros::PhenixValue)]
#[serde(deny_unknown_fields)]
pub struct MemoryFreshnessRecord {
    pub memory_id: String,
    pub freshness: MemoryFreshness,
    pub changed_at: u64,
    pub dependencies: Vec<MemoryDependencyRevision>,
    pub canonical_reference: Option<MemoryCanonicalReference>,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
    phenix_sdk_macros::PhenixValue,
)]
#[serde(rename_all = "snake_case")]
pub enum MemoryRevalidationOutcome {
    KeepCurrent,
    NeedsValidation,
    Supersede,
    Expire,
    RetainHistorical,
}

#[must_use]
pub fn memory_validate_callable() -> CallableId {
    CallableId::parse(MEMORY_VALIDATE_CALLABLE).expect("static memory callable id is valid")
}
