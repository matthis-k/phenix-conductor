use crate::{CompiledConfiguration, ConductorRuntime, SqliteStore};
use phenix_backend::Backend;
use phenix_core::{
    BackendCatalog, BackendId, ExecutionEventKind, ExecutionId, ExecutionState,
    WorkspaceDescriptor, WorkspaceId,
};
use phenix_protocol::{
    ClientEnvelope, ErrorCode, FrontendConnectionCommand, FrontendConnectionEvent,
    FrontendServiceNotification, FrontendServiceProviderDescriptor, FrontendServiceRequest,
    ProtocolError, Reply, ResponsePayload, ServerMessage,
};
use serde_json::Value;
use std::io::{self, BufRead, BufReader, Cursor, Read, Write};
use std::sync::{mpsc, MutexGuard};
use std::thread;

#[allow(dead_code, clippy::too_many_arguments)]
#[path = "server_base.rs"]
mod base;
#[path = "frontend_services.rs"]
mod frontend_services;

pub use base::ServerError;
use frontend_services::{
    FrontendConnectionId, FrontendServiceInboundNotification, FrontendServiceRouter,
    FrontendServiceRouterError,
};

const FRONTEND_OUTPUT_BUFFER: usize = 256;

pub struct ConductorServer {
    inner: base::ConductorServer,
    frontend_services: FrontendServiceRouter,
}

impl ConductorServer {
    #[must_use]
    pub fn new(runtime: ConductorRuntime) -> Self {
        Self {
            inner: base::ConductorServer::new(runtime),
            frontend_services: FrontendServiceRouter::default(),
        }
    }

    pub fn load_or_new(store: SqliteStore, workspace_id: WorkspaceId) -> Result<Self, ServerError> {
        Ok(Self {
            inner: base::ConductorServer::load_or_new(store, workspace_id)?,
            frontend_services: FrontendServiceRouter::default(),
        })
    }

    pub fn install_workspace_consistency(
        &mut self,
        descriptor: WorkspaceDescriptor,
    ) -> Result<(), ServerError> {
        self.inner.install_workspace_consistency(descriptor)
    }

    pub fn install_workspace_tools_into(
        &self,
        configuration: &mut CompiledConfiguration,
    ) -> Result<(), ServerError> {
        self.inner.install_workspace_tools_into(configuration)
    }

    pub fn install_workspace_tools(&mut self) -> Result<(), ServerError> {
        self.inner.install_workspace_tools()
    }

    pub fn register_backend(
        &mut self,
        backend_id: BackendId,
        backend: Box<dyn Backend>,
    ) -> Result<(), ServerError> {
        self.inner.register_backend(backend_id, backend)
    }

    pub fn runtime(&self) -> MutexGuard<'_, ConductorRuntime> {
        self.inner.runtime()
    }

    #[must_use]
    pub fn catalogs(&self) -> Vec<BackendCatalog> {
        self.inner.catalogs()
    }

    pub fn request_frontend_service(
        &self,
        execution_id: &ExecutionId,
        request: FrontendServiceRequest,
    ) -> Result<Value, ProtocolError> {
        let root = self.inner.execution_group_id_for(execution_id)?;
        self.frontend_services
            .request(&root, request)
            .map_err(map_frontend_service_error)
    }

    pub fn notify_frontend_service(
        &self,
        execution_id: &ExecutionId,
        notification: FrontendServiceNotification,
    ) -> Result<(), ProtocolError> {
        let root = self.inner.execution_group_id_for(execution_id)?;
        self.frontend_services
            .notify(&root, notification)
            .map_err(map_frontend_service_error)
    }

    #[allow(dead_code, private_interfaces)]
    pub(crate) fn frontend_service_providers(
        &self,
    ) -> Result<Vec<(FrontendConnectionId, FrontendServiceProviderDescriptor)>, ProtocolError> {
        self.frontend_services
            .live_providers()
            .map_err(map_frontend_service_error)
    }

    #[allow(dead_code, private_interfaces)]
    pub(crate) fn request_frontend_service_on(
        &self,
        connection: FrontendConnectionId,
        request: FrontendServiceRequest,
    ) -> Result<Value, ProtocolError> {
        self.frontend_services
            .request_connection(connection, request)
            .map_err(map_frontend_service_error)
    }

    #[allow(dead_code, private_interfaces)]
    pub(crate) fn subscribe_frontend_service_notifications(
        &self,
    ) -> Result<mpsc::Receiver<FrontendServiceInboundNotification>, ProtocolError> {
        self.frontend_services
            .subscribe_notifications()
            .map_err(map_frontend_service_error)
    }

    pub fn serve_ndjson<R, W>(&mut self, input: R, output: W) -> Result<(), ServerError>
    where
        R: BufRead,
        W: Write + Send,
    {
        let router = self.frontend_services.clone();
        let hook_router = router.clone();
        serve_frontend_transport(input, output, router, false, |input, output, connection| {
            let mut on_root = |root: &ExecutionId| {
                hook_router
                    .bind_execution(connection, root.clone())
                    .map_err(map_frontend_router_server_error)
            };
            self.inner
                .serve_ndjson_with_root_hook(input, output, &mut on_root)
        })
    }
}

#[derive(Clone)]
pub struct ConductorService {
    inner: base::ConductorService,
    frontend_services: FrontendServiceRouter,
}

impl ConductorService {
    pub fn new(server: ConductorServer) -> Result<Self, ServerError> {
        Ok(Self {
            inner: base::ConductorService::new(server.inner)?,
            frontend_services: server.frontend_services,
        })
    }

    pub fn serve_connection<R, W>(&self, input: R, output: W) -> Result<(), ServerError>
    where
        R: BufRead,
        W: Write + Send,
    {
        let router = self.frontend_services.clone();
        let hook_router = router.clone();
        serve_frontend_transport(input, output, router, true, |input, output, connection| {
            let mut on_root = |root: &ExecutionId| {
                hook_router
                    .bind_execution(connection, root.clone())
                    .map_err(map_frontend_router_server_error)
            };
            self.inner
                .serve_connection_with_root_hook(input, output, &mut on_root)
        })
    }

    pub fn request_frontend_service(
        &self,
        execution_id: &ExecutionId,
        request: FrontendServiceRequest,
    ) -> Result<Value, ProtocolError> {
        let root = self.inner.execution_group_id_for(execution_id)?;
        self.frontend_services
            .request(&root, request)
            .map_err(map_frontend_service_error)
    }

    pub fn notify_frontend_service(
        &self,
        execution_id: &ExecutionId,
        notification: FrontendServiceNotification,
    ) -> Result<(), ProtocolError> {
        let root = self.inner.execution_group_id_for(execution_id)?;
        self.frontend_services
            .notify(&root, notification)
            .map_err(map_frontend_service_error)
    }

    #[allow(dead_code, private_interfaces)]
    pub(crate) fn frontend_service_providers(
        &self,
    ) -> Result<Vec<(FrontendConnectionId, FrontendServiceProviderDescriptor)>, ProtocolError> {
        self.frontend_services
            .live_providers()
            .map_err(map_frontend_service_error)
    }

    #[allow(dead_code, private_interfaces)]
    pub(crate) fn request_frontend_service_on(
        &self,
        connection: FrontendConnectionId,
        request: FrontendServiceRequest,
    ) -> Result<Value, ProtocolError> {
        self.frontend_services
            .request_connection(connection, request)
            .map_err(map_frontend_service_error)
    }

    #[allow(dead_code, private_interfaces)]
    pub(crate) fn subscribe_frontend_service_notifications(
        &self,
    ) -> Result<mpsc::Receiver<FrontendServiceInboundNotification>, ProtocolError> {
        self.frontend_services
            .subscribe_notifications()
            .map_err(map_frontend_service_error)
    }
}

fn serve_frontend_transport<R, W, F>(
    input: R,
    output: W,
    router: FrontendServiceRouter,
    normalize_disconnect: bool,
    serve: F,
) -> Result<(), ServerError>
where
    R: BufRead,
    W: Write + Send,
    F: FnOnce(
        BufReader<FrontendCommandReader<R>>,
        ObservingLineWriter,
        FrontendConnectionId,
    ) -> Result<(), ServerError>,
{
    let (line_sender, line_receiver) = mpsc::channel::<Vec<u8>>();
    let (service_sender, service_receiver) = mpsc::sync_channel(FRONTEND_OUTPUT_BUFFER);
    let connection = router
        .open_connection(service_sender)
        .map_err(|_| ServerError::StatePoisoned("frontend service router"))?;
    let connection_id = connection.id();

    thread::scope(|scope| {
        let writer = scope.spawn(move || -> Result<(), ServerError> {
            let mut output = output;
            while let Ok(line) = line_receiver.recv() {
                output.write_all(&line)?;
                output.flush()?;
            }
            Ok(())
        });

        let service_output = line_sender.clone();
        let relay = scope.spawn(move || -> Result<(), ServerError> {
            while let Ok(message) = service_receiver.recv() {
                send_server_message(&service_output, &message)?;
            }
            Ok(())
        });

        let filtered =
            FrontendCommandReader::new(input, router.clone(), connection_id, line_sender.clone());
        let observed = ObservingLineWriter::new(line_sender.clone(), router.clone());
        let result = serve(BufReader::new(filtered), observed, connection_id);

        drop(connection);
        drop(line_sender);
        let relay_result = relay.join().map_err(|_| ServerError::WorkerPanicked)?;
        let writer_result = writer.join().map_err(|_| ServerError::WorkerPanicked)?;

        let combined = result.and(relay_result).and(writer_result);
        if normalize_disconnect {
            normal_frontend_disconnect(combined)
        } else {
            combined
        }
    })
}

struct FrontendCommandReader<R> {
    input: R,
    current: Cursor<Vec<u8>>,
    router: FrontendServiceRouter,
    connection: FrontendConnectionId,
    output: mpsc::Sender<Vec<u8>>,
}

impl<R> FrontendCommandReader<R> {
    fn new(
        input: R,
        router: FrontendServiceRouter,
        connection: FrontendConnectionId,
        output: mpsc::Sender<Vec<u8>>,
    ) -> Self {
        Self {
            input,
            current: Cursor::new(Vec::new()),
            router,
            connection,
            output,
        }
    }
}

impl<R: BufRead> FrontendCommandReader<R> {
    fn refill(&mut self) -> io::Result<bool> {
        loop {
            let mut line = String::new();
            if self.input.read_line(&mut line)? == 0 {
                return Ok(false);
            }
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<ClientEnvelope>(&line) {
                Ok(ClientEnvelope::Command(message)) => {
                    let mut encoded = serde_json::to_vec(&message).map_err(io::Error::other)?;
                    encoded.push(b'\n');
                    self.current = Cursor::new(encoded);
                    return Ok(true);
                }
                Ok(ClientEnvelope::ConnectionCommand(command)) => {
                    self.handle_connection_command(command)?;
                }
                Ok(ClientEnvelope::ConnectionEvent(event)) => {
                    self.handle_connection_event(event)?;
                }
                Ok(ClientEnvelope::FrontendServiceResponse(response)) => {
                    let response_id = response.id;
                    if let Err(error) = self.router.complete_response(self.connection, response) {
                        self.send_protocol_response(
                            response_id,
                            Err(map_frontend_service_error(error)),
                        )?;
                    }
                }
                Err(_) => {
                    self.current = Cursor::new(line.into_bytes());
                    return Ok(true);
                }
            }
        }
    }

    fn handle_connection_command(&self, command: FrontendConnectionCommand) -> io::Result<()> {
        match command {
            FrontendConnectionCommand::SetFrontendServiceProviders { id, providers } => {
                let result = self
                    .router
                    .replace_providers(self.connection, providers)
                    .map(|()| Reply::Accepted)
                    .map_err(map_frontend_service_error);
                self.send_protocol_response(id, result)
            }
        }
    }

    fn handle_connection_event(&self, event: FrontendConnectionEvent) -> io::Result<()> {
        match event {
            FrontendConnectionEvent::FrontendServiceNotification { notification } => self
                .router
                .accept_notification(self.connection, notification)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string())),
        }
    }

    fn send_protocol_response(
        &self,
        id: u64,
        result: Result<Reply, ProtocolError>,
    ) -> io::Result<()> {
        let response = match result {
            Ok(result) => ResponsePayload::Ok { result },
            Err(error) => ResponsePayload::Error { error },
        };
        send_server_message(&self.output, &ServerMessage::Response { id, response })
            .map_err(|error| io::Error::other(error.to_string()))
    }
}

impl<R: BufRead> Read for FrontendCommandReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        loop {
            let read = self.current.read(buffer)?;
            if read > 0 {
                return Ok(read);
            }
            if !self.refill()? {
                return Ok(0);
            }
        }
    }
}

struct ObservingLineWriter {
    output: mpsc::Sender<Vec<u8>>,
    buffer: Vec<u8>,
    router: FrontendServiceRouter,
}

impl ObservingLineWriter {
    fn new(output: mpsc::Sender<Vec<u8>>, router: FrontendServiceRouter) -> Self {
        Self {
            output,
            buffer: Vec::new(),
            router,
        }
    }

    fn observe(&self, bytes: &[u8]) {
        let Ok(ServerMessage::Event { event }) = serde_json::from_slice::<ServerMessage>(bytes)
        else {
            return;
        };
        if let ExecutionEventKind::ExecutionStateChanged { state } = event.kind {
            if terminal_state(&state) {
                let _ = self.router.release_execution(&event.execution_id);
            }
        }
    }

    fn emit_complete_lines(&mut self) -> io::Result<()> {
        while let Some(end) = self.buffer.iter().position(|byte| *byte == b'\n') {
            let line = self.buffer.drain(..=end).collect::<Vec<_>>();
            self.observe(&line);
            self.output
                .send(line)
                .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "frontend output closed"))?;
        }
        Ok(())
    }
}

impl Write for ObservingLineWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.buffer.extend_from_slice(bytes);
        self.emit_complete_lines()?;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.emit_complete_lines()
    }
}

fn send_server_message(
    output: &mpsc::Sender<Vec<u8>>,
    message: &ServerMessage,
) -> Result<(), ServerError> {
    let mut bytes = serde_json::to_vec(message)?;
    bytes.push(b'\n');
    output.send(bytes).map_err(|_| ServerError::OutputClosed)
}

fn map_frontend_router_server_error(_error: FrontendServiceRouterError) -> ServerError {
    ServerError::StatePoisoned("frontend service router")
}

fn map_frontend_service_error(error: FrontendServiceRouterError) -> ProtocolError {
    let code = match error {
        FrontendServiceRouterError::NoFrontendForExecution(_)
        | FrontendServiceRouterError::ProviderUnavailable(_) => ErrorCode::UnsupportedCapability,
        FrontendServiceRouterError::Disconnected | FrontendServiceRouterError::OutputClosed => {
            ErrorCode::BackendTransport
        }
        FrontendServiceRouterError::Remote(_) => ErrorCode::ToolFailure,
        FrontendServiceRouterError::StatePoisoned
        | FrontendServiceRouterError::UnknownConnection
        | FrontendServiceRouterError::DuplicateProvider(_)
        | FrontendServiceRouterError::ExecutionAlreadyOwned(_)
        | FrontendServiceRouterError::UnknownRequest(_)
        | FrontendServiceRouterError::WrongConnection(_) => ErrorCode::InvalidRequest,
    };
    ProtocolError {
        code,
        message: error.to_string(),
        session_id: None,
        execution_id: match &error {
            FrontendServiceRouterError::NoFrontendForExecution(id)
            | FrontendServiceRouterError::ExecutionAlreadyOwned(id) => Some(id.clone()),
            _ => None,
        },
    }
}

fn terminal_state(state: &ExecutionState) -> bool {
    matches!(
        state,
        ExecutionState::Completed
            | ExecutionState::Failed
            | ExecutionState::Cancelled
            | ExecutionState::Interrupted
    )
}

fn normal_frontend_disconnect(result: Result<(), ServerError>) -> Result<(), ServerError> {
    match result {
        Err(ServerError::Io(error)) if is_disconnect_kind(error.kind()) => Ok(()),
        Err(ServerError::Json(error)) if error.io_error_kind().is_some_and(is_disconnect_kind) => {
            Ok(())
        }
        Err(ServerError::OutputClosed) => Ok(()),
        result => result,
    }
}

fn is_disconnect_kind(kind: io::ErrorKind) -> bool {
    matches!(
        kind,
        io::ErrorKind::BrokenPipe
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::ConnectionReset
            | io::ErrorKind::NotConnected
            | io::ErrorKind::UnexpectedEof
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_core::{
        AgentDefinition, CallableDescriptor, CallableId, CallableKind, CallablePolicy,
        CapabilitySet, ExecutionAuthority, ExecutionTarget, InferenceOptions, ModelId, ModelTarget,
        OrchestrationDefinition, OrchestrationNode, OrchestrationNodeId, ProviderId,
    };
    use phenix_protocol::{FrontendServiceProviderDescriptor, FrontendServiceProviderId};
    use std::collections::{BTreeMap, BTreeSet};

    fn provider() -> FrontendServiceProviderDescriptor {
        FrontendServiceProviderDescriptor {
            id: FrontendServiceProviderId::parse("web").unwrap(),
            capabilities: BTreeSet::from(["search".to_owned()]),
        }
    }

    fn model_target() -> ModelTarget {
        ModelTarget {
            backend: BackendId::parse("fixture").unwrap(),
            provider: ProviderId::parse("fixture").unwrap(),
            model: ModelId::parse("fixture-model").unwrap(),
            inference: InferenceOptions::default(),
        }
    }

    fn descriptor(id: &str, kind: CallableKind) -> CallableDescriptor {
        CallableDescriptor {
            id: CallableId::parse(id).unwrap(),
            kind,
            description: "frontend service fixture".to_owned(),
            input_schema: serde_json::json!({"type": "object"}),
            output_schema: serde_json::json!({"type": "object"}),
            capabilities: CapabilitySet::default(),
            policy: CallablePolicy::default(),
        }
    }

    #[test]
    fn connection_registration_is_acknowledged_without_entering_conductor_commands() {
        let mut server = ConductorServer::new(ConductorRuntime::new());
        let input = serde_json::to_vec(&ClientEnvelope::ConnectionCommand(
            FrontendConnectionCommand::SetFrontendServiceProviders {
                id: 7,
                providers: vec![provider()],
            },
        ))
        .unwrap();
        let mut input = input;
        input.push(b'\n');
        let mut output = Vec::new();
        server
            .serve_ndjson(std::io::Cursor::new(input), &mut output)
            .unwrap();
        let message: ServerMessage = serde_json::from_slice(&output).unwrap();
        assert!(matches!(
            message,
            ServerMessage::Response {
                id: 7,
                response: ResponsePayload::Ok {
                    result: Reply::Accepted
                }
            }
        ));
    }

    #[test]
    fn inbound_frontend_notification_reaches_conductor_subscriber() {
        let mut server = ConductorServer::new(ConductorRuntime::new());
        let notifications = server.subscribe_frontend_service_notifications().unwrap();
        let register = ClientEnvelope::ConnectionCommand(
            FrontendConnectionCommand::SetFrontendServiceProviders {
                id: 7,
                providers: vec![provider()],
            },
        );
        let notify =
            ClientEnvelope::ConnectionEvent(FrontendConnectionEvent::FrontendServiceNotification {
                notification: FrontendServiceNotification {
                    provider: FrontendServiceProviderId::parse("web").unwrap(),
                    method: "changed".to_owned(),
                    params: serde_json::json!({"document": "src/lib.rs"}),
                },
            });
        let mut input = serde_json::to_vec(&register).unwrap();
        input.push(b'\n');
        input.extend(serde_json::to_vec(&notify).unwrap());
        input.push(b'\n');
        let mut output = Vec::new();
        server
            .serve_ndjson(std::io::Cursor::new(input), &mut output)
            .unwrap();
        let received = notifications.recv().unwrap();
        assert_eq!(received.notification.method, "changed");
        assert_eq!(received.notification.provider.as_str(), "web");
    }

    #[test]
    fn descendant_service_routing_resolves_the_durable_root() {
        let mut runtime = ConductorRuntime::new();
        runtime
            .register_agent(AgentDefinition::new(
                descriptor("agent.child", CallableKind::Agent),
                ExecutionAuthority::read_only(),
            ))
            .unwrap();
        runtime
            .register_orchestration(OrchestrationDefinition {
                descriptor: descriptor("orchestration.tree", CallableKind::Orchestration),
                interface_agent: None,
                nodes: vec![OrchestrationNode {
                    id: OrchestrationNodeId::parse("child").unwrap(),
                    callable: CallableId::parse("agent.child").unwrap(),
                    depends_on: Vec::new(),
                    objective: Some("child".to_owned()),
                    input_bindings: BTreeMap::new(),
                }],
                output_bindings: BTreeMap::new(),
            })
            .unwrap();
        let session = runtime
            .create_session(None, None, ExecutionTarget::Fixed(model_target()))
            .unwrap();
        let root = runtime.submit(&session.id, "root").unwrap();
        let orchestration = runtime
            .start_orchestration(
                &root.id,
                &CallableId::parse("orchestration.tree").unwrap(),
                serde_json::json!({}),
            )
            .unwrap();
        let child = runtime
            .snapshot()
            .executions
            .into_iter()
            .find(|execution| execution.parent_execution.as_ref() == Some(&orchestration.id))
            .unwrap();
        let server = ConductorServer::new(runtime);
        assert_eq!(
            server.inner.execution_group_id_for(&child.id).unwrap(),
            root.id
        );
    }

    #[test]
    fn restored_server_starts_without_frontend_routes() {
        let mut runtime = ConductorRuntime::new();
        let session = runtime
            .create_session(None, None, ExecutionTarget::Fixed(model_target()))
            .unwrap();
        let execution = runtime.submit(&session.id, "root").unwrap();
        let server = ConductorServer::new(runtime);
        let error = server
            .request_frontend_service(
                &execution.id,
                FrontendServiceRequest {
                    provider: FrontendServiceProviderId::parse("web").unwrap(),
                    method: "search".to_owned(),
                    params: Value::Null,
                },
            )
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::UnsupportedCapability);
    }
}
