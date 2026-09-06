#![forbid(unsafe_code)]

//! Generated, typed application API for ACP-facing Phenix clients.
//!
//! The application descriptor owns identifiers and structural payloads. This crate
//! owns the public client façade; ACP transport and connection lifecycle remain
//! handwritten and are added below this stable generated API.

use phenix_application_interface::{
    ApplicationClient, ApplicationTransport, Capabilities, Operation,
};

pub use phenix_application_interface::{application_descriptor, types::ApplicationError, INTERFACE_ID};

/// Types, capabilities, event/callback identities, and operation wrappers generated
/// from the fixed application descriptor at build time.
pub mod generated {
    include!(concat!(env!("OUT_DIR"), "/application.rs"));
}

pub use generated::*;

/// Capability-checked typed application client.
///
/// The contained transport is deliberately protocol-neutral. ACP connection,
/// request correlation, and stream lifecycle are layered in handwritten code
/// without changing descriptor-generated public types.
pub struct Client<T> {
    application: ApplicationClient<T>,
}

impl<T: ApplicationTransport> Client<T> {
    #[must_use]
    pub fn new(transport: T, capabilities: Capabilities) -> Self {
        Self {
            application: ApplicationClient::new(transport, capabilities),
        }
    }

    #[must_use]
    pub fn application(&self) -> &ApplicationClient<T> {
        &self.application
    }

    pub async fn invoke<O: Operation>(
        &self,
        input: O::Input,
    ) -> Result<O::Output, ApplicationError> {
        self.application.invoke::<O>(input).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_api_reports_the_fixed_interface_identity() {
        assert_eq!(generated::INTERFACE_ID, INTERFACE_ID);
        assert!(generated::type_schemas().contains_key(
            &phenix_core::ContractId::parse("phenix.application.error@1")
                .expect("static contract id is valid"),
        ));
    }

    #[test]
    fn generated_source_is_deterministic_for_the_fixed_descriptor() {
        let first = phenix_application_interface::generate::rust(&application_descriptor())
            .expect("fixed descriptor is generatable");
        let second = phenix_application_interface::generate::rust(&application_descriptor())
            .expect("fixed descriptor is generatable");
        assert_eq!(first, second);
    }
}
