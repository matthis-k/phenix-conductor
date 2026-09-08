use crate::{CallableRef, PhenixValue, Type, ValueError};

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
}
