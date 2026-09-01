use phenix_core::{ComponentInterface, InterfaceId, InterfaceSchema};
use phenix_sdk_macros::PhenixValue;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const JOB_SERVICE: &str = "phenix.jobs@1";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, PhenixValue)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeResourceKind {
    Terminal,
    Job,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, PhenixValue)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeResourceState {
    Running,
    Exited { code: Option<i32> },
    Revoked { reason: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, PhenixValue)]
pub struct RuntimeResourceRecord {
    pub id: String,
    pub kind: RuntimeResourceKind,
    pub owner_execution: String,
    pub promoted_to_workspace: bool,
    pub authority: BTreeSet<String>,
    pub state: RuntimeResourceState,
    pub output_references: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, PhenixValue)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum JobCommand {
    Create {
        id: String,
        kind: RuntimeResourceKind,
        owner_execution: String,
        authority: BTreeSet<String>,
    },
    Promote {
        id: String,
    },
    Complete {
        id: String,
        code: Option<i32>,
        output_references: Vec<String>,
    },
    ExecutionTerminated {
        execution_id: String,
    },
    NarrowAuthority {
        execution_id: String,
        authority: BTreeSet<String>,
    },
    Get {
        id: String,
    },
    List,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, PhenixValue)]
#[serde(tag = "response", rename_all = "snake_case")]
pub enum JobResponse {
    Resource {
        resource: Option<RuntimeResourceRecord>,
    },
    Resources {
        resources: Vec<RuntimeResourceRecord>,
    },
    Affected {
        resources: Vec<RuntimeResourceRecord>,
    },
}

pub struct JobInterface;

impl ComponentInterface for JobInterface {
    fn interface_id() -> InterfaceId {
        InterfaceId::parse(JOB_SERVICE).expect("static job interface id is valid")
    }

    fn schema() -> InterfaceSchema {
        InterfaceSchema::of::<JobCommand, JobResponse>()
    }
}
