use phenix_core::{ContractId, PhenixContract, PhenixSchema};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Every reference is a versioned semantic identity, independent of Rust symbol names.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationDescriptor {
    pub id: ContractId,
    pub operations: BTreeMap<ContractId, OperationDescriptor>,
    pub events: BTreeMap<ContractId, EventDescriptor>,
    pub callbacks: BTreeMap<ContractId, CallbackDescriptor>,
    pub capabilities: BTreeMap<ContractId, CapabilityDescriptor>,
    pub types: BTreeMap<ContractId, PhenixSchema>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationDescriptor {
    pub input: ContractId,
    pub output: ContractId,
    pub error: ContractId,
    pub capability: ContractId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventDescriptor {
    pub payload: ContractId,
    pub ordering: OrderingScope,
    pub capability: ContractId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderingScope {
    Session,
    Execution,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CallbackDescriptor {
    pub request: ContractId,
    pub response: ContractId,
    pub capability: ContractId,
    pub semantics: CallbackSemantics,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CallbackSemantics {
    /// A response supplies data. The runtime still applies the caller's existing authority.
    Data,
    /// Consent applies to the named invocation only; it never grants ambient authority.
    InvocationConsent,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityDescriptor {
    pub dependencies: BTreeSet<ContractId>,
}

impl ApplicationDescriptor {
    /// BTree collections fix entry order; PhenixSchema fixes nested structural key order.
    pub fn canonical_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self).map(|json| json + "\n")
    }

    pub(crate) fn register<T: PhenixContract>(&mut self) -> ContractId {
        let id = T::contract_id();
        let schema = T::phenix_schema();
        if let Some(previous) = self.types.insert(id.clone(), schema.clone()) {
            assert_eq!(
                previous, schema,
                "static application type ids must be unique"
            );
        }
        id
    }
}

pub(crate) fn id(value: &str) -> ContractId {
    ContractId::parse(value).expect("static application contract id is valid")
}
