#![forbid(unsafe_code)]

//! ACP stdio transport for the fixed Phenix application interface.
//!
//! Protocol translation stays in `phenix-adapter-acp`. This crate owns only
//! process transport and the channel boundary used by the configured runtime.

use agent_client_protocol::{schema::v1::*, Agent, Error, ErrorCode, Stdio};
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
    CallableRef, CapabilityError, CapabilityGenerationId,
    CapabilityInvokeInput as CoreCapabilityInvokeInput, CapabilityOwnerId, ContractId,
    ObservableStore, PhenixValue, ResolvedSdkContributions, RuntimeId, SharedCapabilityRegistry,
    Type, ValueCodec,
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

/// One generic runtime-to-client capability invocation waiting for its ACP response.
pub struct ClientCapabilityInvocation {
    request: ApplicationCapabilityInvokeInput,
    response: oneshot::Sender<Result<ApplicationCapabilityInvokeResult, CapabilityError>>,
}

impl ClientCapabilityInvocation {
    #[must_use]
    pub fn request(&self) -> &ApplicationCapabilityInvokeInput {
        &self.request
    }

    pub fn respond(self, response: Result<ApplicationCapabilityInvokeResult, CapabilityError>) {
        let _ = self.response.send(response);
    }
}

/// Bounded bridge from synchronous runtime capability handlers to one ACP client.
#[derive(Clone)]
pub struct ClientCapabilityCallbacks {
    sender: mpsc::Sender<ClientCapabilityInvocation>,
}

impl ClientCapabilityCallbacks {
    #[must_use]
    pub fn bounded(capacity: usize) -> (Self, mpsc::Receiver<ClientCapabilityInvocation>) {
        let (sender, receiver) = mpsc::channel(capacity);
        (Self { sender }, receiver)
    }

    fn invoke(
        &self,
        callable: CallableRef,
        input: PhenixValue,
    ) -> Result<PhenixValue, CapabilityError> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .try_send(ClientCapabilityInvocation {
                request: ApplicationCapabilityInvokeInput {
                    callable: PhenixValue::Callable(callable),
                    input,
                },
                response,
            })
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => CapabilityError::QueueFull,
                mpsc::error::TrySendError::Closed(_) => CapabilityError::Disconnected,
            })?;
        receiver
            .blocking_recv()
            .map_err(|_| CapabilityError::Disconnected)?
            .map(|response| response.output)
    }
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
    client_callbacks: ClientCapabilityCallbacks,
}

impl SdkApplicationService {
    pub fn new(
        sdk: &ResolvedSdkContributions,
        store: &ObservableStore,
        capabilities: SharedCapabilityRegistry,
        runtime: RuntimeId,
        generation: CapabilityGenerationId,
        client_callbacks: ClientCapabilityCallbacks,
    ) -> Result<Self, phenix_core::SdkResolutionError> {
        let sdk = sdk.value_with_observables(store, &capabilities, &runtime, generation)?;
        Ok(Self {
            sdk: ApplicationSdkValue {
                schema: sdk.schema,
                value: sdk.value,
            },
            capabilities,
            client_callbacks,
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
            let request =
                ApplicationCapabilityInvokeInput::from_value(&input).map_err(|error| {
                    ApplicationError::InvalidInput {
                        message: error.to_string(),
                    }
                })?;
            let PhenixValue::Callable(callable) = request.callable else {
                return Err(ApplicationError::InvalidInput {
                    message: "capability invocation requires a callable reference".to_owned(),
                });
            };
            let schema = self.capabilities.schema(&callable)?;
            CoreCapabilityInvokeInput {
                callable: callable.clone(),
                input: request.input.clone(),
            }
            .validate(&schema)
            .map_err(|error| ApplicationError::SchemaMismatch {
                message: error.to_string(),
            })?;
            self.admit_client_callables(&schema, &request.input)?;
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

    fn admit_client_callables(
        &self,
        schema: &Type,
        value: &PhenixValue,
    ) -> Result<(), ApplicationError> {
        match (schema, value) {
            (Type::Callable { .. }, PhenixValue::Callable(callable)) => {
                if matches!(callable.owner(), CapabilityOwnerId::Client(_)) {
                    self.admit_client_callable(callable, schema.clone())?;
                }
            }
            (Type::Option(schema), PhenixValue::Option(Some(value))) => {
                self.admit_client_callables(schema, value)?;
            }
            (Type::Array { item, .. } | Type::List(item), PhenixValue::List(values)) => {
                for value in values {
                    self.admit_client_callables(item, value)?;
                }
            }
            (Type::Map(item), PhenixValue::Map(values)) => {
                for value in values.values() {
                    self.admit_client_callables(item, value)?;
                }
            }
            (Type::Table(fields), PhenixValue::Table(values)) => {
                for (key, schema) in fields {
                    let value =
                        values
                            .get(key)
                            .ok_or_else(|| ApplicationError::SchemaMismatch {
                                message: format!("capability input is missing field {key}"),
                            })?;
                    self.admit_client_callables(schema, value)?;
                }
            }
            (Type::Variant(variants), PhenixValue::Variant { tag, value }) => {
                let schema = variants
                    .get(tag)
                    .ok_or_else(|| ApplicationError::SchemaMismatch {
                        message: format!("capability input has unknown variant {tag}"),
                    })?;
                self.admit_client_callables(schema, value)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn admit_client_callable(
        &self,
        callable: &CallableRef,
        schema: Type,
    ) -> Result<(), ApplicationError> {
        let callbacks = self.client_callbacks.clone();
        let reference = callable.clone();
        let registry = self.capabilities.clone();
        let owner = callable.owner().clone();
        let generation = callable.generation().clone();
        match self
            .capabilities
            .register(reference.clone(), schema.clone(), move |input| {
                let result = callbacks.invoke(reference.clone(), input);
                if matches!(result, Err(CapabilityError::Disconnected)) {
                    registry.retire(owner.clone(), generation.clone());
                }
                result
            }) {
            Ok(()) => Ok(()),
            Err(CapabilityError::DuplicateReference(_)) => {
                let registered = self.capabilities.schema(callable)?;
                if registered == schema {
                    Ok(())
                } else {
                    Err(ApplicationError::SchemaMismatch {
                        message: format!(
                            "client callable {} was already admitted with another schema",
                            callable.id()
                        ),
                    })
                }
            }
            Err(error) => Err(error.into()),
        }
    }
}

pub async fn serve_sdk_application(
    service: SdkApplicationService,
    mut receiver: mpsc::Receiver<ApplicationInvocation>,
) {
    while let Some(invocation) = receiver.recv().await {
        let service = service.clone();
        tokio::task::spawn_blocking(move || service.handle(invocation));
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
    events: mpsc::Receiver<ApplicationEvent>,
) -> Result<(), Error> {
    let (keep_callbacks_open, callbacks) = ClientCapabilityCallbacks::bounded(1);
    let result =
        serve_stdio_with_events_and_callbacks(transport, advertised, events, callbacks).await;
    drop(keep_callbacks_open);
    result
}

pub async fn serve_stdio_with_events_and_callbacks(
    transport: ChannelTransport,
    advertised: impl IntoIterator<Item = ContractId>,
    mut events: mpsc::Receiver<ApplicationEvent>,
    mut callbacks: mpsc::Receiver<ClientCapabilityInvocation>,
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
    let callback_adapter = Arc::clone(&adapter);

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
                    callback = callbacks.recv() => match callback {
                        Some(callback) => {
                            let callable = match &callback.request().callable {
                                PhenixValue::Callable(callable) => callable.clone(),
                                _ => {
                                    callback.respond(Err(CapabilityError::SchemaMismatch {
                                        message: "client capability callback requires a callable reference".to_owned(),
                                    }));
                                    continue;
                                }
                            };
                            let (callback_id, request) = match callback_adapter
                                .extension_callback_request(callback.request())
                            {
                                Ok(request) => request,
                                Err(error) => {
                                    callback.respond(Err(application_error_to_capability(error, callable)));
                                    continue;
                                }
                            };
                            let response = match connection
                                .send_request(AgentRequest::ExtMethodRequest(request))
                                .block_task()
                                .await
                            {
                                Ok(response) => response,
                                Err(error) => {
                                    callback.respond(Err(acp_error_to_capability(error, callable)));
                                    continue;
                                }
                            };
                            let response = match serde_json::from_value(response) {
                                Ok(response) => response,
                                Err(error) => {
                                    callback.respond(Err(CapabilityError::SchemaMismatch {
                                        message: format!(
                                            "cannot decode ACP extension callback response: {error}"
                                        ),
                                    }));
                                    continue;
                                }
                            };
                            callback.respond(
                                callback_adapter
                                    .extension_callback_response(&callback_id, &response)
                                    .map_err(|error| application_error_to_capability(error, callable)),
                            );
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

fn application_error_to_capability(
    error: ApplicationError,
    callable: CallableRef,
) -> CapabilityError {
    match error {
        ApplicationError::Cancelled => CapabilityError::Cancelled,
        ApplicationError::Disconnected => CapabilityError::Disconnected,
        ApplicationError::StaleReference { .. } => CapabilityError::StaleReference(callable),
        ApplicationError::NotFound { .. } => CapabilityError::UnknownReference(callable),
        ApplicationError::SchemaMismatch { message }
        | ApplicationError::InvalidInput { message }
        | ApplicationError::InvalidResponse { message } => {
            CapabilityError::SchemaMismatch { message }
        }
        other => CapabilityError::ProviderFailed {
            message: other.to_string(),
        },
    }
}

fn acp_error_to_capability(error: Error, callable: CallableRef) -> CapabilityError {
    let class = error
        .data
        .as_ref()
        .and_then(|data| data.get("phenix.class"))
        .and_then(serde_json::Value::as_str);
    if error.code == ErrorCode::RequestCancelled || class == Some("cancelled") {
        return CapabilityError::Cancelled;
    }
    match class {
        Some("disconnected") => CapabilityError::Disconnected,
        Some("queue_full") => CapabilityError::QueueFull,
        Some("stale_reference") => CapabilityError::StaleReference(callable),
        Some("not_found") => CapabilityError::UnknownReference(callable),
        Some("schema_mismatch") | Some("invalid_input") | Some("invalid_response") => {
            CapabilityError::SchemaMismatch {
                message: error.to_string(),
            }
        }
        _ => CapabilityError::ProviderFailed {
            message: error.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_core::{
        Authority, CapabilityOwnerId, ClientConnectionId, Key, ObservableRegistration, PhenixValue,
        PluginExecution, PluginId, PluginManifest, ReferenceId, SdkContribution, SdkNamespace,
        SdkObservableResource, SdkResourceId, SnapshotPolicy, Type, ValueId, ValuePath,
    };

    fn client_callable() -> CallableRef {
        CallableRef::new(
            ContractId::parse("fixture.client-callback@1").unwrap(),
            CapabilityOwnerId::Client(ClientConnectionId::parse("fixture-client").unwrap()),
            CapabilityGenerationId::parse("fixture-generation").unwrap(),
            ReferenceId::parse("callback").unwrap(),
        )
    }

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
    async fn client_capability_bridge_forwards_the_canonical_invocation() {
        let (callbacks, mut receiver) = ClientCapabilityCallbacks::bounded(1);
        let callable = client_callable();
        let expected = callable.clone();
        let worker =
            tokio::task::spawn_blocking(move || callbacks.invoke(callable, PhenixValue::U64(7)));

        let invocation = receiver.recv().await.unwrap();
        assert_eq!(
            invocation.request().callable,
            PhenixValue::Callable(expected)
        );
        assert_eq!(invocation.request().input, PhenixValue::U64(7));
        invocation.respond(Ok(ApplicationCapabilityInvokeResult {
            output: PhenixValue::String("ok".to_owned()),
        }));
        assert_eq!(
            worker.await.unwrap().unwrap(),
            PhenixValue::String("ok".to_owned())
        );
    }

    #[tokio::test]
    async fn client_capability_bridge_preserves_structural_failures() {
        let cases = [
            CapabilityError::Cancelled,
            CapabilityError::Disconnected,
            CapabilityError::QueueFull,
        ];
        for expected in cases {
            let (callbacks, mut receiver) = ClientCapabilityCallbacks::bounded(1);
            let callable = client_callable();
            let worker = tokio::task::spawn_blocking(move || {
                callbacks.invoke(callable, PhenixValue::U64(7))
            });
            receiver.recv().await.unwrap().respond(Err(expected.clone()));
            assert_eq!(worker.await.unwrap().unwrap_err(), expected);
        }
    }

    #[tokio::test]
    async fn generic_callable_input_admits_a_client_owned_reference() {
        let capabilities = SharedCapabilityRegistry::default();
        let (callbacks, mut receiver) = ClientCapabilityCallbacks::bounded(1);
        let service = SdkApplicationService {
            sdk: ApplicationSdkValue {
                schema: Type::Table(Default::default()),
                value: PhenixValue::Table(Default::default()),
            },
            capabilities: capabilities.clone(),
            client_callbacks: callbacks,
        };
        let callable = client_callable();
        let schema = Type::Callable {
            contract: callable.contract().clone(),
            input: Box::new(Type::U64),
            output: Box::new(Type::String),
        };
        service
            .admit_client_callables(&schema, &PhenixValue::Callable(callable.clone()))
            .unwrap();

        let worker = tokio::task::spawn_blocking(move || {
            capabilities.invoke(CoreCapabilityInvokeInput {
                callable,
                input: PhenixValue::U64(7),
            })
        });
        let invocation = receiver.recv().await.unwrap();
        invocation.respond(Ok(ApplicationCapabilityInvokeResult {
            output: PhenixValue::String("ok".to_owned()),
        }));
        assert_eq!(
            worker.await.unwrap().unwrap().output,
            PhenixValue::String("ok".to_owned())
        );
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
        let mut contribution =
            SdkContribution::new(manifest.id.clone(), SdkNamespace::parse("testing").unwrap());
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
        let (client_callbacks, _callback_receiver) = ClientCapabilityCallbacks::bounded(1);
        let runtime = RuntimeId::parse("phenix.application-runtime").unwrap();
        let generation = CapabilityGenerationId::parse("application-generation-1").unwrap();
        let service = SdkApplicationService::new(
            &resolved,
            &store,
            capabilities,
            runtime.clone(),
            generation.clone(),
            client_callbacks,
        )
        .unwrap();
        let (transport, receiver) = ChannelTransport::new(2);
        let worker = tokio::spawn(serve_sdk_application(service, receiver));

        let sdk = transport
            .invoke(&ContractId::parse(GetSdk::ID).unwrap(), Empty {}.to_value())
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

    #[test]
    fn callback_request_errors_keep_capability_classes() {
        let callable = client_callable();
        let cancelled = Error::request_cancelled().data(json!({
            "phenix.class": "cancelled",
            "phenix.details": null,
        }));
        assert_eq!(
            acp_error_to_capability(cancelled, callable.clone()),
            CapabilityError::Cancelled
        );

        let full = Error::internal_error().data(json!({
            "phenix.class": "queue_full",
            "phenix.details": null,
        }));
        assert_eq!(
            acp_error_to_capability(full, callable.clone()),
            CapabilityError::QueueFull
        );

        let disconnected = Error::internal_error().data(json!({
            "phenix.class": "disconnected",
            "phenix.details": null,
        }));
        assert_eq!(
            acp_error_to_capability(disconnected, callable),
            CapabilityError::Disconnected
        );
    }
}
