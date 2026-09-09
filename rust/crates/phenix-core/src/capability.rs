use crate::{
    CallableRef, CapabilityGenerationId, CapabilityOwnerId, PhenixValue, ReferenceId, Type,
    ValueError,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt::{self, Display, Formatter},
    sync::Arc,
};

/// The transport-neutral input to a capability invocation.
#[derive(Clone, Debug, PartialEq)]
pub struct CapabilityInvokeInput {
    pub callable: CallableRef,
    pub input: PhenixValue,
}

/// The transport-neutral successful result of a capability invocation.
#[derive(Clone, Debug, PartialEq)]
pub struct CapabilityInvokeResult {
    pub output: PhenixValue,
}

impl CapabilityInvokeInput {
    /// Validates the reference contract and input before provider code may run.
    pub fn validate(&self, callable_schema: &Type) -> Result<(), ValueError> {
        let Type::Callable {
            contract, input, ..
        } = callable_schema
        else {
            return Err(ValueError::InvalidValue(
                "capability invocation requires a callable schema".to_owned(),
            ));
        };
        if self.callable.contract() != contract {
            return Err(ValueError::InvalidValue(format!(
                "callable reference contract {} does not match schema contract {contract}",
                self.callable.contract()
            )));
        }
        input.parse(&self.input)
    }
}

impl CapabilityInvokeResult {
    /// Validates provider output before it crosses the capability boundary.
    pub fn validate(&self, callable_schema: &Type) -> Result<(), ValueError> {
        let Type::Callable { output, .. } = callable_schema else {
            return Err(ValueError::InvalidValue(
                "capability invocation requires a callable schema".to_owned(),
            ));
        };
        output.parse(&self.output)
    }
}

/// The owner-side implementation of a callable capability.
///
/// The registry validates both sides of every call. Handlers therefore receive
/// only values that satisfy their declared input schema and cannot publish an
/// unchecked output.
pub trait CapabilityHandler: Send + Sync {
    fn invoke(&self, input: PhenixValue) -> Result<PhenixValue, CapabilityError>;
}

impl<F> CapabilityHandler for F
where
    F: Fn(PhenixValue) -> Result<PhenixValue, CapabilityError> + Send + Sync,
{
    fn invoke(&self, input: PhenixValue) -> Result<PhenixValue, CapabilityError> {
        self(input)
    }
}

#[derive(Clone)]
struct RegisteredCapability {
    schema: Type,
    handler: Arc<dyn CapabilityHandler>,
}

type CapabilityKey = (CapabilityOwnerId, CapabilityGenerationId, ReferenceId);

/// Process-local capability dispatch keyed by opaque owner, generation, and
/// reference identities. Transport adapters retain ownership of client queues;
/// they register a handler that forwards the canonical invocation unchanged.
#[derive(Default)]
pub struct CapabilityRegistry {
    entries: BTreeMap<CapabilityKey, RegisteredCapability>,
    retired: BTreeSet<(CapabilityOwnerId, CapabilityGenerationId)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CapabilityError {
    UnknownReference(CallableRef),
    StaleReference(CallableRef),
    SchemaMismatch { message: String },
    ProviderFailed { message: String },
    Cancelled,
    Disconnected,
    QueueFull,
}

impl Display for CapabilityError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownReference(reference) => {
                write!(formatter, "unknown capability reference {}", reference.id())
            }
            Self::StaleReference(reference) => {
                write!(formatter, "stale capability reference {}", reference.id())
            }
            Self::SchemaMismatch { message } => {
                write!(formatter, "capability schema mismatch: {message}")
            }
            Self::ProviderFailed { message } => {
                write!(formatter, "capability provider failed: {message}")
            }
            Self::Cancelled => formatter.write_str("capability invocation cancelled"),
            Self::Disconnected => formatter.write_str("capability provider disconnected"),
            Self::QueueFull => formatter.write_str("capability provider queue is full"),
        }
    }
}

impl Error for CapabilityError {}

impl CapabilityRegistry {
    /// Registers one callable for its exact owner generation.
    pub fn register(
        &mut self,
        reference: CallableRef,
        schema: Type,
        handler: impl CapabilityHandler + 'static,
    ) -> Result<(), CapabilityError> {
        let Type::Callable { contract, .. } = &schema else {
            return Err(CapabilityError::SchemaMismatch {
                message: "registered capability schema is not callable".to_owned(),
            });
        };
        if reference.contract() != contract {
            return Err(CapabilityError::SchemaMismatch {
                message: format!(
                    "reference contract {} does not match schema contract {contract}",
                    reference.contract()
                ),
            });
        }
        let key = (
            reference.owner().clone(),
            reference.generation().clone(),
            reference.id().clone(),
        );
        if self.retired.contains(&(key.0.clone(), key.1.clone())) {
            return Err(CapabilityError::StaleReference(reference));
        }
        self.entries.insert(
            key,
            RegisteredCapability {
                schema,
                handler: Arc::new(handler),
            },
        );
        Ok(())
    }

    /// Makes every reference from an owner generation permanently stale.
    pub fn retire(&mut self, owner: CapabilityOwnerId, generation: CapabilityGenerationId) {
        self.entries
            .retain(|(entry_owner, entry_generation, _), _| {
                entry_owner != &owner || entry_generation != &generation
            });
        self.retired.insert((owner, generation));
    }

    /// Invokes one reference after input validation and before output release.
    pub fn invoke(
        &self,
        invocation: CapabilityInvokeInput,
    ) -> Result<CapabilityInvokeResult, CapabilityError> {
        let reference = &invocation.callable;
        let owner_generation = (reference.owner().clone(), reference.generation().clone());
        if self.retired.contains(&owner_generation) {
            return Err(CapabilityError::StaleReference(reference.clone()));
        }
        let key = (
            reference.owner().clone(),
            reference.generation().clone(),
            reference.id().clone(),
        );
        let entry = self
            .entries
            .get(&key)
            .ok_or_else(|| CapabilityError::UnknownReference(reference.clone()))?;
        invocation
            .validate(&entry.schema)
            .map_err(|error| CapabilityError::SchemaMismatch {
                message: error.to_string(),
            })?;
        let result = CapabilityInvokeResult {
            output: entry.handler.invoke(invocation.input)?,
        };
        result
            .validate(&entry.schema)
            .map_err(|error| CapabilityError::SchemaMismatch {
                message: error.to_string(),
            })?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CapabilityGenerationId, CapabilityOwnerId, ClientConnectionId, ContractId, ReferenceId,
    };

    fn schema() -> Type {
        Type::Callable {
            contract: ContractId::parse("fixture.callback@1").unwrap(),
            input: Box::new(Type::U64),
            output: Box::new(Type::String),
        }
    }

    fn reference() -> CallableRef {
        CallableRef::new(
            ContractId::parse("fixture.callback@1").unwrap(),
            CapabilityOwnerId::Client(ClientConnectionId::parse("fixture.client").unwrap()),
            CapabilityGenerationId::parse("fixture.generation").unwrap(),
            ReferenceId::parse("fixture.callback").unwrap(),
        )
    }

    #[test]
    fn invocation_validates_contract_and_input_before_dispatch() {
        let invocation = CapabilityInvokeInput {
            callable: reference(),
            input: PhenixValue::U64(1),
        };
        invocation.validate(&schema()).unwrap();

        let wrong_input = CapabilityInvokeInput {
            callable: reference(),
            input: PhenixValue::String("wrong".to_owned()),
        };
        assert!(wrong_input.validate(&schema()).is_err());
    }

    #[test]
    fn invocation_validates_output_before_returning_to_the_caller() {
        CapabilityInvokeResult {
            output: PhenixValue::String("ok".to_owned()),
        }
        .validate(&schema())
        .unwrap();
        assert!(CapabilityInvokeResult {
            output: PhenixValue::U64(1),
        }
        .validate(&schema())
        .is_err());
    }

    #[test]
    fn registry_validates_both_sides_of_a_plugin_or_client_capability_call() {
        let reference = reference();
        let mut registry = CapabilityRegistry::default();
        registry
            .register(reference.clone(), schema(), |input| match input {
                PhenixValue::U64(value) => Ok(PhenixValue::String(value.to_string())),
                _ => unreachable!("registry validates input before handler execution"),
            })
            .unwrap();

        let result = registry
            .invoke(CapabilityInvokeInput {
                callable: reference.clone(),
                input: PhenixValue::U64(7),
            })
            .unwrap();
        assert_eq!(result.output, PhenixValue::String("7".to_owned()));

        let error = registry
            .invoke(CapabilityInvokeInput {
                callable: reference,
                input: PhenixValue::String("wrong".to_owned()),
            })
            .unwrap_err();
        assert!(matches!(error, CapabilityError::SchemaMismatch { .. }));
    }

    #[test]
    fn retired_owner_generation_is_structurally_stale() {
        let reference = reference();
        let mut registry = CapabilityRegistry::default();
        registry
            .register(reference.clone(), schema(), |_| {
                Ok(PhenixValue::String("ok".to_owned()))
            })
            .unwrap();
        registry.retire(reference.owner().clone(), reference.generation().clone());

        assert_eq!(
            registry
                .invoke(CapabilityInvokeInput {
                    callable: reference.clone(),
                    input: PhenixValue::U64(1),
                })
                .unwrap_err(),
            CapabilityError::StaleReference(reference)
        );
    }

    #[test]
    fn invalid_provider_output_never_reaches_the_caller() {
        let reference = reference();
        let mut registry = CapabilityRegistry::default();
        registry
            .register(reference.clone(), schema(), |_| Ok(PhenixValue::U64(1)))
            .unwrap();

        assert!(matches!(
            registry.invoke(CapabilityInvokeInput {
                callable: reference,
                input: PhenixValue::U64(1),
            }),
            Err(CapabilityError::SchemaMismatch { .. })
        ));
    }
}
