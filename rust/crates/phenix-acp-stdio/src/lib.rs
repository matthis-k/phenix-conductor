#![forbid(unsafe_code)]

//! ACP stdio transport for the fixed Phenix application interface.
//!
//! Protocol translation stays in `phenix-adapter-acp`. This crate owns only
//! process transport and the channel boundary used by the configured runtime.

use agent_client_protocol::{schema::v1::*, Agent, Error, Stdio};
use phenix_adapter_acp::ApplicationAdapter;
use phenix_application_interface::{
    types::{
        ApplicationError, CapabilityInvokeInput as ApplicationCapabilityInvokeInput,
        CapabilityInvokeResult as ApplicationCapabilityInvokeResult, Empty,
        SdkValue as ApplicationSdkValue,
    },
    ApplicationTransport, GetSdk, InvokeCapability, Operation,
};
use phenix_core::{
    CapabilityGenerationId, CapabilityInvokeInput as CoreCapabilityInvokeInput, ContractId,
    ObservableStore, PhenixValue, ResolvedSdkContributions, RuntimeId, SharedCapabilityRegistry,
    ValueCodec,
};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};

pub struct ApplicationInvocation {
    pub operation: ContractId,
    pub input: PhenixValue,
    response: oneshot::Sender<Result<PhenixValue, ApplicationError>>,
}

pub struct ApplicationEvent {
    pub event: ContractId,
    pub payload: PhenixValue,
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

/// Live application handler for the value SDK operations owned by #503.
///
/// SDK materialization and generic invocation share one capability registry.
/// The service materializes the SDK once so repeated `GetSdk` calls return the
/// same callable identities instead of re-registering them.
#[derive(Clone)]
pub struct SdkApplicationService {
    sdk: ApplicationSdkValue,
    capabilities: SharedCapabilityRegistry,
}

impl SdkApplicationService {
    pub fn new(
        sdk: &ResolvedSdkContributions,
        store: &ObservableStore,
        capabilities: SharedCapabilityRegistry,
        runtime: RuntimeId,
        generation: CapabilityGenerationId,
    ) -> Result<Self, phenix_core::SdkResolutionError> {
        let sdk = sdk.value_with_observables(store, &capabilities, &runtime, generation)?;
        Ok(Self {
            sdk: ApplicationSdkValue {
                schema: sdk.schema,
                value: sdk.value,
            },
            capabilities,
        })
    }

    #[must_use]
    pub fn capabilities(&self) -> &SharedCapabilityRegistry {
        &self.capabilities
    }

    pub fn invoke(
        &self,
        operation: &ContractId,
        input: PhenixValue,
    ) -> Result<PhenixValue, ApplicationError> {
        if operation.as_str() == GetSdk::ID {
            Empty::from_value(&input).map_err(|error| ApplicationError::InvalidInput {
                message: error.to_string(),
            })?;
            return Ok(self.sdk.to_value());
        }
        if operation.as_str() == InvokeCapability::ID {
            let request = ApplicationCapabilityInvokeInput::from_value(&input).map_err(|error| {
                ApplicationError::InvalidInput {
                    message: error.to_string(),
                }
            })?;
            let PhenixValue::Callable(callable) = request.callable else {
                return Err(ApplicationError::InvalidInput {
                    message: "capability invocation requires a callable reference".to_owned(),
                });
            };
            let result = self.capabilities.invoke(CoreCapabilityInvokeInput {
                callable,
                input: request.input,
            })?;
            return Ok(ApplicationCapabilityInvokeResult {
                output: result.output,
            }
            .to_value());
        }
        Err(ApplicationError::InvalidInput {
            message: format!("unsupported SDK application operation {operation}"),
        })
    }

    pub fn handle(&self, invocation: ApplicationInvocation) {
        let response = self.invoke(&invocation.operation, invocation.input.clone());
        invocation.respond(response);
    }
}

pub async fn serve_sdk_application(
    service: SdkApplicationService,
    mut receiver: mpsc::Receiver<ApplicationInvocation>,
) {
    while let Some(invocation) = receiver.recv().await {
        service.handle(invocation);
    }
}

pub async fn serve_stdio(
    transport: ChannelTransport,
    advertised: impl IntoIterator<Item = ContractId>,
) -> Result<(), Error> {
    let (keep_events_open, events) = mpsc::channel(1);
    let result = serve_stdio_with_events(transport, advertised, events).await;
    drop(keep_events_open);
    result
}

pub async fn serve_stdio_with_events(
    transport: ChannelTransport,
    advertised: impl IntoIterator<Item = ContractId>,
    mut events: mpsc::Receiver<ApplicationEvent>,
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
    let event_adapter = Arc::clone(&adapter);

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
        .connect_with(
            Stdio::new(),
            move |connection: agent_client_protocol::ConnectionTo<agent_client_protocol::Client>| async move {
            loop {
                tokio::select! {
                    event = events.recv() => match event {
                        Some(event) => {
                            let notification = event_adapter
                                .extension_event(&event.event, &event.payload)
                                .map_err(application_error_to_acp)?;
                            connection.send_notification(AgentNotification::ExtNotification(notification))?;
                        }
                        None => return Ok(()),
                    },
                    () = connection.incoming_closed() => return Ok(()),
                }
            }
        },
        )
        .await
}

fn application_error_to_acp(error: ApplicationError) -> Error {
    let data = json!({
        "phenix.class": error.class(),
        "phenix.details": application_error_details(&error),
    });
    let error = match &error {
        ApplicationError::UnsupportedCapability { .. } => Error::method_not_found(),
        ApplicationError::InvalidInput { .. } => Error::invalid_params(),
        ApplicationError::InvalidResponse { .. }
        | ApplicationError::PermissionDenied { .. }
        | ApplicationError::Conflict { .. }
        | ApplicationError::Failed { .. }
        | ApplicationError::Disconnected => Error::internal_error(),
        ApplicationError::NotFound { resource } => {
            Error::resource_not_found(Some(resource.clone()))
        }
        ApplicationError::UnknownValue { value } | ApplicationError::StaleReference { value } => {
            Error::resource_not_found(Some(value.clone()))
        }
        ApplicationError::InvalidPath { .. } | ApplicationError::SchemaMismatch { .. } => {
            Error::invalid_params()
        }
        ApplicationError::UnsupportedSnapshotPolicy { .. }
        | ApplicationError::TransactionConflict { .. }
        | ApplicationError::SubscriptionCapacity
        | ApplicationError::Closed => Error::internal_error(),
        ApplicationError::Unauthenticated { .. } => Error::auth_required(),
        ApplicationError::Cancelled => Error::request_cancelled(),
    };
    error.data(data)
}

fn application_error_details(error: &ApplicationError) -> serde_json::Value {
    match error {
        ApplicationError::UnsupportedCapability { capability } => {
            json!({ "capability": capability.as_str() })
        }
        ApplicationError::InvalidInput { message }
        | ApplicationError::InvalidResponse { message }
        | ApplicationError::Unauthenticated { message }
        | ApplicationError::PermissionDenied { message }
        | ApplicationError::Conflict { message }
        | ApplicationError::Failed { message }
        | ApplicationError::InvalidPath { message }
        | ApplicationError::SchemaMismatch { message }
        | ApplicationError::UnsupportedSnapshotPolicy { message }
        | ApplicationError::TransactionConflict { message } => json!({ "message": message }),
        ApplicationError::NotFound { resource } => json!({ "resource": resource }),
        ApplicationError::UnknownValue { value } | ApplicationError::StaleReference { value } => {
            json!({ "value": value })
        }
        ApplicationError::Cancelled
        | ApplicationError::Disconnected
        | ApplicationError::SubscriptionCapacity
        | ApplicationError::Closed => serde_json::Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_core::{
        Authority, CapabilityOwnerId, Key, ObservableRegistration, PhenixValue, PluginExecution,
        PluginId, PluginManifest, SdkContribution, SdkNamespace, SdkObservableResource,
        SdkResourceId, SnapshotPolicy, Type, ValueId, ValuePath,
    };

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

    #[tokio::test]
    async fn sdk_get_and_capability_invoke_share_the_live_production_registry() {
        let manifest = PluginManifest {
            id: PluginId::parse("testing").unwrap(),
            version: 1,
            execution: PluginExecution::ResourceOnly,
            dependencies: Vec::new(),
            services: Vec::new(),
            resource_namespaces: Vec::new(),
            maximum_authority: Authority::default(),
        };
        let value_id = ValueId::parse("testing.state@1").unwrap();
        let mut contribution = SdkContribution::new(
            manifest.id.clone(),
            SdkNamespace::parse("testing").unwrap(),
        );
        contribution.insert_observable(SdkObservableResource::new(
            SdkResourceId::parse("sdk/testing/state").unwrap(),
            ["state"],
            value_id.clone(),
            ValuePath::root(),
            Type::U64,
        ));
        let resolved = ResolvedSdkContributions::resolve(&[manifest], &[], [contribution]).unwrap();
        let store = ObservableStore::default();
        store
            .register(ObservableRegistration {
                id: value_id,
                owner: PluginId::parse("testing").unwrap(),
                schema: Type::U64,
                snapshot_policy: SnapshotPolicy::CopyOnChange,
                initial: PhenixValue::U64(7),
            })
            .unwrap();
        let capabilities = SharedCapabilityRegistry::default();
        let runtime = RuntimeId::parse("phenix.application-runtime").unwrap();
        let generation = CapabilityGenerationId::parse("application-generation-1").unwrap();
        let service = SdkApplicationService::new(
            &resolved,
            &store,
            capabilities,
            runtime.clone(),
            generation.clone(),
        )
        .unwrap();
        let (transport, receiver) = ChannelTransport::new(2);
        let worker = tokio::spawn(serve_sdk_application(service, receiver));

        let sdk = transport
            .invoke(
                &ContractId::parse(GetSdk::ID).unwrap(),
                Empty {}.to_value(),
            )
            .await
            .unwrap();
        let sdk = ApplicationSdkValue::from_value(&sdk).unwrap();
        let PhenixValue::Table(namespaces) = sdk.value else {
            panic!("SDK root is a table");
        };
        let PhenixValue::Table(resources) = namespaces.get("testing").unwrap() else {
            panic!("namespace is a table");
        };
        let PhenixValue::Table(state) = resources.get("state").unwrap() else {
            panic!("state resource is a table");
        };
        let PhenixValue::Callable(get) = state.get("get").unwrap() else {
            panic!("get is callable");
        };
        assert_eq!(get.owner(), &CapabilityOwnerId::Runtime(runtime));
        assert_eq!(get.generation(), &generation);

        let result = transport
            .invoke(
                &ContractId::parse(InvokeCapability::ID).unwrap(),
                ApplicationCapabilityInvokeInput {
                    callable: PhenixValue::Callable(get.clone()),
                    input: PhenixValue::Unit,
                }
                .to_value(),
            )
            .await
            .unwrap();
        let result = ApplicationCapabilityInvokeResult::from_value(&result).unwrap();
        assert_eq!(
            result.output,
            PhenixValue::Table(std::collections::BTreeMap::from([
                (Key::parse("value").unwrap(), PhenixValue::U64(7)),
                (Key::parse("version").unwrap(), PhenixValue::U64(0)),
            ]))
        );

        drop(transport);
        worker.await.unwrap();
    }

    #[test]
    fn application_error_bridge_preserves_structural_class_and_details() {
        let error = application_error_to_acp(ApplicationError::PermissionDenied {
            message: "same display text".to_owned(),
        });
        let data = error.data.expect("structured ACP error data");
        assert_eq!(data["phenix.class"], "permission_denied");
        assert_eq!(data["phenix.details"]["message"], "same display text");
    }

    #[test]
    fn cancellation_keeps_its_application_error_class() {
        let error = application_error_to_acp(ApplicationError::Cancelled);
        let data = error.data.expect("structured ACP error data");
        assert_eq!(data["phenix.class"], "cancelled");
        assert!(data["phenix.details"].is_null());
    }
}
