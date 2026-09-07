#![forbid(unsafe_code)]

use agent_client_protocol::schema::{
    v1::{
        CancelNotification, CloseSessionRequest, CloseSessionResponse, InitializeRequest,
        ListSessionsRequest, ListSessionsResponse, LoadSessionRequest, LoadSessionResponse,
        NewSessionRequest, NewSessionResponse, PromptRequest, PromptResponse, ResumeSessionRequest,
        ResumeSessionResponse, SetSessionConfigOptionRequest, SetSessionConfigOptionResponse,
    },
    ProtocolVersion,
};
use agent_client_protocol::{AcpAgent, AcpAgentConfig, Agent, ConnectionTo};
use phenix_application_interface::{
    ApplicationClient, ApplicationTransport, Capabilities, Operation,
};
use phenix_core::{ContractId, PhenixSchema};
use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    path::PathBuf,
};

pub use phenix_application_interface::{
    application_descriptor, types::ApplicationError, INTERFACE_ID,
};

pub mod generated {
    include!(concat!(env!("OUT_DIR"), "/application.rs"));
}

pub struct ApplicationApi<T> {
    application: ApplicationClient<T>,
}

impl<T: ApplicationTransport> ApplicationApi<T> {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StdioConfig {
    command: PathBuf,
    args: Vec<String>,
    env: BTreeMap<String, String>,
}

impl StdioConfig {
    #[must_use]
    pub fn new(command: impl Into<PathBuf>) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
            env: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    #[must_use]
    pub fn env(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(name.into(), value.into());
        self
    }

    fn agent(&self) -> AcpAgent {
        AcpAgent::new(
            AcpAgentConfig::new(self.command.clone())
                .args(self.args.clone())
                .envs(self.env.clone()),
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClientError {
    Transport(String),
    Protocol(String),
    UnsupportedCapability {
        operation: ContractId,
        capability: ContractId,
    },
    OutOfOrderUpdate {
        session_id: String,
        expected: u64,
        received: u64,
    },
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(message) => write!(formatter, "ACP transport failure: {message}"),
            Self::Protocol(message) => write!(formatter, "ACP protocol failure: {message}"),
            Self::UnsupportedCapability {
                operation,
                capability,
            } => write!(
                formatter,
                "ACP peer does not support operation {operation}; capability {capability} is unavailable"
            ),
            Self::OutOfOrderUpdate {
                session_id,
                expected,
                received,
            } => write!(
                formatter,
                "ACP session {session_id} update sequence is {received}; expected {expected}"
            ),
        }
    }
}

impl std::error::Error for ClientError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtensionMethod {
    pub operation: ContractId,
    pub capability: ContractId,
    pub method: String,
    pub input: PhenixSchema,
    pub output: PhenixSchema,
    pub error: PhenixSchema,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DescriptorExtensions {
    interface: ContractId,
    methods: BTreeMap<ContractId, ExtensionMethod>,
    advertised_capabilities: BTreeSet<ContractId>,
}

impl DescriptorExtensions {
    #[must_use]
    pub fn from_descriptor(capabilities: impl IntoIterator<Item = ContractId>) -> Self {
        let descriptor = application_descriptor();
        let advertised_capabilities = capabilities.into_iter().collect::<BTreeSet<_>>();
        let methods = descriptor
            .operations
            .iter()
            .filter(|(_, operation)| advertised_capabilities.contains(&operation.capability))
            .map(|(operation, definition)| {
                (
                    operation.clone(),
                    ExtensionMethod {
                        operation: operation.clone(),
                        capability: definition.capability.clone(),
                        method: extension_name(operation),
                        input: schema(&descriptor, &definition.input),
                        output: schema(&descriptor, &definition.output),
                        error: schema(&descriptor, &definition.error),
                    },
                )
            })
            .collect();

        Self {
            interface: descriptor.id,
            methods,
            advertised_capabilities,
        }
    }

    #[must_use]
    pub fn interface(&self) -> &ContractId {
        &self.interface
    }

    #[must_use]
    pub fn method(&self, operation: &ContractId) -> Option<&ExtensionMethod> {
        self.methods.get(operation)
    }

    pub fn require(&self, operation: &ContractId) -> Result<&ExtensionMethod, ClientError> {
        self.method(operation).ok_or_else(|| {
            let descriptor = application_descriptor();
            let capability = descriptor
                .operations
                .get(operation)
                .map(|definition| definition.capability.clone())
                .unwrap_or_else(|| operation.clone());
            ClientError::UnsupportedCapability {
                operation: operation.clone(),
                capability,
            }
        })
    }

    #[must_use]
    pub fn supports(&self, capability: &ContractId) -> bool {
        self.advertised_capabilities.contains(capability)
    }
}

fn extension_name(operation: &ContractId) -> String {
    let suffix = operation
        .as_str()
        .strip_prefix("phenix.application.")
        .unwrap_or(operation.as_str());
    format!("_phenix/{suffix}")
}

fn schema(
    descriptor: &phenix_application_interface::ApplicationDescriptor,
    id: &ContractId,
) -> PhenixSchema {
    descriptor
        .types
        .get(id)
        .unwrap_or_else(|| panic!("application descriptor is missing schema {id}"))
        .clone()
}

#[derive(Debug, Default)]
pub struct OrderedUpdates {
    next_sequence: BTreeMap<String, u64>,
}

impl OrderedUpdates {
    pub fn accept(
        &mut self,
        session_id: impl Into<String>,
        sequence: u64,
    ) -> Result<(), ClientError> {
        let session_id = session_id.into();
        let expected = self.next_sequence.entry(session_id.clone()).or_insert(0);
        if sequence != *expected {
            return Err(ClientError::OutOfOrderUpdate {
                session_id,
                expected: *expected,
                received: sequence,
            });
        }
        *expected = expected.saturating_add(1);
        Ok(())
    }

    pub fn resume_at(&mut self, session_id: impl Into<String>, next_sequence: u64) {
        self.next_sequence.insert(session_id.into(), next_sequence);
    }
}

#[derive(Clone)]
pub struct AcpClient {
    config: StdioConfig,
}

impl AcpClient {
    #[must_use]
    pub fn new(config: StdioConfig) -> Self {
        Self { config }
    }

    #[must_use]
    pub fn config(&self) -> &StdioConfig {
        &self.config
    }

    pub async fn connect_with<F, Fut, R>(&self, use_connection: F) -> Result<R, ClientError>
    where
        F: FnOnce(AcpConnection) -> Fut,
        Fut: Future<Output = Result<R, ClientError>>,
    {
        agent_client_protocol::Client
            .builder()
            .connect_with(
                self.config.agent(),
                move |connection: ConnectionTo<Agent>| async move {
                    let initialized = connection
                        .send_request(InitializeRequest::new(ProtocolVersion::V1))
                        .block_task()
                        .await?;
                    let extensions = descriptor_extensions(&initialized)?;
                    use_connection(AcpConnection {
                        connection,
                        extensions,
                    })
                    .await
                    .map_err(|error| {
                        agent_client_protocol::Error::internal_error().data(error.to_string())
                    })
                },
            )
            .await
            .map_err(|error| ClientError::Transport(error.to_string()))
    }

    pub async fn reconnect_with<F, Fut, R>(&self, use_connection: F) -> Result<R, ClientError>
    where
        F: FnOnce(AcpConnection) -> Fut,
        Fut: Future<Output = Result<R, ClientError>>,
    {
        self.connect_with(use_connection).await
    }
}

fn descriptor_extensions(
    initialized: &agent_client_protocol::schema::v1::InitializeResponse,
) -> Result<DescriptorExtensions, agent_client_protocol::Error> {
    let value = serde_json::to_value(initialized)
        .map_err(agent_client_protocol::Error::into_internal_error)?;
    let extension = value
        .get("meta")
        .and_then(|meta| meta.get("phenix.extensions"))
        .ok_or_else(|| {
            agent_client_protocol::Error::invalid_params()
                .data("ACP peer did not advertise phenix.extensions metadata")
        })?;
    let interface = extension
        .get("interface")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            agent_client_protocol::Error::invalid_params()
                .data("ACP peer omitted its Phenix application interface id")
        })?;
    if interface != INTERFACE_ID {
        return Err(agent_client_protocol::Error::invalid_params().data(format!(
            "ACP peer advertises application interface {interface}; expected {INTERFACE_ID}"
        )));
    }
    let capabilities = extension
        .get("methods")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|method| method.get("capability"))
        .filter_map(serde_json::Value::as_str)
        .map(ContractId::parse)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            agent_client_protocol::Error::invalid_params().data(format!(
                "ACP peer advertised an invalid Phenix capability: {error}"
            ))
        })?;
    Ok(DescriptorExtensions::from_descriptor(capabilities))
}

pub struct AcpConnection {
    connection: ConnectionTo<Agent>,
    extensions: DescriptorExtensions,
}

impl AcpConnection {
    #[must_use]
    pub fn extensions(&self) -> &DescriptorExtensions {
        &self.extensions
    }

    pub async fn new_session(
        &self,
        request: NewSessionRequest,
    ) -> Result<NewSessionResponse, ClientError> {
        self.connection
            .send_request(request)
            .block_task()
            .await
            .map_err(|error| ClientError::Transport(error.to_string()))
    }

    pub async fn list_sessions(
        &self,
        request: ListSessionsRequest,
    ) -> Result<ListSessionsResponse, ClientError> {
        self.connection
            .send_request(request)
            .block_task()
            .await
            .map_err(|error| ClientError::Transport(error.to_string()))
    }

    pub async fn resume_session(
        &self,
        request: ResumeSessionRequest,
    ) -> Result<ResumeSessionResponse, ClientError> {
        self.connection
            .send_request(request)
            .block_task()
            .await
            .map_err(|error| ClientError::Transport(error.to_string()))
    }

    pub async fn load_session(
        &self,
        request: LoadSessionRequest,
    ) -> Result<LoadSessionResponse, ClientError> {
        self.connection
            .send_request(request)
            .block_task()
            .await
            .map_err(|error| ClientError::Transport(error.to_string()))
    }

    pub async fn close_session(
        &self,
        request: CloseSessionRequest,
    ) -> Result<CloseSessionResponse, ClientError> {
        self.connection
            .send_request(request)
            .block_task()
            .await
            .map_err(|error| ClientError::Transport(error.to_string()))
    }

    pub async fn prompt(&self, request: PromptRequest) -> Result<PromptResponse, ClientError> {
        self.connection
            .send_request(request)
            .block_task()
            .await
            .map_err(|error| ClientError::Transport(error.to_string()))
    }

    pub async fn set_session_config_option(
        &self,
        request: SetSessionConfigOptionRequest,
    ) -> Result<SetSessionConfigOptionResponse, ClientError> {
        self.connection
            .send_request(request)
            .block_task()
            .await
            .map_err(|error| ClientError::Transport(error.to_string()))
    }

    pub fn cancel(&self, notification: CancelNotification) {
        let _ = self.connection.send_notification(notification);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_core::{PhenixContract, PhenixValue};

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
        ) -> impl Future<Output = Result<PhenixValue, ApplicationError>> {
            std::future::ready(Err(self.0.clone()))
        }
    }

    fn client(error: ApplicationError) -> ApplicationApi<RejectingTransport> {
        let capability =
            ContractId::parse(FixtureOperation::CAPABILITY).expect("static capability id is valid");
        let capabilities = Capabilities::negotiate(&application_descriptor(), [capability])
            .expect("discovery has no missing dependency");
        ApplicationApi::new(RejectingTransport(error), capabilities)
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

    #[test]
    fn descriptor_extensions_use_the_fixed_operation_and_schema_ids() {
        let capability = ContractId::parse("phenix.application.capability.sessions@1")
            .expect("static capability id is valid");
        let extensions = DescriptorExtensions::from_descriptor([capability]);
        let operation = ContractId::parse("phenix.application.session.create@1")
            .expect("static operation id is valid");
        let method = extensions
            .require(&operation)
            .expect("sessions capability supports creation");
        assert_eq!(method.method, "_phenix/session.create@1");
        assert_eq!(method.operation, operation);
    }

    #[test]
    fn unavailable_descriptor_operations_fail_before_the_request_is_sent() {
        let extensions = DescriptorExtensions::from_descriptor([]);
        let operation = ContractId::parse("phenix.application.session.create@1")
            .expect("static operation id is valid");
        assert!(matches!(
            extensions.require(&operation),
            Err(ClientError::UnsupportedCapability { .. })
        ));
    }

    #[test]
    fn ordered_updates_reject_gaps_and_allow_resume() {
        let mut updates = OrderedUpdates::default();
        updates
            .accept("session-1", 0)
            .expect("first update is ordered");
        assert!(matches!(
            updates.accept("session-1", 2),
            Err(ClientError::OutOfOrderUpdate {
                expected: 1,
                received: 2,
                ..
            })
        ));
        updates.resume_at("session-1", 7);
        updates
            .accept("session-1", 7)
            .expect("resumed update is ordered");
    }
}
