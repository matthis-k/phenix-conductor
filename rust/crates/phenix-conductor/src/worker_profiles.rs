use phenix_core::{AgentDefinition, CallableId, ExecutionAuthority};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkerProfileId(String);

impl WorkerProfileId {
    pub fn parse(value: impl Into<String>) -> Result<Self, WorkerProfileError> {
        let value = value.into();
        if value.trim().is_empty() {
            Err(WorkerProfileError::InvalidId)
        } else {
            Ok(Self(value))
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for WorkerProfileId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerProfileDefinition {
    pub id: WorkerProfileId,
    pub role: String,
    pub agent: CallableId,
    pub authority_maximum: ExecutionAuthority,
}

#[derive(Clone, Debug)]
pub struct ResolvedWorkerProfile<'a> {
    pub profile: &'a WorkerProfileDefinition,
    pub agent: &'a AgentDefinition,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorkerProfileError {
    InvalidId,
    Duplicate(WorkerProfileId),
    Unknown(WorkerProfileId),
    InvalidAgent {
        profile: WorkerProfileId,
        agent: CallableId,
    },
}

impl Display for WorkerProfileError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidId => f.write_str("worker profile identifier must not be empty"),
            Self::Duplicate(id) => write!(f, "worker profile already registered: {id}"),
            Self::Unknown(id) => write!(f, "unknown worker profile: {id}"),
            Self::InvalidAgent { profile, agent } => {
                write!(f, "worker profile {profile} references non-agent callable {agent}")
            }
        }
    }
}

impl std::error::Error for WorkerProfileError {}

#[derive(Clone, Debug, Default)]
pub(crate) struct WorkerProfileRegistry {
    profiles: BTreeMap<WorkerProfileId, WorkerProfileDefinition>,
}

impl WorkerProfileRegistry {
    pub(crate) fn register(
        &mut self,
        profile: WorkerProfileDefinition,
    ) -> Result<(), WorkerProfileError> {
        if self.profiles.contains_key(&profile.id) {
            return Err(WorkerProfileError::Duplicate(profile.id));
        }
        self.profiles.insert(profile.id.clone(), profile);
        Ok(())
    }

    pub(crate) fn get(
        &self,
        id: &WorkerProfileId,
    ) -> Result<&WorkerProfileDefinition, WorkerProfileError> {
        self.profiles
            .get(id)
            .ok_or_else(|| WorkerProfileError::Unknown(id.clone()))
    }

    pub(crate) fn semantic_manifest(&self) -> Value {
        Value::Array(
            self.profiles
                .values()
                .map(|profile| json!(profile))
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_id_rejects_empty_values() {
        assert_eq!(WorkerProfileId::parse(" "), Err(WorkerProfileError::InvalidId));
    }
}
