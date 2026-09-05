use crate::{extension_name, wire, ApplicationAdapter};
use phenix_application_interface::types::ApplicationError;
use phenix_application_interface::{
    application_descriptor, ActivateSkill, ApplicationTransport, Authenticate,
    DiscoverAuthentication, GetDiagnostics, GetExecutionTree, GetLineage, GetProvenance,
    InvokeCallable, ListCallables, ListSkills, Operation, RenameSession,
};
use phenix_core::{ContractId, PhenixValue, ValueCodec};
use std::sync::Arc;
use wire::schema::v1::{ExtRequest, ExtResponse};

impl<T: ApplicationTransport> ApplicationAdapter<T> {
    pub async fn extension_request(
        &self,
        request: ExtRequest,
    ) -> Result<ExtResponse, ApplicationError> {
        macro_rules! dispatch {
            ($($operation:ty),+ $(,)?) => {
                $(
                    if extension_matches::<$operation>(request.method.as_ref()) {
                        return invoke_extension::<T, $operation>(self, &request).await;
                    }
                )+
            };
        }

        dispatch!(
            DiscoverAuthentication,
            Authenticate,
            RenameSession,
            GetLineage,
            ListSkills,
            ActivateSkill,
            ListCallables,
            InvokeCallable,
            GetExecutionTree,
            GetProvenance,
            GetDiagnostics,
        );

        Err(ApplicationError::InvalidInput {
            message: format!("unsupported ACP extension method {}", request.method),
        })
    }
}

async fn invoke_extension<T, O>(
    adapter: &ApplicationAdapter<T>,
    request: &ExtRequest,
) -> Result<ExtResponse, ApplicationError>
where
    T: ApplicationTransport,
    O: Operation,
{
    let descriptor = application_descriptor();
    let operation = ContractId::parse(O::ID).expect("static application operation id is valid");
    let declaration = descriptor
        .operations
        .get(&operation)
        .expect("typed application operation is present in the fixed descriptor");
    let input_schema = descriptor
        .types
        .get(&declaration.input)
        .expect("application operation input type is present in the fixed descriptor");
    let output_schema = descriptor
        .types
        .get(&declaration.output)
        .expect("application operation output type is present in the fixed descriptor");

    let value = serde_json::from_str::<PhenixValue>(request.params.get()).map_err(|error| {
        ApplicationError::InvalidInput {
            message: format!("cannot decode ACP extension input: {error}"),
        }
    })?;
    input_schema
        .parse(&value)
        .map_err(|error| ApplicationError::InvalidInput {
            message: format!("ACP extension input violates the application descriptor: {error}"),
        })?;
    let input = O::Input::from_value(&value).map_err(|error| ApplicationError::InvalidInput {
        message: format!("cannot decode typed application extension input: {error}"),
    })?;
    let output = adapter.invoke_application::<O>(input).await?;
    let value = output.to_value();
    output_schema
        .parse(&value)
        .map_err(|error| ApplicationError::InvalidResponse {
            message: format!("application extension output violates the descriptor: {error}"),
        })?;
    let raw = serde_json::value::to_raw_value(&value).map_err(|error| {
        ApplicationError::InvalidResponse {
            message: format!("cannot encode ACP extension output: {error}"),
        }
    })?;
    Ok(ExtResponse::new(Arc::from(raw)))
}

fn extension_matches<O: Operation>(method: &str) -> bool {
    let operation = ContractId::parse(O::ID).expect("static application operation id is valid");
    method == extension_name(&operation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_application_interface::types::{Empty, SessionInfo, SessionRenameInput};
    use phenix_core::SessionId;
    use std::{cell::RefCell, future::ready, rc::Rc};

    type Calls = Rc<RefCell<Vec<(ContractId, PhenixValue)>>>;

    struct TestTransport {
        calls: Calls,
        response: PhenixValue,
    }

    impl ApplicationTransport for TestTransport {
        fn invoke(
            &self,
            operation: &ContractId,
            input: PhenixValue,
        ) -> impl std::future::Future<Output = Result<PhenixValue, ApplicationError>> {
            self.calls.borrow_mut().push((operation.clone(), input));
            ready(Ok(self.response.clone()))
        }
    }

    fn contract(value: &str) -> ContractId {
        ContractId::parse(value).expect("valid contract id")
    }

    fn adapter(
        response: PhenixValue,
        extra: &[&str],
    ) -> (ApplicationAdapter<TestTransport>, Calls) {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let advertised = [
            "phenix.application.capability.discovery@1",
            "phenix.application.capability.sessions@1",
            "phenix.application.capability.prompt@1",
        ]
        .into_iter()
        .chain(extra.iter().copied())
        .map(contract)
        .collect::<Vec<_>>();
        let adapter = ApplicationAdapter::new(
            TestTransport {
                calls: calls.clone(),
                response,
            },
            advertised,
        )
        .expect("adapter capabilities");
        (adapter, calls)
    }

    fn request(method: &str, input: PhenixValue) -> ExtRequest {
        let raw = serde_json::value::to_raw_value(&input).expect("extension input JSON");
        ExtRequest::new(method, Arc::from(raw))
    }

    fn rename_input() -> SessionRenameInput {
        SessionRenameInput {
            session_id: SessionId::parse("session-1").expect("session id"),
            title: "Renamed".to_owned(),
        }
    }

    #[tokio::test]
    async fn extension_dispatch_invokes_the_typed_application_operation() {
        let input = rename_input();
        let output = SessionInfo {
            session_id: input.session_id.clone(),
            title: Some("Renamed".to_owned()),
            working_directory: "/workspace".to_owned(),
        };
        let (adapter, calls) = adapter(output.to_value(), &[RenameSession::CAPABILITY]);

        let response = adapter
            .extension_request(request("_phenix/session-rename@1", input.to_value()))
            .await
            .expect("extension response");

        assert_eq!(
            serde_json::to_value(response).expect("response JSON"),
            serde_json::to_value(output.to_value()).expect("expected JSON")
        );
        let calls = calls.borrow();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, contract(RenameSession::ID));
        assert_eq!(calls[0].1, input.to_value());
    }

    #[tokio::test]
    async fn extension_dispatch_preserves_capability_rejection() {
        let (adapter, calls) = adapter(Empty {}.to_value(), &[]);
        let error = adapter
            .extension_request(request("_phenix/diagnostics@1", Empty {}.to_value()))
            .await
            .expect_err("diagnostics must require its capability");

        assert_eq!(
            error,
            ApplicationError::UnsupportedCapability {
                capability: contract(GetDiagnostics::CAPABILITY),
            }
        );
        assert!(calls.borrow().is_empty());
    }

    #[tokio::test]
    async fn extension_dispatch_validates_input_before_transport() {
        let (adapter, calls) = adapter(Empty {}.to_value(), &[RenameSession::CAPABILITY]);
        let error = adapter
            .extension_request(request("_phenix/session-rename@1", Empty {}.to_value()))
            .await
            .expect_err("wrong input shape must fail");

        assert!(matches!(error, ApplicationError::InvalidInput { .. }));
        assert!(calls.borrow().is_empty());
    }

    #[tokio::test]
    async fn extension_dispatch_rejects_invalid_runtime_output() {
        let (adapter, calls) = adapter(Empty {}.to_value(), &[RenameSession::CAPABILITY]);
        let error = adapter
            .extension_request(request(
                "_phenix/session-rename@1",
                rename_input().to_value(),
            ))
            .await
            .expect_err("wrong output shape must fail");

        assert!(matches!(error, ApplicationError::InvalidResponse { .. }));
        assert_eq!(calls.borrow().len(), 1);
    }

    #[tokio::test]
    async fn extension_dispatch_rejects_unknown_methods_without_transport() {
        let (adapter, calls) = adapter(Empty {}.to_value(), &[]);
        let error = adapter
            .extension_request(request("_phenix/not-an-operation@1", Empty {}.to_value()))
            .await
            .expect_err("unknown extension method must fail");

        assert!(matches!(error, ApplicationError::InvalidInput { .. }));
        assert!(calls.borrow().is_empty());
    }
}
