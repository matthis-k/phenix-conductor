use phenix_backend::{
    Backend, BackendCapabilities, BackendError, BackendEvent, BackendExecutionRequest, BackendHost,
    BackendSession, BackendSessionRequest,
};
use phenix_conductor::{
    ConductorRuntime, ConductorServer, ConductorService, ExecutionProvider, ExecutionProviderError,
    ExecutionProviderHost, ExecutionProviderKind, ExecutionProviderRequest,
};
use phenix_core::{
    AgentDefinition, AuthenticationState, BackendCatalog, BackendId, CallableDescriptor,
    CallableId, CallableKind, CallablePolicy, CapabilitySet, ExecutionAuthority,
    ExecutionEventKind, ExecutionId, ExecutionState, ExecutionTarget, InferenceOptions,
    ModelDescriptor, ModelId, ModelTarget, OrchestrationDefinition, OrchestrationNode,
    OrchestrationNodeId, ProviderId, SessionId,
};
use phenix_protocol::{
    ClientEnvelope, ClientMessage, Command, FrontendConnectionCommand,
    FrontendServiceProviderDescriptor, FrontendServiceProviderId, FrontendServiceRequest,
    FrontendServiceResponse, FrontendServiceResponsePayload, Reply, ResponsePayload, ServerMessage,
};
use serde::Serialize;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, BufRead, BufReader, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::{Duration, Instant};

struct BlockingBackend {
    started: Arc<AtomicBool>,
    cancel_called: Arc<AtomicBool>,
    release: Arc<AtomicBool>,
    late_event_rejected: Arc<AtomicBool>,
}

struct BlockingSession {
    started: Arc<AtomicBool>,
    cancel_called: Arc<AtomicBool>,
    release: Arc<AtomicBool>,
    late_event_rejected: Arc<AtomicBool>,
}

fn target() -> ModelTarget {
    ModelTarget {
        backend: BackendId::parse("blocking").unwrap(),
        provider: ProviderId::parse("fixture").unwrap(),
        model: ModelId::parse("fixture-model").unwrap(),
        inference: InferenceOptions::default(),
    }
}

impl Backend for BlockingBackend {
    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            tool_presentations: BTreeSet::new(),
            images: false,
            persistent_sessions: false,
        }
    }

    fn catalog(&mut self) -> Result<BackendCatalog, BackendError> {
        Ok(BackendCatalog {
            backend: BackendId::parse("blocking").unwrap(),
            models: vec![ModelDescriptor {
                target: target(),
                name: "Fixture Model".to_owned(),
                selectable: true,
            }],
            authentication_state: AuthenticationState::NotRequired,
            authentication_methods: Vec::new(),
        })
    }

    fn open_session(
        &mut self,
        _request: BackendSessionRequest,
    ) -> Result<Arc<dyn BackendSession>, BackendError> {
        Ok(Arc::new(BlockingSession {
            started: self.started.clone(),
            cancel_called: self.cancel_called.clone(),
            release: self.release.clone(),
            late_event_rejected: self.late_event_rejected.clone(),
        }))
    }
}

impl BackendSession for BlockingSession {
    fn execute(
        &self,
        _request: BackendExecutionRequest,
        host: &mut dyn BackendHost,
    ) -> Result<(), BackendError> {
        self.started.store(true, Ordering::SeqCst);
        while !self.release.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(2));
        }
        if host
            .emit(BackendEvent::ContentDelta("late".to_owned()))
            .is_err()
        {
            self.late_event_rejected.store(true, Ordering::SeqCst);
        }
        Ok(())
    }

    fn cancel(&self, _execution_id: &phenix_core::ExecutionId) -> Result<(), BackendError> {
        self.cancel_called.store(true, Ordering::SeqCst);
        self.release.store(true, Ordering::SeqCst);
        Ok(())
    }
}

fn send(stream: &mut UnixStream, id: u64, command: Command) {
    let message = ClientMessage { id, command };
    writeln!(stream, "{}", serde_json::to_string(&message).unwrap()).unwrap();
    stream.flush().unwrap();
}

fn read_response(reader: &mut BufReader<UnixStream>, expected_id: u64) -> Reply {
    loop {
        let mut line = String::new();
        assert_ne!(
            reader.read_line(&mut line).unwrap(),
            0,
            "server closed output"
        );
        let message: ServerMessage = serde_json::from_str(line.trim()).unwrap();
        if let ServerMessage::Response { id, response } = message {
            if id == expected_id {
                return match response {
                    ResponsePayload::Ok { result } => result,
                    ResponsePayload::Error { error } => {
                        panic!("request {expected_id} failed: {error:?}")
                    }
                };
            }
        }
    }
}

fn wait_until(flag: &AtomicBool, label: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !flag.load(Ordering::SeqCst) {
        assert!(Instant::now() < deadline, "timed out waiting for {label}");
        thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn active_turn_remains_queryable_and_cancellable() {
    let started = Arc::new(AtomicBool::new(false));
    let cancel_called = Arc::new(AtomicBool::new(false));
    let release = Arc::new(AtomicBool::new(false));
    let late_event_rejected = Arc::new(AtomicBool::new(false));

    let mut server = ConductorServer::new(ConductorRuntime::new());
    server
        .register_backend(
            BackendId::parse("blocking").unwrap(),
            Box::new(BlockingBackend {
                started: started.clone(),
                cancel_called: cancel_called.clone(),
                release: release.clone(),
                late_event_rejected: late_event_rejected.clone(),
            }),
        )
        .unwrap();

    let (mut frontend, server_socket) = UnixStream::pair().unwrap();
    frontend
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let server_input = BufReader::new(server_socket.try_clone().unwrap());
    let server_thread = thread::spawn(move || {
        server.serve_ndjson(server_input, server_socket).unwrap();
        server
    });
    let mut reader = BufReader::new(frontend.try_clone().unwrap());

    send(
        &mut frontend,
        1,
        Command::Initialize {
            after_sequence: Some(0),
        },
    );
    assert!(matches!(
        read_response(&mut reader, 1),
        Reply::Initialized { .. }
    ));

    send(
        &mut frontend,
        2,
        Command::CreateSession {
            parent_session: None,
            name: Some("root".to_owned()),
            target: ExecutionTarget::Fixed(target()),
        },
    );
    assert!(matches!(
        read_response(&mut reader, 2),
        Reply::Session { .. }
    ));

    send(
        &mut frontend,
        3,
        Command::Submit {
            session_id: SessionId::parse("session-1").unwrap(),
            text: "block".to_owned(),
        },
    );
    assert!(matches!(
        read_response(&mut reader, 3),
        Reply::Execution { .. }
    ));
    wait_until(&started, "backend execution to start");

    send(&mut frontend, 4, Command::GetSnapshot);
    let Reply::Snapshot { snapshot, .. } = read_response(&mut reader, 4) else {
        panic!("snapshot request returned the wrong reply");
    };
    assert_eq!(snapshot.executions.len(), 1);
    assert_eq!(snapshot.executions[0].state, ExecutionState::Running);

    send(
        &mut frontend,
        5,
        Command::CancelExecution {
            execution_id: phenix_core::ExecutionId::parse("execution-1").unwrap(),
        },
    );
    assert_eq!(read_response(&mut reader, 5), Reply::Accepted);
    wait_until(&cancel_called, "backend cancellation hook");
    wait_until(&late_event_rejected, "late backend event rejection");

    frontend.shutdown(Shutdown::Write).unwrap();
    let server = server_thread.join().unwrap();
    let runtime = server.runtime();
    let snapshot = runtime.snapshot();
    assert_eq!(snapshot.executions[0].state, ExecutionState::Cancelled);
    assert!(!runtime.events_since(0).iter().any(|event| {
        matches!(
            &event.kind,
            ExecutionEventKind::AssistantContentDelta { text } if text == "late"
        )
    }));
}

struct BlockingProvider {
    started: Arc<AtomicBool>,
    release: Arc<AtomicBool>,
}

impl ExecutionProvider for BlockingProvider {
    fn kind(&self) -> ExecutionProviderKind {
        ExecutionProviderKind::Native
    }

    fn execute(
        &self,
        _request: &ExecutionProviderRequest,
        _host: &mut dyn ExecutionProviderHost,
    ) -> Result<(), ExecutionProviderError> {
        self.started.store(true, Ordering::SeqCst);
        while !self.release.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(2));
        }
        Ok(())
    }
}

fn frontend_descriptor(id: &str, kind: CallableKind) -> CallableDescriptor {
    CallableDescriptor {
        id: CallableId::parse(id).unwrap(),
        kind,
        description: "frontend service fixture".to_owned(),
        input_schema: json!({"type": "object"}),
        output_schema: json!({"type": "object"}),
        capabilities: CapabilitySet::default(),
        policy: CallablePolicy::default(),
    }
}

fn frontend_provider() -> FrontendServiceProviderDescriptor {
    FrontendServiceProviderDescriptor {
        id: FrontendServiceProviderId::parse("web").unwrap(),
        capabilities: BTreeSet::from(["search".to_owned()]),
    }
}

fn write_json(stream: &mut UnixStream, message: &impl Serialize) {
    writeln!(stream, "{}", serde_json::to_string(message).unwrap()).unwrap();
    stream.flush().unwrap();
}

fn wait_for_response(reader: &mut BufReader<UnixStream>, id: u64) -> Reply {
    loop {
        let mut line = String::new();
        assert!(reader.read_line(&mut line).unwrap() > 0);
        match serde_json::from_str::<ServerMessage>(&line).unwrap() {
            ServerMessage::Response {
                id: response_id,
                response: ResponsePayload::Ok { result },
            } if response_id == id => return result,
            ServerMessage::Response {
                id: response_id,
                response: ResponsePayload::Error { error },
            } if response_id == id => panic!("request {id} failed: {error:?}"),
            _ => {}
        }
    }
}

fn assert_no_frontend_service_request(reader: &mut BufReader<UnixStream>) {
    reader
        .get_ref()
        .set_read_timeout(Some(Duration::from_millis(100)))
        .unwrap();
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => panic!("other frontend disconnected"),
            Ok(_) => {
                assert!(
                    !matches!(
                        serde_json::from_str::<ServerMessage>(&line),
                        Ok(ServerMessage::FrontendServiceRequest { .. })
                    ),
                    "frontend service request leaked to another frontend"
                );
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                break;
            }
            Err(error) => panic!("unexpected frontend read error: {error}"),
        }
    }
    reader.get_ref().set_read_timeout(None).unwrap();
}

#[test]
fn descendant_frontend_service_call_stays_with_the_root_frontend() {
    let started = Arc::new(AtomicBool::new(false));
    let release = Arc::new(AtomicBool::new(false));
    let mut runtime = ConductorRuntime::new();
    runtime
        .register_provider_agent(
            AgentDefinition::new(
                frontend_descriptor("agent.blocking", CallableKind::Agent),
                ExecutionAuthority::read_only(),
            ),
            BlockingProvider {
                started: started.clone(),
                release: release.clone(),
            },
        )
        .unwrap();
    runtime
        .register_orchestration(OrchestrationDefinition {
            descriptor: frontend_descriptor("orchestration.frontend", CallableKind::Orchestration),
            interface_agent: None,
            nodes: vec![OrchestrationNode {
                id: OrchestrationNodeId::parse("child").unwrap(),
                callable: CallableId::parse("agent.blocking").unwrap(),
                depends_on: Vec::new(),
                objective: Some("hold child open".to_owned()),
                input_bindings: BTreeMap::new(),
            }],
            output_bindings: BTreeMap::new(),
        })
        .unwrap();
    let session = runtime
        .create_session(None, None, ExecutionTarget::Fixed(target()))
        .unwrap();
    let service = ConductorService::new(ConductorServer::new(runtime)).unwrap();

    let (mut owner_client, owner_server) = UnixStream::pair().unwrap();
    let owner_writer = owner_server.try_clone().unwrap();
    let owner_service = service.clone();
    let owner_thread = thread::spawn(move || {
        owner_service.serve_connection(BufReader::new(owner_server), owner_writer)
    });
    let mut owner_reader = BufReader::new(owner_client.try_clone().unwrap());

    let (mut other_client, other_server) = UnixStream::pair().unwrap();
    let other_writer = other_server.try_clone().unwrap();
    let other_service = service.clone();
    let other_thread = thread::spawn(move || {
        other_service.serve_connection(BufReader::new(other_server), other_writer)
    });
    let mut other_reader = BufReader::new(other_client.try_clone().unwrap());

    write_json(
        &mut owner_client,
        &ClientEnvelope::ConnectionCommand(
            FrontendConnectionCommand::SetFrontendServiceProviders {
                id: 1,
                providers: vec![frontend_provider()],
            },
        ),
    );
    assert_eq!(wait_for_response(&mut owner_reader, 1), Reply::Accepted);
    write_json(
        &mut other_client,
        &ClientEnvelope::ConnectionCommand(
            FrontendConnectionCommand::SetFrontendServiceProviders {
                id: 1,
                providers: vec![frontend_provider()],
            },
        ),
    );
    assert_eq!(wait_for_response(&mut other_reader, 1), Reply::Accepted);

    write_json(
        &mut owner_client,
        &ClientEnvelope::Command(ClientMessage {
            id: 2,
            command: Command::StartCallable {
                session_id: session.id,
                callable: CallableId::parse("orchestration.frontend").unwrap(),
                input: json!({}),
            },
        }),
    );

    let mut root = None;
    let mut child = None;
    while root.is_none() || child.is_none() {
        let mut line = String::new();
        assert!(owner_reader.read_line(&mut line).unwrap() > 0);
        match serde_json::from_str::<ServerMessage>(&line).unwrap() {
            ServerMessage::Response {
                id: 2,
                response:
                    ResponsePayload::Ok {
                        result: Reply::Execution { execution },
                    },
            } => root = Some(execution.id),
            ServerMessage::Event { event } => {
                if let ExecutionEventKind::ChildExecutionStarted { child: child_id } = event.kind {
                    child = Some(child_id);
                }
            }
            _ => {}
        }
    }
    let root = root.unwrap();
    let child = child.unwrap();
    assert_ne!(root, child);
    wait_until(&started, "native child execution to start");

    let call_service = service.clone();
    let call_child = child.clone();
    let call = thread::spawn(move || {
        call_service.request_frontend_service(
            &call_child,
            FrontendServiceRequest {
                provider: FrontendServiceProviderId::parse("web").unwrap(),
                method: "search".to_owned(),
                params: json!({"query": "nixos"}),
            },
        )
    });

    let request_id = loop {
        let mut line = String::new();
        assert!(owner_reader.read_line(&mut line).unwrap() > 0);
        if let ServerMessage::FrontendServiceRequest { id, request } =
            serde_json::from_str::<ServerMessage>(&line).unwrap()
        {
            assert_eq!(request.provider.as_str(), "web");
            assert_eq!(request.method, "search");
            break id;
        }
    };
    assert_no_frontend_service_request(&mut other_reader);

    write_json(
        &mut owner_client,
        &ClientEnvelope::FrontendServiceResponse(FrontendServiceResponse {
            id: request_id,
            response: FrontendServiceResponsePayload::Ok {
                result: json!({"items": ["owned"]}),
            },
        }),
    );
    assert_eq!(call.join().unwrap().unwrap(), json!({"items": ["owned"]}));

    release.store(true, Ordering::SeqCst);
    drop(owner_reader);
    drop(other_reader);
    drop(owner_client);
    drop(other_client);
    owner_thread.join().unwrap().unwrap();
    other_thread.join().unwrap().unwrap();
}
