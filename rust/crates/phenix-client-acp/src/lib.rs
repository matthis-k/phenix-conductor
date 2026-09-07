#![forbid(unsafe_code)]

use phenix_application_interface::{
    ApplicationClient, ApplicationTransport, Capabilities, Operation,
};

pub use phenix_application_interface::{
    application_descriptor, types::ApplicationError, INTERFACE_ID,
};

pub mod generated {
    include!(concat!(env!("OUT_DIR"), "/application.rs"));
}

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
    use phenix_core::{ContractId, PhenixContract, PhenixValue};

    #[derive(phenix_sdk_macros::PhenixValue)]
    struct Request;

    impl PhenixContract for Request {
        fn contract_id() -> ContractId {
            ContractId::parse("fixture.client.request@1").expect("static contract id is valid")
        }
    }

    #[derive(Debug, PartialEq, phenix_sdk_macros::PhenixValue)]
    struct Response;

    impl PhenixContract for Response {
        fn contract_id() -> ContractId {
            ContractId::parse("fixture.client.response@1").expect("static contract id is valid")
        }
    }

    struct FixtureOperation;

    impl Operation for FixtureOperation {
        const ID: &'static str = "phenix.application.capabilities@1";
        const CAPABILITY: &'static str = "phenix.application.capability.discovery@1";
        type Input = Request;
        type Output = Response;
    }

    #[derive(Clone)]
    struct RejectingTransport(ApplicationError);

    impl ApplicationTransport for RejectingTransport {
        fn invoke(
            &self,
            _operation: &ContractId,
            _input: PhenixValue,
        ) -> impl std::future::Future<Output = Result<PhenixValue, ApplicationError>> {
            std::future::ready(Err(self.0.clone()))
        }
    }

    fn client(error: ApplicationError) -> Client<RejectingTransport> {
        let capability =
            ContractId::parse(FixtureOperation::CAPABILITY).expect("static capability id is valid");
        let capabilities = Capabilities::negotiate(&application_descriptor(), [capability])
            .expect("discovery has no missing dependency");
        Client::new(RejectingTransport(error), capabilities)
    }

    #[test]
    fn generated_api_reports_the_fixed_interface_identity() {
        assert_eq!(generated::INTERFACE_ID, INTERFACE_ID);
        assert_eq!(generated::DESCRIPTOR_SHA256.len(), 64);
        assert!(generated::type_schemas().contains_key(
            &ContractId::parse("phenix.application.error@1").expect("static contract id is valid"),
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

    #[test]
    fn typed_client_preserves_typed_application_failures() {
        let cases = [
            ApplicationError::Conflict {
                message: "same text".to_owned(),
            },
            ApplicationError::PermissionDenied {
                message: "same text".to_owned(),
            },
            ApplicationError::Cancelled,
            ApplicationError::Disconnected,
        ];

        for expected in cases {
            let result = futures::executor::block_on(
                client(expected.clone()).invoke::<FixtureOperation>(Request),
            );
            assert_eq!(result, Err(expected));
        }
    }
}
