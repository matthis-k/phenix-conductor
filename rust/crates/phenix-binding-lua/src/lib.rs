#![forbid(unsafe_code)]

//! LuaJIT and Lua 5.1 native entry point for the Phenix ACP client.
//!
//! The module only touches Lua from calls made by the host. ACP work runs on a
//! background thread. Lua applications poll request handles to receive results
//! and ordered updates, so the binding never invokes a host event loop itself.

use agent_client_protocol::schema::v1::{
    CancelNotification, CloseSessionRequest, ContentBlock, ListSessionsRequest, LoadSessionRequest,
    NewSessionRequest, PromptRequest, ResumeSessionRequest, SetSessionConfigOptionRequest,
    TextContent,
};
use futures::{
    channel::{mpsc, oneshot},
    StreamExt,
};
use mlua::{
    Error as LuaError, Lua, LuaSerdeExt, MultiValue, RegistryKey, Result as LuaResult, Table,
    UserData, UserDataMethods, Value,
};
use phenix_application_interface::{
    types::{CapabilityInvokeInput, CapabilityInvokeResult, Empty},
    GetSdk, InvokeCapability, Operation,
};
use phenix_client_acp::{
    application_descriptor, AcpClient, ApplicationEvent, ClientError, ExtensionCallbacks,
    ExtensionUpdates, SessionUpdates, StdioConfig, INTERFACE_ID,
};
use phenix_core::{
    CallableRef, CapabilityGenerationId, CapabilityOwnerId, ClientConnectionId, ContractId, Key,
    PhenixSchema, PhenixValue, ReferenceId, Type, ValueCodec,
};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    num::NonZeroUsize,
    path::PathBuf,
    rc::Rc,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc as std_mpsc, Arc, Mutex,
    },
    thread,
};

static NEXT_CONNECTION: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Eq, PartialEq)]
enum ErrorKind {
    Transport,
    Protocol,
    Cancelled,
    Rejected,
    UnsupportedCapability,
    QueueFull,
    Conversion,
}

impl ErrorKind {
    const fn as_str(&self) -> &'static str {
        match self {
            Self::Transport => "transport",
            Self::Protocol => "protocol",
            Self::Cancelled => "cancelled",
            Self::Rejected => "rejected",
            Self::UnsupportedCapability => "unsupported_capability",
            Self::QueueFull => "queue_full",
            Self::Conversion => "conversion",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct BindingError {
    kind: ErrorKind,
    code: String,
    message: String,
    details: Box<Option<serde_json::Value>>,
}

impl BindingError {
    fn local(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            code: kind.as_str().to_owned(),
            kind,
            message: message.into(),
            details: Box::new(None),
        }
    }

    fn transport(message: impl Into<String>) -> Self {
        Self::local(ErrorKind::Transport, message)
    }

    fn conversion(message: impl Into<String>) -> Self {
        Self::local(ErrorKind::Conversion, message)
    }

    fn unsupported(operation: &ContractId) -> Self {
        Self::local(
            ErrorKind::UnsupportedCapability,
            format!("application operation {operation} is not a negotiated ACP extension"),
        )
    }

    fn from_client(error: ClientError) -> Self {
        let message = error.to_string();
        let (kind, code, details) = match error {
            ClientError::Transport(_) => {
                (ErrorKind::Transport, "transport".to_owned(), Box::new(None))
            }
            ClientError::Protocol(_) | ClientError::OutOfOrderUpdate { .. } => {
                (ErrorKind::Protocol, "protocol".to_owned(), Box::new(None))
            }
            ClientError::Cancelled { details, .. } => {
                (ErrorKind::Cancelled, "cancelled".to_owned(), details)
            }
            ClientError::Rejected(rejection) => (
                ErrorKind::Rejected,
                rejection.class.unwrap_or_else(|| "rejected".to_owned()),
                Box::new(rejection.details),
            ),
            ClientError::UnsupportedCapability { .. } => (
                ErrorKind::UnsupportedCapability,
                "unsupported_capability".to_owned(),
                Box::new(None),
            ),
            ClientError::UpdateQueueFull => (
                ErrorKind::QueueFull,
                "queue_full".to_owned(),
                Box::new(None),
            ),
        };
        Self {
            kind,
            code,
            message,
            details,
        }
    }
}

type CommandResult = Result<Response, BindingError>;

#[derive(Clone, Debug, PartialEq)]
enum Response {
    Session(String),
    Json(serde_json::Value),
    Application {
        operation: ContractId,
        value: PhenixValue,
    },
    Projected {
        schema: PhenixSchema,
        value: PhenixValue,
    },
    PromptComplete(String),
    Acknowledged,
}

enum Command {
    NewSession {
        cwd: PathBuf,
        reply: oneshot::Sender<CommandResult>,
    },
    ListSessions {
        cwd: Option<PathBuf>,
        cursor: Option<String>,
        reply: oneshot::Sender<CommandResult>,
    },
    ResumeSession {
        session_id: String,
        cwd: PathBuf,
        reply: oneshot::Sender<CommandResult>,
    },
    LoadSession {
        session_id: String,
        cwd: PathBuf,
        reply: oneshot::Sender<CommandResult>,
    },
    CloseSession {
        session_id: String,
        reply: oneshot::Sender<CommandResult>,
    },
    Prompt {
        session_id: String,
        text: String,
        reply: oneshot::Sender<CommandResult>,
    },
    SetOption {
        session_id: String,
        config_id: String,
        value: String,
        reply: oneshot::Sender<CommandResult>,
    },
    Application {
        operation: ContractId,
        input: PhenixValue,
        reply: oneshot::Sender<CommandResult>,
    },
    InvokeCapability {
        input: CapabilityInvokeInput,
        output_schema: PhenixSchema,
        reply: oneshot::Sender<CommandResult>,
    },
    Cancel {
        session_id: String,
    },
}

struct ClientState {
    commands: mpsc::UnboundedSender<Command>,
    updates: Mutex<std_mpsc::Receiver<agent_client_protocol::schema::v1::SessionNotification>>,
    extension_updates: Mutex<std_mpsc::Receiver<ApplicationEvent>>,
    callbacks: Mutex<std_mpsc::Receiver<phenix_client_acp::ExtensionCallbackRequest>>,
    capabilities: Mutex<BTreeSet<String>>,
    extensions: Mutex<BTreeSet<String>>,
    terminal_error: Mutex<Option<BindingError>>,
    owner: ClientConnectionId,
    generation: CapabilityGenerationId,
}

impl ClientState {
    fn send(&self, command: Command) -> Result<(), BindingError> {
        self.commands
            .unbounded_send(command)
            .map_err(|_| self.failure())
    }

    fn record_failure(&self, error: BindingError) {
        if let Ok(mut terminal_error) = self.terminal_error.lock() {
            *terminal_error = Some(error);
        }
    }

    fn failure(&self) -> BindingError {
        self.terminal_error
            .lock()
            .ok()
            .and_then(|error| error.clone())
            .unwrap_or_else(|| BindingError::transport("ACP connection is closed"))
    }

    fn supports_extension(&self, operation: &ContractId) -> Result<bool, BindingError> {
        self.extensions
            .lock()
            .map(|extensions| extensions.contains(operation.as_str()))
            .map_err(|_| BindingError::transport("extension lock is poisoned"))
    }
}

#[derive(Clone)]
struct Client {
    state: Arc<ClientState>,
    local_callables: Rc<RefCell<LocalCallables>>,
}

struct LocalCallable {
    schema: Type,
    function: RegistryKey,
}

#[derive(Default)]
struct LocalCallables {
    next: u64,
    entries: BTreeMap<ReferenceId, LocalCallable>,
}

impl LocalCallables {
    fn lift(
        &mut self,
        lua: &Lua,
        state: &ClientState,
        schema: Type,
        function: mlua::Function,
    ) -> Result<CallableRef, BindingError> {
        let Type::Callable { contract, .. } = &schema else {
            return Err(BindingError::conversion(
                "Lua functions require an expected callable schema",
            ));
        };
        self.next = self.next.saturating_add(1);
        let id = ReferenceId::parse(format!("lua-callable-{}", self.next))
            .expect("generated Lua callable ids are valid");
        let function = lua
            .create_registry_value(function)
            .map_err(|error| BindingError::conversion(error.to_string()))?;
        self.entries.insert(
            id.clone(),
            LocalCallable {
                schema: schema.clone(),
                function,
            },
        );
        Ok(CallableRef::new(
            contract.clone(),
            CapabilityOwnerId::Client(state.owner.clone()),
            state.generation.clone(),
            id,
        ))
    }
}

#[derive(Clone)]
struct Sessions {
    state: Arc<ClientState>,
}

#[derive(Clone)]
struct Session {
    state: Arc<ClientState>,
    id: String,
}

struct Request {
    state: Arc<ClientState>,
    receiver: Option<oneshot::Receiver<CommandResult>>,
    result: Option<CommandResult>,
    local_callables: Option<Rc<RefCell<LocalCallables>>>,
}

impl Request {
    fn pending(
        state: Arc<ClientState>,
        local_callables: Option<Rc<RefCell<LocalCallables>>>,
        receiver: oneshot::Receiver<CommandResult>,
    ) -> Self {
        Self {
            state,
            receiver: Some(receiver),
            result: None,
            local_callables,
        }
    }

    fn poll(&mut self) -> Option<CommandResult> {
        if let Some(result) = &self.result {
            return Some(result.clone());
        }
        let receiver = self.receiver.as_mut()?;
        match receiver.try_recv() {
            Ok(Some(result)) => {
                self.receiver = None;
                self.result = Some(result.clone());
                Some(result)
            }
            Ok(None) => None,
            Err(_) => {
                let result = Err(self.state.failure());
                self.receiver = None;
                self.result = Some(result.clone());
                Some(result)
            }
        }
    }
}

impl UserData for Client {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("capabilities", |lua, this, ()| {
            capabilities(lua, &this.state)
        });
        methods.add_method("extensions", |lua, this, ()| {
            let extensions =
                this.state.extensions.lock().map_err(|_| {
                    lua_error(BindingError::transport("extension lock is poisoned"))
                })?;
            let result = lua.create_table()?;
            for operation in extensions.iter() {
                result.set(operation.as_str(), true)?;
            }
            Ok(result)
        });
        methods.add_method("sessions", |lua, this, ()| {
            lua.create_userdata(Sessions {
                state: Arc::clone(&this.state),
            })
        });
        methods.add_method("application", |lua, this, ()| -> LuaResult<Table> {
            let descriptor = descriptor(lua)?;
            let bind: mlua::Function = descriptor.get("bind")?;
            let application: Table = bind.call(lua.create_userdata(this.clone())?)?;
            let operations: Table = descriptor.get("operations")?;
            let extensions =
                this.state.extensions.lock().map_err(|_| {
                    lua_error(BindingError::transport("extension lock is poisoned"))
                })?;
            for pair in operations.pairs::<String, Table>() {
                let (operation, metadata) = pair?;
                if !extensions.contains(operation.as_str()) {
                    let name: String = metadata.get("name")?;
                    application.set(name, Value::Nil)?;
                }
            }
            Ok(application)
        });
        methods.add_method("sdk", |lua, this, ()| {
            let operation = ContractId::parse(GetSdk::ID).expect("static application operation id");
            if !this
                .state
                .supports_extension(&operation)
                .map_err(lua_error)?
            {
                return Err(lua_error(BindingError::unsupported(&operation)));
            }
            let request = request_for(
                &this.state,
                Some(Rc::clone(&this.local_callables)),
                |reply| Command::Application {
                    operation,
                    input: Empty {}.to_value(),
                    reply,
                },
            )?;
            lua.create_userdata(request)
        });
        methods.add_method(
            "_invoke_application",
            |lua, this, (operation, input): (String, Value)| {
                let operation = ContractId::parse(operation)
                    .map_err(|error| lua_error(BindingError::conversion(error)))?;
                if !this
                    .state
                    .supports_extension(&operation)
                    .map_err(lua_error)?
                {
                    return Err(lua_error(BindingError::unsupported(&operation)));
                }
                let descriptor = application_descriptor();
                let declaration = descriptor.operations.get(&operation).ok_or_else(|| {
                    lua_error(BindingError::conversion(format!(
                        "unknown application operation {operation}"
                    )))
                })?;
                let schema = descriptor.types.get(&declaration.input).ok_or_else(|| {
                    lua_error(BindingError::conversion(format!(
                        "application descriptor is missing input schema {}",
                        declaration.input
                    )))
                })?;
                let input =
                    lua_to_phenix_with_host(lua, schema, input, &this.state, &this.local_callables)
                        .map_err(lua_error)?;
                let request = request_for(
                    &this.state,
                    Some(Rc::clone(&this.local_callables)),
                    |reply| Command::Application {
                        operation,
                        input,
                        reply,
                    },
                )?;
                lua.create_userdata(request)
            },
        );
        methods.add_method("poll", |lua, this, ()| {
            let notification = this
                .state
                .updates
                .lock()
                .map_err(|_| {
                    lua_error(BindingError::transport("ACP update queue lock is poisoned"))
                })?
                .try_recv();
            match notification {
                Ok(notification) => {
                    let value = serde_json::to_value(notification)
                        .map_err(|error| lua_error(BindingError::conversion(error.to_string())))?;
                    return lua.to_value(&value);
                }
                Err(std_mpsc::TryRecvError::Empty) => {}
                Err(std_mpsc::TryRecvError::Disconnected) => {
                    return Err(lua_error(this.state.failure()));
                }
            }
            let extension = this
                .state
                .extension_updates
                .lock()
                .map_err(|_| {
                    lua_error(BindingError::transport(
                        "ACP extension event queue lock is poisoned",
                    ))
                })?
                .try_recv();
            match extension {
                Ok(event) => application_event_to_lua(lua, event),
                Err(std_mpsc::TryRecvError::Empty) => {
                    let callback = this
                        .state
                        .callbacks
                        .lock()
                        .map_err(|_| {
                            lua_error(BindingError::transport("callback queue lock is poisoned"))
                        })?
                        .try_recv();
                    match callback {
                        Ok(callback) => {
                            dispatch_local_callback(
                                lua,
                                &this.state,
                                &this.local_callables,
                                callback,
                            )?;
                            Ok(Value::Nil)
                        }
                        Err(std_mpsc::TryRecvError::Empty) => Ok(Value::Nil),
                        Err(std_mpsc::TryRecvError::Disconnected) => {
                            Err(lua_error(this.state.failure()))
                        }
                    }
                }
                Err(std_mpsc::TryRecvError::Disconnected) => Err(lua_error(this.state.failure())),
            }
        });
    }
}

impl UserData for Sessions {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("new", |lua, this, cwd: String| {
            let request = request_for(&this.state, None, |reply| Command::NewSession {
                cwd: PathBuf::from(cwd),
                reply,
            })?;
            lua.create_userdata(request)
        });
        methods.add_method(
            "list",
            |lua, this, (cwd, cursor): (Option<String>, Option<String>)| {
                let request = request_for(&this.state, None, |reply| Command::ListSessions {
                    cwd: cwd.map(PathBuf::from),
                    cursor,
                    reply,
                })?;
                lua.create_userdata(request)
            },
        );
        methods.add_method(
            "resume",
            |lua, this, (session_id, cwd): (String, String)| {
                let request = request_for(&this.state, None, |reply| Command::ResumeSession {
                    session_id,
                    cwd: PathBuf::from(cwd),
                    reply,
                })?;
                lua.create_userdata(request)
            },
        );
        methods.add_method("load", |lua, this, (session_id, cwd): (String, String)| {
            let request = request_for(&this.state, None, |reply| Command::LoadSession {
                session_id,
                cwd: PathBuf::from(cwd),
                reply,
            })?;
            lua.create_userdata(request)
        });
    }
}

impl UserData for Session {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("id", |_lua, this, ()| Ok(this.id.clone()));
        methods.add_method("prompt", |lua, this, text: String| {
            let request = request_for(&this.state, None, |reply| Command::Prompt {
                session_id: this.id.clone(),
                text,
                reply,
            })?;
            lua.create_userdata(request)
        });
        methods.add_method(
            "set_option",
            |lua, this, (config_id, value): (String, String)| {
                let request = request_for(&this.state, None, |reply| Command::SetOption {
                    session_id: this.id.clone(),
                    config_id,
                    value,
                    reply,
                })?;
                lua.create_userdata(request)
            },
        );
        methods.add_method("close", |lua, this, ()| {
            let request = request_for(&this.state, None, |reply| Command::CloseSession {
                session_id: this.id.clone(),
                reply,
            })?;
            lua.create_userdata(request)
        });
        methods.add_method("cancel", |_lua, this, ()| {
            this.state
                .send(Command::Cancel {
                    session_id: this.id.clone(),
                })
                .map_err(lua_error)?;
            Ok(())
        });
    }
}

impl UserData for Request {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("poll", |lua, this, ()| match this.poll() {
            None => Ok(MultiValue::new()),
            Some(Ok(response)) => {
                response_values(lua, &this.state, this.local_callables.as_ref(), response)
            }
            Some(Err(error)) => error_values(lua, &error),
        });
    }
}

fn request_for(
    state: &Arc<ClientState>,
    local_callables: Option<Rc<RefCell<LocalCallables>>>,
    make_command: impl FnOnce(oneshot::Sender<CommandResult>) -> Command,
) -> Result<Request, LuaError> {
    let (reply, receiver) = oneshot::channel();
    state.send(make_command(reply)).map_err(lua_error)?;
    Ok(Request::pending(
        Arc::clone(state),
        local_callables,
        receiver,
    ))
}

fn response_values(
    lua: &Lua,
    state: &Arc<ClientState>,
    local_callables: Option<&Rc<RefCell<LocalCallables>>>,
    response: Response,
) -> LuaResult<MultiValue> {
    let value = match response {
        Response::Session(id) => Value::UserData(lua.create_userdata(Session {
            state: Arc::clone(state),
            id,
        })?),
        Response::Json(value) => lua.to_value(&value)?,
        Response::Application { operation, value } => {
            let descriptor = application_descriptor();
            let declaration = descriptor.operations.get(&operation).ok_or_else(|| {
                lua_error(BindingError::conversion(format!(
                    "unknown application operation {operation}"
                )))
            })?;
            let schema = descriptor.types.get(&declaration.output).ok_or_else(|| {
                lua_error(BindingError::conversion(format!(
                    "application descriptor is missing output schema {}",
                    declaration.output
                )))
            })?;
            schema.parse(&value).map_err(|error| {
                lua_error(BindingError::conversion(format!(
                    "application output violates descriptor: {error}"
                )))
            })?;
            phenix_to_lua_with_state(lua, Some(state), local_callables, schema, &value)
                .map_err(lua_error)?
        }
        Response::Projected { schema, value } => {
            phenix_to_lua_with_state(lua, Some(state), local_callables, &schema, &value)
                .map_err(lua_error)?
        }
        Response::PromptComplete(stop_reason) => {
            let result = lua.create_table()?;
            result.set("kind", "complete")?;
            result.set("stop_reason", stop_reason)?;
            Value::Table(result)
        }
        Response::Acknowledged => {
            let result = lua.create_table()?;
            result.set("ok", true)?;
            Value::Table(result)
        }
    };
    Ok(MultiValue::from_vec(vec![value]))
}

fn error_values(lua: &Lua, error: &BindingError) -> LuaResult<MultiValue> {
    let result = lua.create_table()?;
    result.set("kind", error.kind.as_str())?;
    result.set("code", error.code.as_str())?;
    result.set("message", error.message.as_str())?;
    if let Some(details) = error
        .details
        .as_ref()
        .as_ref()
        .filter(|details| !details.is_null())
    {
        result.set("details", lua.to_value(&details)?)?;
    }
    Ok(MultiValue::from_vec(vec![Value::Nil, Value::Table(result)]))
}

fn lua_error(error: BindingError) -> LuaError {
    LuaError::RuntimeError(format!("{}: {}", error.code, error.message))
}

fn capabilities(lua: &Lua, state: &ClientState) -> LuaResult<Table> {
    let capabilities = state
        .capabilities
        .lock()
        .map_err(|_| lua_error(BindingError::transport("capability lock is poisoned")))?;
    let result = lua.create_table()?;
    for capability in capabilities.iter() {
        result.set(capability.as_str(), true)?;
    }
    Ok(result)
}

fn descriptor(lua: &Lua) -> LuaResult<Table> {
    let source = phenix_binding_generator::lua(&application_descriptor())
        .map_err(|error| LuaError::RuntimeError(error.to_string()))?;
    lua.load(source).eval()
}

fn parse_config(options: Table) -> LuaResult<StdioConfig> {
    let command = options
        .get::<String>("command")
        .map_err(|_| lua_error(BindingError::conversion("connect options require command")))?;
    if command.is_empty() {
        return Err(lua_error(BindingError::conversion(
            "connect command must not be empty",
        )));
    }
    let args = options
        .get::<Option<Table>>("args")?
        .map(|args| {
            args.sequence_values::<String>()
                .collect::<LuaResult<Vec<_>>>()
                .map_err(|_| lua_error(BindingError::conversion("connect args must be strings")))
        })
        .transpose()?
        .unwrap_or_default();
    let env = options
        .get::<Option<Table>>("env")?
        .map(|env| {
            env.pairs::<String, String>()
                .collect::<LuaResult<BTreeMap<_, _>>>()
                .map_err(|_| {
                    lua_error(BindingError::conversion(
                        "connect env must map strings to strings",
                    ))
                })
        })
        .transpose()?
        .unwrap_or_default();

    let mut config = StdioConfig::new(command).args(args);
    for (name, value) in env {
        config = config.env(name, value);
    }
    Ok(config)
}

fn connect(options: Table) -> LuaResult<Client> {
    let config = parse_config(options)?;
    let (commands, receiver) = mpsc::unbounded();
    let (updates, update_receiver) = SessionUpdates::bounded(
        NonZeroUsize::new(256).expect("static update queue capacity is non-zero"),
    );
    let (extension_updates, extension_update_receiver) = ExtensionUpdates::bounded(
        NonZeroUsize::new(256).expect("static extension queue capacity is non-zero"),
    );
    let (callbacks, callback_receiver) = ExtensionCallbacks::bounded(
        NonZeroUsize::new(256).expect("static callback queue capacity is non-zero"),
    );
    let connection = NEXT_CONNECTION.fetch_add(1, Ordering::Relaxed);
    let state = Arc::new(ClientState {
        commands,
        updates: Mutex::new(update_receiver),
        extension_updates: Mutex::new(extension_update_receiver),
        callbacks: Mutex::new(callback_receiver),
        capabilities: Mutex::new(BTreeSet::new()),
        extensions: Mutex::new(BTreeSet::new()),
        terminal_error: Mutex::new(None),
        owner: ClientConnectionId::parse(format!("lua-client-{connection}"))
            .expect("generated client id is valid"),
        generation: CapabilityGenerationId::parse(format!("connection-{connection}"))
            .expect("generated generation id is valid"),
    });
    let worker_state = Arc::clone(&state);
    thread::Builder::new()
        .name("phenix-lua-acp".to_owned())
        .spawn(move || {
            run_client(
                config,
                receiver,
                updates,
                extension_updates,
                callbacks,
                worker_state,
            )
        })
        .map_err(|error| lua_error(BindingError::transport(error.to_string())))?;
    Ok(Client {
        state,
        local_callables: Rc::new(RefCell::new(LocalCallables::default())),
    })
}

fn run_client(
    config: StdioConfig,
    mut commands: mpsc::UnboundedReceiver<Command>,
    updates: SessionUpdates,
    extension_updates: ExtensionUpdates,
    callbacks: ExtensionCallbacks,
    state: Arc<ClientState>,
) {
    let worker_state = Arc::clone(&state);
    let result = futures::executor::block_on(
        AcpClient::new(config).connect_with_updates_extensions_and_callbacks(
            updates,
            extension_updates,
            callbacks,
            move |connection| async move {
                let negotiated_extensions = connection.negotiated_extensions();
                if let Ok(mut extensions) = worker_state.extensions.lock() {
                    extensions.extend(
                        negotiated_extensions
                            .iter()
                            .map(|extension| extension.operation.to_string()),
                    );
                }
                if let Ok(mut capabilities) = worker_state.capabilities.lock() {
                    capabilities.extend([
                        "phenix.application.capability.discovery@1".to_owned(),
                        "phenix.application.capability.sessions@1".to_owned(),
                        "phenix.application.capability.prompt@1".to_owned(),
                    ]);
                    for extension in &negotiated_extensions {
                        capabilities.insert(extension.capability.to_string());
                    }
                }

                while let Some(command) = commands.next().await {
                    match command {
                        Command::NewSession { cwd, reply } => {
                            let result = connection
                                .new_session(NewSessionRequest::new(cwd))
                                .await
                                .map(|response| Response::Session(response.session_id.to_string()))
                                .map_err(BindingError::from_client);
                            let _ = reply.send(result);
                        }
                        Command::ListSessions { cwd, cursor, reply } => {
                            let mut request = ListSessionsRequest::new();
                            if let Some(cwd) = cwd {
                                request = request.cwd(cwd);
                            }
                            if let Some(cursor) = cursor {
                                request = request.cursor(cursor);
                            }
                            let result = connection
                                .list_sessions(request)
                                .await
                                .map_err(BindingError::from_client)
                                .and_then(|response| {
                                    serde_json::to_value(response).map(Response::Json).map_err(
                                        |error| BindingError::conversion(error.to_string()),
                                    )
                                });
                            let _ = reply.send(result);
                        }
                        Command::ResumeSession {
                            session_id,
                            cwd,
                            reply,
                        } => {
                            let result = connection
                                .resume_session(ResumeSessionRequest::new(session_id.clone(), cwd))
                                .await
                                .map(|_| Response::Session(session_id))
                                .map_err(BindingError::from_client);
                            let _ = reply.send(result);
                        }
                        Command::LoadSession {
                            session_id,
                            cwd,
                            reply,
                        } => {
                            let result = connection
                                .load_session(LoadSessionRequest::new(session_id.clone(), cwd))
                                .await
                                .map(|_| Response::Session(session_id))
                                .map_err(BindingError::from_client);
                            let _ = reply.send(result);
                        }
                        Command::CloseSession { session_id, reply } => {
                            let result = connection
                                .close_session(CloseSessionRequest::new(session_id))
                                .await
                                .map(|_| Response::Acknowledged)
                                .map_err(BindingError::from_client);
                            let _ = reply.send(result);
                        }
                        Command::Prompt {
                            session_id,
                            text,
                            reply,
                        } => {
                            let request = PromptRequest::new(
                                session_id,
                                vec![ContentBlock::Text(TextContent::new(text))],
                            );
                            let result = connection
                                .prompt(request)
                                .await
                                .map(|response| {
                                    Response::PromptComplete(
                                        format!("{:?}", response.stop_reason).to_lowercase(),
                                    )
                                })
                                .map_err(BindingError::from_client);
                            let _ = reply.send(result);
                        }
                        Command::SetOption {
                            session_id,
                            config_id,
                            value,
                            reply,
                        } => {
                            let result = connection
                                .set_session_config_option(SetSessionConfigOptionRequest::new(
                                    session_id,
                                    config_id,
                                    value.as_str(),
                                ))
                                .await
                                .map_err(BindingError::from_client)
                                .and_then(|response| {
                                    serde_json::to_value(response).map(Response::Json).map_err(
                                        |error| BindingError::conversion(error.to_string()),
                                    )
                                });
                            let _ = reply.send(result);
                        }
                        Command::Application {
                            operation,
                            input,
                            reply,
                        } => {
                            let result = connection
                                .invoke_extension(&operation, input)
                                .await
                                .map(|value| Response::Application { operation, value })
                                .map_err(BindingError::from_client);
                            let _ = reply.send(result);
                        }
                        Command::InvokeCapability {
                            input,
                            output_schema,
                            reply,
                        } => {
                            let result = connection
                                .invoke_extension(
                                    &ContractId::parse(InvokeCapability::ID)
                                        .expect("static operation id"),
                                    input.to_value(),
                                )
                                .await
                                .map_err(BindingError::from_client)
                                .and_then(|value| {
                                    let result = CapabilityInvokeResult::from_value(&value)
                                        .map_err(|error| {
                                            BindingError::conversion(error.to_string())
                                        })?;
                                    output_schema.parse(&result.output).map_err(|error| {
                                        BindingError::conversion(format!(
                                            "capability output violates callable schema: {error}"
                                        ))
                                    })?;
                                    Ok(Response::Projected {
                                        schema: output_schema,
                                        value: result.output,
                                    })
                                });
                            let _ = reply.send(result);
                        }
                        Command::Cancel { session_id } => {
                            connection.cancel(CancelNotification::new(session_id));
                        }
                    }
                }
                Ok(())
            },
        ),
    );
    if let Err(error) = result {
        state.record_failure(BindingError::from_client(error));
    }
}

fn application_event_to_lua(lua: &Lua, event: ApplicationEvent) -> LuaResult<Value> {
    let result = lua.create_table()?;
    result.set("kind", "application_event")?;
    result.set("event", event.event.as_str())?;
    result.set(
        "payload",
        phenix_to_lua(lua, &event.payload).map_err(lua_error)?,
    )?;
    Ok(Value::Table(result))
}

fn dispatch_local_callback(
    lua: &Lua,
    state: &ClientState,
    local_callables: &Rc<RefCell<LocalCallables>>,
    callback: phenix_client_acp::ExtensionCallbackRequest,
) -> LuaResult<()> {
    let result = (|| -> Result<PhenixValue, BindingError> {
        let invocation = CapabilityInvokeInput::from_value(&callback.input)
            .map_err(|error| BindingError::conversion(error.to_string()))?;
        let callable = invocation
            .callable
            .callable()
            .map_err(|error| BindingError::conversion(error.to_string()))?;
        if callable.owner() != &CapabilityOwnerId::Client(state.owner.clone())
            || callable.generation() != &state.generation
        {
            return Err(BindingError::local(
                ErrorKind::Rejected,
                "callback refers to a stale or foreign client callable",
            ));
        }
        let (contract, input, output, function) = {
            let local = local_callables.borrow();
            let entry = local.entries.get(callable.id()).ok_or_else(|| {
                BindingError::local(
                    ErrorKind::Rejected,
                    "callback callable is no longer registered",
                )
            })?;
            let Type::Callable {
                contract,
                input,
                output,
            } = &entry.schema
            else {
                return Err(BindingError::conversion(
                    "registered callable schema is not callable",
                ));
            };
            let function: mlua::Function = lua
                .registry_value(&entry.function)
                .map_err(|error| BindingError::conversion(error.to_string()))?;
            (
                contract.clone(),
                (**input).clone(),
                (**output).clone(),
                function,
            )
        };
        if callable.contract() != &contract {
            return Err(BindingError::conversion(
                "callback callable contract does not match its schema",
            ));
        }
        input
            .parse(&invocation.input)
            .map_err(|error| BindingError::conversion(error.to_string()))?;
        let argument = phenix_to_lua(lua, &invocation.input)?;
        let output_value: Value = function.call(argument).map_err(|error| {
            BindingError::local(ErrorKind::Rejected, format!("Lua callable failed: {error}"))
        })?;
        lua_to_phenix_with_host(lua, &output, output_value, state, local_callables)
    })();
    match result {
        Ok(output) => {
            callback.respond(Ok(CapabilityInvokeResult { output }.to_value()));
            Ok(())
        }
        Err(error) => {
            callback.respond(Err(
                phenix_application_interface::types::ApplicationError::Failed {
                    message: error.message.clone(),
                },
            ));
            Err(lua_error(error))
        }
    }
}

fn lua_to_phenix(
    lua: &Lua,
    schema: &PhenixSchema,
    value: Value,
) -> Result<PhenixValue, BindingError> {
    match schema {
        Type::Any => lua_any_to_phenix(value),
        Type::Never => Err(BindingError::conversion(
            "Lua value cannot satisfy the never schema",
        )),
        Type::Unit => match value {
            Value::Nil => Ok(PhenixValue::Unit),
            _ => Err(type_error("nil", &value)),
        },
        Type::Bool => match value {
            Value::Boolean(value) => Ok(PhenixValue::Bool(value)),
            _ => Err(type_error("boolean", &value)),
        },
        Type::I64 => match value {
            Value::Integer(value) => Ok(PhenixValue::I64(value)),
            _ => Err(type_error("integer", &value)),
        },
        Type::U64 => match value {
            Value::Integer(value) if value >= 0 => Ok(PhenixValue::U64(value as u64)),
            _ => Err(type_error("non-negative integer", &value)),
        },
        Type::F64 => match value {
            Value::Integer(value) => Ok(PhenixValue::F64(value as f64)),
            Value::Number(value) if value.is_finite() => Ok(PhenixValue::F64(value)),
            _ => Err(type_error("finite number", &value)),
        },
        Type::String => match value {
            Value::String(value) => Ok(PhenixValue::String(value.to_string_lossy())),
            _ => Err(type_error("string", &value)),
        },
        Type::Bytes => match value {
            Value::String(value) => Ok(PhenixValue::Bytes(value.as_bytes().to_vec())),
            _ => Err(type_error("string bytes", &value)),
        },
        Type::Option(item) => match value {
            Value::Nil => Ok(PhenixValue::Option(None)),
            value => Ok(PhenixValue::Option(Some(Box::new(lua_to_phenix(
                lua, item, value,
            )?)))),
        },
        Type::Array { item, len } => {
            let Value::Table(table) = value else {
                return Err(type_error("array table", &value));
            };
            let values = lua_sequence_to_phenix(lua, table, item)?;
            if values.len() != *len {
                return Err(BindingError::conversion(format!(
                    "expected {len} array items, got {}",
                    values.len()
                )));
            }
            Ok(PhenixValue::List(values))
        }
        Type::List(item) => {
            let Value::Table(table) = value else {
                return Err(type_error("list table", &value));
            };
            Ok(PhenixValue::List(lua_sequence_to_phenix(lua, table, item)?))
        }
        Type::Map(item) => {
            let Value::Table(table) = value else {
                return Err(type_error("map table", &value));
            };
            let mut values = BTreeMap::new();
            for pair in table.pairs::<String, Value>() {
                let (key, value) =
                    pair.map_err(|error| BindingError::conversion(error.to_string()))?;
                values.insert(key, lua_to_phenix(lua, item, value)?);
            }
            Ok(PhenixValue::Map(values))
        }
        Type::Table(fields) => {
            let Value::Table(table) = value else {
                return Err(type_error("record table", &value));
            };
            Ok(PhenixValue::Table(lua_table_to_record(
                lua, table, fields, None,
            )?))
        }
        Type::Variant(variants) => {
            let Value::Table(table) = value else {
                return Err(type_error("variant table", &value));
            };
            let kind = table.get::<String>("kind").map_err(|_| {
                BindingError::conversion("variant table requires string field kind")
            })?;
            let key = Key::parse(kind.clone()).map_err(BindingError::conversion)?;
            let payload_schema = variants
                .get(kind.as_str())
                .ok_or_else(|| BindingError::conversion(format!("unknown variant kind {kind}")))?;
            let payload = match payload_schema {
                Type::Table(fields) => {
                    PhenixValue::Table(lua_table_to_record(lua, table, fields, Some("kind"))?)
                }
                Type::Unit => PhenixValue::Unit,
                schema => {
                    let value = table.get::<Value>("value").map_err(|error| {
                        BindingError::conversion(format!("variant {kind} requires value: {error}"))
                    })?;
                    lua_to_phenix(lua, schema, value)?
                }
            };
            Ok(PhenixValue::Variant {
                tag: key,
                value: Box::new(payload),
            })
        }
        Type::Callable { .. } | Type::Object { .. } => Err(BindingError::conversion(
            "callable and object references cannot be constructed from Lua tables",
        )),
    }
}

fn lua_to_phenix_with_host(
    lua: &Lua,
    schema: &PhenixSchema,
    value: Value,
    state: &ClientState,
    local_callables: &Rc<RefCell<LocalCallables>>,
) -> Result<PhenixValue, BindingError> {
    match schema {
        Type::Callable { .. } => {
            let Value::Function(function) = value else {
                return Err(type_error("function", &value));
            };
            let reference =
                local_callables
                    .borrow_mut()
                    .lift(lua, state, schema.clone(), function)?;
            Ok(PhenixValue::Callable(reference))
        }
        Type::Option(item) => match value {
            Value::Nil => Ok(PhenixValue::Option(None)),
            value => Ok(PhenixValue::Option(Some(Box::new(
                lua_to_phenix_with_host(lua, item, value, state, local_callables)?,
            )))),
        },
        Type::Array { item, len } => {
            let Value::Table(table) = value else {
                return Err(type_error("array table", &value));
            };
            let values = table
                .sequence_values::<Value>()
                .map(|value| {
                    value
                        .map_err(|error| BindingError::conversion(error.to_string()))
                        .and_then(|value| {
                            lua_to_phenix_with_host(lua, item, value, state, local_callables)
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;
            if values.len() != *len {
                return Err(BindingError::conversion(format!(
                    "expected {len} array items, got {}",
                    values.len()
                )));
            }
            Ok(PhenixValue::List(values))
        }
        Type::List(item) => {
            let Value::Table(table) = value else {
                return Err(type_error("list table", &value));
            };
            Ok(PhenixValue::List(
                table
                    .sequence_values::<Value>()
                    .map(|value| {
                        value
                            .map_err(|error| BindingError::conversion(error.to_string()))
                            .and_then(|value| {
                                lua_to_phenix_with_host(lua, item, value, state, local_callables)
                            })
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            ))
        }
        Type::Map(item) => {
            let Value::Table(table) = value else {
                return Err(type_error("map table", &value));
            };
            let mut values = BTreeMap::new();
            for pair in table.pairs::<String, Value>() {
                let (key, value) =
                    pair.map_err(|error| BindingError::conversion(error.to_string()))?;
                values.insert(
                    key,
                    lua_to_phenix_with_host(lua, item, value, state, local_callables)?,
                );
            }
            Ok(PhenixValue::Map(values))
        }
        Type::Table(fields) => {
            let Value::Table(table) = value else {
                return Err(type_error("record table", &value));
            };
            let mut values = BTreeMap::new();
            for (field, field_schema) in fields {
                let value = table
                    .get::<Value>(field.as_str())
                    .map_err(|error| BindingError::conversion(error.to_string()))?;
                values.insert(
                    field.clone(),
                    lua_to_phenix_with_host(lua, field_schema, value, state, local_callables)?,
                );
            }
            for pair in table.pairs::<Value, Value>() {
                let (key, _) = pair.map_err(|error| BindingError::conversion(error.to_string()))?;
                let Value::String(key) = key else {
                    return Err(BindingError::conversion(
                        "record tables require string field names",
                    ));
                };
                if !fields.contains_key(key.to_string_lossy().as_str()) {
                    return Err(BindingError::conversion(format!(
                        "unexpected record field {}",
                        key.to_string_lossy()
                    )));
                }
            }
            Ok(PhenixValue::Table(values))
        }
        Type::Variant(variants) => {
            let Value::Table(table) = value else {
                return Err(type_error("variant table", &value));
            };
            let kind = table.get::<String>("kind").map_err(|_| {
                BindingError::conversion("variant table requires string field kind")
            })?;
            let tag = Key::parse(kind.clone()).map_err(BindingError::conversion)?;
            let payload_schema = variants
                .get(kind.as_str())
                .ok_or_else(|| BindingError::conversion(format!("unknown variant kind {kind}")))?;
            let payload = match payload_schema {
                Type::Unit => PhenixValue::Unit,
                Type::Table(fields) => {
                    let mut values = BTreeMap::new();
                    for (field, field_schema) in fields {
                        let value = table
                            .get::<Value>(field.as_str())
                            .map_err(|error| BindingError::conversion(error.to_string()))?;
                        values.insert(
                            field.clone(),
                            lua_to_phenix_with_host(
                                lua,
                                field_schema,
                                value,
                                state,
                                local_callables,
                            )?,
                        );
                    }
                    PhenixValue::Table(values)
                }
                schema => lua_to_phenix_with_host(
                    lua,
                    schema,
                    table.get::<Value>("value").map_err(|error| {
                        BindingError::conversion(format!("variant {kind} requires value: {error}"))
                    })?,
                    state,
                    local_callables,
                )?,
            };
            Ok(PhenixValue::Variant {
                tag,
                value: Box::new(payload),
            })
        }
        _ => lua_to_phenix(lua, schema, value),
    }
}

fn lua_table_to_record(
    lua: &Lua,
    table: Table,
    fields: &BTreeMap<Key, Type>,
    allowed_extra: Option<&str>,
) -> Result<BTreeMap<Key, PhenixValue>, BindingError> {
    let mut values = BTreeMap::new();
    for (field, schema) in fields {
        let value = table
            .get::<Value>(field.as_str())
            .map_err(|error| BindingError::conversion(error.to_string()))?;
        values.insert(field.clone(), lua_to_phenix(lua, schema, value)?);
    }

    for pair in table.pairs::<Value, Value>() {
        let (key, _) = pair.map_err(|error| BindingError::conversion(error.to_string()))?;
        let Value::String(key) = key else {
            return Err(BindingError::conversion(
                "record tables require string field names",
            ));
        };
        let key = key.to_string_lossy();
        if allowed_extra == Some(key.as_str()) {
            continue;
        }
        if !fields.contains_key(key.as_str()) {
            return Err(BindingError::conversion(format!(
                "unexpected record field {key}"
            )));
        }
    }
    Ok(values)
}

fn lua_sequence_to_phenix(
    lua: &Lua,
    table: Table,
    item: &Type,
) -> Result<Vec<PhenixValue>, BindingError> {
    table
        .sequence_values::<Value>()
        .map(|value| {
            value
                .map_err(|error| BindingError::conversion(error.to_string()))
                .and_then(|value| lua_to_phenix(lua, item, value))
        })
        .collect()
}

fn lua_any_to_phenix(value: Value) -> Result<PhenixValue, BindingError> {
    match value {
        Value::Nil => Ok(PhenixValue::Unit),
        Value::Boolean(value) => Ok(PhenixValue::Bool(value)),
        Value::Integer(value) => Ok(PhenixValue::I64(value)),
        Value::Number(value) if value.is_finite() => Ok(PhenixValue::F64(value)),
        Value::String(value) => Ok(PhenixValue::String(value.to_string_lossy())),
        Value::Table(table) => {
            let raw_len = table.raw_len();
            if raw_len > 0 {
                let values = table
                    .sequence_values::<Value>()
                    .map(|value| {
                        value
                            .map_err(|error| BindingError::conversion(error.to_string()))
                            .and_then(lua_any_to_phenix)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                return Ok(PhenixValue::List(values));
            }
            let mut values = BTreeMap::new();
            for pair in table.pairs::<String, Value>() {
                let (key, value) =
                    pair.map_err(|error| BindingError::conversion(error.to_string()))?;
                values.insert(key, lua_any_to_phenix(value)?);
            }
            Ok(PhenixValue::Map(values))
        }
        _ => Err(type_error("serializable Lua value", &value)),
    }
}

fn phenix_to_lua(lua: &Lua, value: &PhenixValue) -> Result<Value, BindingError> {
    match value {
        PhenixValue::Unit => Ok(Value::Nil),
        PhenixValue::Bool(value) => Ok(Value::Boolean(*value)),
        PhenixValue::I64(value) => Ok(Value::Integer(*value)),
        PhenixValue::U64(value) if *value <= i64::MAX as u64 => Ok(Value::Integer(*value as i64)),
        PhenixValue::U64(_) => Err(BindingError::conversion(
            "u64 value exceeds Lua 5.1 integer range",
        )),
        PhenixValue::F64(value) if value.is_finite() => Ok(Value::Number(*value)),
        PhenixValue::F64(_) => Err(BindingError::conversion(
            "non-finite float cannot cross the Lua boundary",
        )),
        PhenixValue::String(value) => Ok(Value::String(
            lua.create_string(value)
                .map_err(|error| BindingError::conversion(error.to_string()))?,
        )),
        PhenixValue::Bytes(value) => Ok(Value::String(
            lua.create_string(value)
                .map_err(|error| BindingError::conversion(error.to_string()))?,
        )),
        PhenixValue::Option(None) => Ok(Value::Nil),
        PhenixValue::Option(Some(value)) => phenix_to_lua(lua, value),
        PhenixValue::List(values) => {
            let table = lua
                .create_table()
                .map_err(|error| BindingError::conversion(error.to_string()))?;
            for (index, value) in values.iter().enumerate() {
                table
                    .set(index + 1, phenix_to_lua(lua, value)?)
                    .map_err(|error| BindingError::conversion(error.to_string()))?;
            }
            Ok(Value::Table(table))
        }
        PhenixValue::Map(values) => {
            let table = lua
                .create_table()
                .map_err(|error| BindingError::conversion(error.to_string()))?;
            for (key, value) in values {
                table
                    .set(key.as_str(), phenix_to_lua(lua, value)?)
                    .map_err(|error| BindingError::conversion(error.to_string()))?;
            }
            Ok(Value::Table(table))
        }
        PhenixValue::Table(values) => {
            let table = lua
                .create_table()
                .map_err(|error| BindingError::conversion(error.to_string()))?;
            for (key, value) in values {
                table
                    .set(key.as_str(), phenix_to_lua(lua, value)?)
                    .map_err(|error| BindingError::conversion(error.to_string()))?;
            }
            Ok(Value::Table(table))
        }
        PhenixValue::Variant { tag, value } => {
            let table = lua
                .create_table()
                .map_err(|error| BindingError::conversion(error.to_string()))?;
            table
                .set("kind", tag.as_str())
                .map_err(|error| BindingError::conversion(error.to_string()))?;
            match value.as_ref() {
                PhenixValue::Unit => {}
                PhenixValue::Table(fields) => {
                    for (key, value) in fields {
                        table
                            .set(key.as_str(), phenix_to_lua(lua, value)?)
                            .map_err(|error| BindingError::conversion(error.to_string()))?;
                    }
                }
                value => {
                    table
                        .set("value", phenix_to_lua(lua, value)?)
                        .map_err(|error| BindingError::conversion(error.to_string()))?;
                }
            }
            Ok(Value::Table(table))
        }
        PhenixValue::Callable(reference) => {
            let value = serde_json::to_value(reference)
                .map_err(|error| BindingError::conversion(error.to_string()))?;
            lua.to_value(&value)
                .map_err(|error| BindingError::conversion(error.to_string()))
        }
        PhenixValue::Object(reference) => {
            let value = serde_json::to_value(reference)
                .map_err(|error| BindingError::conversion(error.to_string()))?;
            lua.to_value(&value)
                .map_err(|error| BindingError::conversion(error.to_string()))
        }
    }
}

/// Projects a value with the schema that travelled with the SDK graph.
///
/// Raw callable references deliberately do not contain their input/output
/// schemas. Structural projection may infer ordinary values, but callable
/// leaves must retain this paired schema through the Lua boundary.
fn phenix_to_lua_with_state(
    lua: &Lua,
    state: Option<&Arc<ClientState>>,
    local_callables: Option<&Rc<RefCell<LocalCallables>>>,
    schema: &PhenixSchema,
    value: &PhenixValue,
) -> Result<Value, BindingError> {
    schema.parse(value).map_err(|error| {
        BindingError::conversion(format!("value violates projection schema: {error}"))
    })?;
    match (schema, value) {
        (Type::Callable { input, output, .. }, PhenixValue::Callable(reference)) => {
            remote_callable(
                lua,
                state,
                local_callables,
                reference.clone(),
                (**input).clone(),
                (**output).clone(),
            )
        }
        (Type::Option(_), PhenixValue::Option(None)) => Ok(Value::Nil),
        (Type::Option(item), PhenixValue::Option(Some(value))) => {
            phenix_to_lua_with_state(lua, state, local_callables, item, value)
        }
        (Type::Array { item, .. } | Type::List(item), PhenixValue::List(values)) => {
            let table = lua
                .create_table()
                .map_err(|error| BindingError::conversion(error.to_string()))?;
            for (index, value) in values.iter().enumerate() {
                table
                    .set(
                        index + 1,
                        phenix_to_lua_with_state(lua, state, local_callables, item, value)?,
                    )
                    .map_err(|error| BindingError::conversion(error.to_string()))?;
            }
            Ok(Value::Table(table))
        }
        (Type::Map(item), PhenixValue::Map(values)) => {
            let table = lua
                .create_table()
                .map_err(|error| BindingError::conversion(error.to_string()))?;
            for (key, value) in values {
                table
                    .set(
                        key.as_str(),
                        phenix_to_lua_with_state(lua, state, local_callables, item, value)?,
                    )
                    .map_err(|error| BindingError::conversion(error.to_string()))?;
            }
            Ok(Value::Table(table))
        }
        (Type::Table(fields), PhenixValue::Table(values)) => {
            let table = lua
                .create_table()
                .map_err(|error| BindingError::conversion(error.to_string()))?;
            for (key, field_schema) in fields {
                let value = values
                    .get(key)
                    .expect("schema validation requires table field");
                table
                    .set(
                        key.as_str(),
                        phenix_to_lua_with_state(lua, state, local_callables, field_schema, value)?,
                    )
                    .map_err(|error| BindingError::conversion(error.to_string()))?;
            }
            Ok(Value::Table(table))
        }
        (Type::Variant(variants), PhenixValue::Variant { tag, value }) => {
            let payload_schema = variants
                .get(tag)
                .expect("schema validation requires variant payload schema");
            let table = lua
                .create_table()
                .map_err(|error| BindingError::conversion(error.to_string()))?;
            table
                .set("kind", tag.as_str())
                .map_err(|error| BindingError::conversion(error.to_string()))?;
            match (payload_schema, value.as_ref()) {
                (Type::Unit, PhenixValue::Unit) => {}
                (Type::Table(fields), PhenixValue::Table(values)) => {
                    for (key, field_schema) in fields {
                        let field = values
                            .get(key)
                            .expect("schema validation requires variant table field");
                        table
                            .set(
                                key.as_str(),
                                phenix_to_lua_with_state(
                                    lua,
                                    state,
                                    local_callables,
                                    field_schema,
                                    field,
                                )?,
                            )
                            .map_err(|error| BindingError::conversion(error.to_string()))?;
                    }
                }
                (payload_schema, value) => table
                    .set(
                        "value",
                        phenix_to_lua_with_state(
                            lua,
                            state,
                            local_callables,
                            payload_schema,
                            value,
                        )?,
                    )
                    .map_err(|error| BindingError::conversion(error.to_string()))?,
            }
            Ok(Value::Table(table))
        }
        _ => phenix_to_lua(lua, value),
    }
}

fn remote_callable(
    lua: &Lua,
    state: Option<&Arc<ClientState>>,
    local_callables: Option<&Rc<RefCell<LocalCallables>>>,
    callable: CallableRef,
    input_schema: PhenixSchema,
    output_schema: PhenixSchema,
) -> Result<Value, BindingError> {
    let state = state.ok_or_else(|| {
        BindingError::conversion("callable projection requires a live Phenix client connection")
    })?;
    let state = Arc::clone(state);
    let local_callables = local_callables.cloned();
    let proxy = lua
        .create_function(move |lua, input: Value| {
            let input = if let Some(local_callables) = &local_callables {
                lua_to_phenix_with_host(lua, &input_schema, input, &state, local_callables)
            } else {
                lua_to_phenix(lua, &input_schema, input)
            }
            .map_err(lua_error)?;
            let request = request_for(&state, local_callables.clone(), |reply| {
                Command::InvokeCapability {
                    input: CapabilityInvokeInput {
                        callable: PhenixValue::Callable(callable.clone()),
                        input,
                    },
                    output_schema: output_schema.clone(),
                    reply,
                }
            })?;
            lua.create_userdata(request)
        })
        .map_err(|error| BindingError::conversion(error.to_string()))?;
    Ok(Value::Function(proxy))
}

fn type_error(expected: &str, value: &Value) -> BindingError {
    BindingError::conversion(format!("expected {expected}, got {}", value.type_name()))
}

#[mlua::lua_module(name = "phenix")]
fn phenix(lua: &Lua) -> LuaResult<Table> {
    let exports = lua.create_table()?;
    exports.set("interface_id", INTERFACE_ID)?;
    exports.set("descriptor", descriptor(lua)?)?;
    exports.set(
        "connect",
        lua.create_function(|_lua, options: Table| connect(options))?,
    )?;
    Ok(exports)
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_client_acp::RequestRejection;
    use phenix_core::{CapabilityGenerationId, CapabilityOwnerId, ClientConnectionId, ReferenceId};

    #[test]
    fn descriptor_source_is_deterministic_and_complete() {
        let descriptor = application_descriptor();
        let first =
            phenix_binding_generator::lua(&descriptor).expect("fixed descriptor generates Lua");
        let second =
            phenix_binding_generator::lua(&descriptor).expect("fixed descriptor generates Lua");
        assert_eq!(first, second);
        assert!(first.contains("phenix.application@1"));
        assert!(first.contains("phenix.application.error@1"));
        assert!(first.contains("client:_invoke_application"));
    }

    #[test]
    fn client_errors_keep_a_lua_visible_kind() {
        let error = BindingError::from_client(ClientError::OutOfOrderUpdate {
            session_id: "session-1".to_owned(),
            expected: 1,
            received: 2,
        });
        assert_eq!(error.kind, ErrorKind::Protocol);
        assert_eq!(error.code, "protocol");
    }

    #[test]
    fn cancellation_and_rejection_keep_distinct_lua_codes() {
        let cancelled = BindingError::from_client(ClientError::Cancelled {
            message: "same display text".to_owned(),
            details: Box::new(None),
        });
        let rejected =
            BindingError::from_client(ClientError::Rejected(Box::new(RequestRejection {
                code: agent_client_protocol::ErrorCode::InternalError,
                class: Some("permission_denied".to_owned()),
                message: "same display text".to_owned(),
                details: Some(serde_json::json!({ "message": "same display text" })),
            })));

        assert_eq!(cancelled.kind, ErrorKind::Cancelled);
        assert_eq!(cancelled.code, "cancelled");
        assert_eq!(rejected.kind, ErrorKind::Rejected);
        assert_eq!(rejected.code, "permission_denied");
        assert_eq!(
            rejected
                .details
                .as_ref()
                .as_ref()
                .expect("rejection details")["message"],
            "same display text"
        );
    }

    #[test]
    fn structural_lua_conversion_rejects_unknown_record_fields() {
        let lua = Lua::new();
        let descriptor = application_descriptor();
        let operation = descriptor
            .operations
            .get(
                &ContractId::parse("phenix.application.session-rename@1")
                    .expect("static operation id"),
            )
            .expect("rename operation");
        let schema = descriptor
            .types
            .get(&operation.input)
            .expect("rename input schema");
        let input = lua.create_table().expect("input table");
        input.set("session_id", "session-1").expect("session id");
        input.set("title", "renamed").expect("title");
        input.set("extra", true).expect("extra");

        let error =
            lua_to_phenix(&lua, schema, Value::Table(input)).expect_err("unknown fields must fail");
        assert_eq!(error.kind, ErrorKind::Conversion);
        assert!(error.message.contains("unexpected record field extra"));
    }

    #[test]
    fn paired_sdk_projection_turns_a_callable_value_into_a_generic_request() {
        let lua = Lua::new();
        let (commands, mut receiver) = mpsc::unbounded();
        let (_updates_sender, updates) = std_mpsc::channel();
        let (_extension_sender, extension_updates) = std_mpsc::channel();
        let (_callback_sender, callbacks) = std_mpsc::channel();
        let state = Arc::new(ClientState {
            commands,
            updates: Mutex::new(updates),
            extension_updates: Mutex::new(extension_updates),
            callbacks: Mutex::new(callbacks),
            capabilities: Mutex::new(BTreeSet::new()),
            extensions: Mutex::new(BTreeSet::new()),
            terminal_error: Mutex::new(None),
            owner: ClientConnectionId::parse("fixture-client").unwrap(),
            generation: CapabilityGenerationId::parse("generation-1").unwrap(),
        });
        let reference = CallableRef::new(
            ContractId::parse("fixture.echo@1").unwrap(),
            CapabilityOwnerId::Client(ClientConnectionId::parse("fixture-client").unwrap()),
            CapabilityGenerationId::parse("generation-1").unwrap(),
            ReferenceId::parse("echo").unwrap(),
        );
        let schema = Type::Callable {
            contract: reference.contract().clone(),
            input: Box::new(Type::U64),
            output: Box::new(Type::String),
        };
        let Value::Function(proxy) = phenix_to_lua_with_state(
            &lua,
            Some(&state),
            None,
            &schema,
            &PhenixValue::Callable(reference.clone()),
        )
        .unwrap() else {
            panic!("callable projection must be a Lua function");
        };

        let _: mlua::AnyUserData = proxy.call(7_i64).unwrap();
        let command = futures::executor::block_on(receiver.next()).unwrap();
        let Command::InvokeCapability {
            input,
            output_schema,
            ..
        } = command
        else {
            panic!("callable proxy must use generic capability invocation");
        };
        assert_eq!(input.callable, PhenixValue::Callable(reference));
        assert_eq!(input.input, PhenixValue::U64(7));
        assert_eq!(output_schema, Type::String);
    }
}
