use crate::{descriptor::id, types::ApplicationError, ApplicationDescriptor};
use phenix_core::{ContractId, PhenixContract, PhenixValue, ValueCodec};
use std::{collections::BTreeSet, future::Future};

pub trait Operation {
    const ID: &'static str;
    const CAPABILITY: &'static str;
    type Input: PhenixContract + ValueCodec;
    type Output: PhenixContract + ValueCodec;
}

/// Application feature support, never execution authority.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Capabilities(BTreeSet<ContractId>);

impl Capabilities {
    /// Unknown versions are ignored. An advertised feature must include its dependencies.
    pub fn negotiate(
        descriptor: &ApplicationDescriptor,
        advertised: impl IntoIterator<Item = ContractId>,
    ) -> Result<Self, ApplicationError> {
        let supported: BTreeSet<_> = advertised
            .into_iter()
            .filter(|capability| descriptor.capabilities.contains_key(capability))
            .collect();
        for capability in &supported {
            for dependency in &descriptor.capabilities[capability].dependencies {
                if !supported.contains(dependency) {
                    return Err(ApplicationError::UnsupportedCapability {
                        capability: dependency.clone(),
                    });
                }
            }
        }
        Ok(Self(supported))
    }

    pub fn require(&self, capability: &ContractId) -> Result<(), ApplicationError> {
        if self.0.contains(capability) {
            return Ok(());
        }
        Err(ApplicationError::UnsupportedCapability {
            capability: capability.clone(),
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = &ContractId> {
        self.0.iter()
    }
}

/// Handwritten adapters own connection lifecycle and protocol semantics behind this boundary.
/// It imposes no executor, transport, runtime implementation, or Send requirement.
pub trait ApplicationTransport {
    fn invoke(
        &self,
        operation: &ContractId,
        input: PhenixValue,
    ) -> impl Future<Output = Result<PhenixValue, ApplicationError>>;
}

pub struct ApplicationClient<T> {
    transport: T,
    capabilities: Capabilities,
}

impl<T: ApplicationTransport> ApplicationClient<T> {
    pub fn new(transport: T, capabilities: Capabilities) -> Self {
        Self {
            transport,
            capabilities,
        }
    }

    pub async fn invoke<O: Operation>(
        &self,
        input: O::Input,
    ) -> Result<O::Output, ApplicationError> {
        self.capabilities.require(&id(O::CAPABILITY))?;
        let response = self.transport.invoke(&id(O::ID), input.to_value()).await?;
        O::Output::from_value(&response).map_err(|error| ApplicationError::InvalidResponse {
            message: error.to_string(),
        })
    }
}
