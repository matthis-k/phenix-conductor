#![forbid(unsafe_code)]

//! ACP stdio transport for the fixed Phenix application interface.
//!
//! Protocol translation stays in `phenix-adapter-acp`. This crate owns only
//! process transport and the channel boundary used by the configured runtime.

use agent_client_protocol::{schema::v1::*, Agent, Error, Stdio};
use phenix_adapter_acp::ApplicationAdapter;
use phenix_application_interface::{types::ApplicationError, ApplicationTransport};
use phenix_core::{ContractId, PhenixValue};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};

pub struct ApplicationInvocation {
    pub operation: ContractId,
    pub input: PhenixValue,
    response: oneshot::Sender<Result<PhenixValue, ApplicationError>>,
}

impl ApplicationInvocation {
    pub fn respond(self, response: Result<PhenixValue, ApplicationError>) {
        let _ = self.response.send(response);
    }
}

#[derive(Clone)]
pub struct ChannelTransport {
    sender: mpsc::Sender<ApplicationInvocation>,
}

impl ChannelTransport {
    #[must_use]
    pub fn new(capacity: usize) -> (Self, mpsc::Receiver<ApplicationInvocation>) {
        let (sender, receiver) = mpsc::channel(capacity);
        (Self { sender }, receiver)
    }
}

impl ApplicationTransport for ChannelTransport {
    fn invoke(
        &self,
        operation: &ContractId,
        input: PhenixValue,
    ) -> impl std::future::Future<Output = Result<PhenixValue, ApplicationError>> {
        let sender = self.sender.clone();
        let operation = operation.clone();
        async move {
            let (response, receive) = oneshot::channel();
            sender
                .send(ApplicationInvocation {
                    operation,
                    input,
                    response,
                })
                .await
                .map_err(|_| ApplicationError::Disconnected)?;
            receive.await.map_err(|_| ApplicationError::Disconnected)?
        }
    }
}

pub async fn serve_stdio(
    transport: ChannelTransport,
    advertised: impl IntoIterator<Item = ContractId>,
) -> Result<(), Error> {
    let adapter =
        Arc::new(ApplicationAdapter::new(transport, advertised).map_err(application_error_to_acp)?);

    let initialize = Arc::clone(&adapter);
    let new_session = Arc::clone(&adapter);
    let list_sessions = Arc::clone(&adapter);
    let resume_session = Arc::clone(&adapter);
    let load_session = Arc::clone(&adapter);
    let close_session = Arc::clone(&adapter);
    let prompt = Arc::clone(&adapter);
    let cancel = Arc::clone(&adapter);
    let set_config = Arc::clone(&adapter);

    Agent
        .builder()
        .name("phenix-acp")
        .on_receive_request(
            async move |request: InitializeRequest, responder, _cx| {
                responder.respond(initialize.initialize(request))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: NewSessionRequest, responder, _cx| {
                let response = new_session
                    .new_session(request)
                    .await
                    .map_err(application_error_to_acp)?;
                responder.respond(response)
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: ListSessionsRequest, responder, _cx| {
                let response = list_sessions
                    .list_sessions(request)
                    .await
                    .map_err(application_error_to_acp)?;
                responder.respond(response)
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: ResumeSessionRequest, responder, _cx| {
                let response = resume_session
                    .resume_session(request)
                    .await
                    .map_err(application_error_to_acp)?;
                responder.respond(response)
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: LoadSessionRequest, responder, _cx| {
                let loaded = load_session
                    .load_session(request)
                    .await
                    .map_err(application_error_to_acp)?;
                responder.respond(loaded.response)
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: CloseSessionRequest, responder, _cx| {
                let response = close_session
                    .close_session(request)
                    .await
                    .map_err(application_error_to_acp)?;
                responder.respond(response)
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: PromptRequest, responder, _cx| {
                let response = prompt
                    .prompt(request)
                    .await
                    .map_err(application_error_to_acp)?;
                responder.respond(response)
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: SetSessionConfigOptionRequest, responder, _cx| {
                let response = set_config
                    .set_session_config_option(request)
                    .await
                    .map_err(application_error_to_acp)?;
                responder.respond(response)
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_notification(
            async move |notification: CancelNotification, _cx| {
                cancel
                    .cancel(notification)
                    .await
                    .map_err(application_error_to_acp)
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .connect_to(Stdio::new())
        .await
}

fn application_error_to_acp(error: ApplicationError) -> Error {
    match error {
        ApplicationError::UnsupportedCapability { capability } => Error::method_not_found().data(
            format!("unsupported Phenix application capability: {capability}"),
        ),
        ApplicationError::InvalidInput { message } => Error::invalid_params().data(message),
        ApplicationError::InvalidResponse { message } => Error::internal_error().data(message),
        ApplicationError::NotFound { resource } => Error::resource_not_found(Some(resource)),
        ApplicationError::Unauthenticated { message } => Error::auth_required().data(message),
        ApplicationError::PermissionDenied { message }
        | ApplicationError::Conflict { message }
        | ApplicationError::Failed { message } => Error::internal_error().data(message),
        ApplicationError::Cancelled => Error::request_cancelled(),
        ApplicationError::Disconnected => Error::internal_error().data("application disconnected"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn channel_transport_preserves_typed_operation_and_response() {
        let (transport, mut receiver) = ChannelTransport::new(1);
        let operation = ContractId::parse("phenix.application.session-create@1").unwrap();
        let expected_operation = operation.clone();
        let worker = tokio::spawn(async move {
            let invocation = receiver.recv().await.unwrap();
            assert_eq!(invocation.operation, expected_operation);
            assert_eq!(invocation.input, PhenixValue::String("input".to_owned()));
            invocation.respond(Ok(PhenixValue::String("output".to_owned())));
        });

        let output = transport
            .invoke(&operation, PhenixValue::String("input".to_owned()))
            .await
            .unwrap();
        assert_eq!(output, PhenixValue::String("output".to_owned()));
        worker.await.unwrap();
    }
}
