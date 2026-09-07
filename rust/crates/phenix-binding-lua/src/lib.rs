#![forbid(unsafe_code)]

//! LuaJIT and Lua 5.1 native entry point for the Phenix ACP client.
//!
//! The module only touches Lua from calls made by the host. ACP work runs on a
//! background thread. Lua applications poll request handles to receive results
//! and ordered updates, so the binding never invokes a host event loop itself.

use agent_client_protocol::schema::v1::{
    ContentBlock, NewSessionRequest, PromptRequest, TextContent,
};
use futures::{
    channel::{mpsc, oneshot},
    StreamExt,
};
use mlua::{
    Error as LuaError, Lua, LuaSerdeExt, MultiValue, Result as LuaResult, Table, UserData,
    UserDataMethods, Value,
};
use phenix_client_acp::{
    application_descriptor, AcpClient, ClientError, SessionUpdates, StdioConfig, INTERFACE_ID,
};
use std::{
    collections::BTreeMap,
    num::NonZeroUsize,
    path::PathBuf,
    sync::{mpsc as std_mpsc, Arc, Mutex},
    thread,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum ErrorKind {
    Transport,
    Protocol,
    UnsupportedCapability,
    QueueFull,
    Conversion,
}

impl ErrorKind {
    const fn as_str(&self) -> &'static str {
        match self {
            Self::Transport => "transport",
            Self::Protocol => "protocol",
            Self::UnsupportedCapability => "unsupported_capability",
            Self::QueueFull => "queue_full",
            Self::Conversion => "conversion",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct BindingError {
    kind: ErrorKind,
    message: String,
}

impl BindingError {
    fn transport(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Transport,
            message: message.into(),
        }
    }

    fn conversion(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Conversion,
            message: message.into(),
        }
    }

    fn from_client(error: ClientError) -> Self {
        let kind = match error {
            ClientError::Transport(_) => ErrorKind::Transport,
            ClientError::Protocol(_) | ClientError::OutOfOrderUpdate { .. } => ErrorKind::Protocol,
            ClientError::UnsupportedCapability { .. } => ErrorKind::UnsupportedCapability,
            ClientError::UpdateQueueFull => ErrorKind::QueueFull,
        };
        Self {
            kind,
            message: error.to_string(),
        }
    }
}

type CommandResult = Result<Response, BindingError>;

#[derive(Clone, Debug, Eq, PartialEq)]
enum Response {
    Session(String),
    PromptComplete(String),
}

enum Command {
    NewSession {
        cwd: PathBuf,
        reply: oneshot::Sender<CommandResult>,
    },
    Prompt {
        session_id: String,
        text: String,
        reply: oneshot::Sender<CommandResult>,
    },
    Cancel {
        session_id: String,
    },
}

struct ClientState {
    commands: mpsc::UnboundedSender<Command>,
    updates: Mutex<std_mpsc::Receiver<agent_client_protocol::schema::v1::SessionNotification>>,
    terminal_error: Mutex<Option<BindingError>>,
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
}

#[derive(Clone)]
struct Client {
    state: Arc<ClientState>,
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
}

impl Request {
    fn pending(state: Arc<ClientState>, receiver: oneshot::Receiver<CommandResult>) -> Self {
        Self {
            state,
            receiver: Some(receiver),
            result: None,
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
        methods.add_method("capabilities", |lua, _this, ()| capabilities(lua));
        methods.add_method("sessions", |lua, this, ()| {
            lua.create_userdata(Sessions {
                state: Arc::clone(&this.state),
            })
        });
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
                    lua.to_value(&value)
                }
                Err(std_mpsc::TryRecvError::Empty) => Ok(Value::Nil),
                Err(std_mpsc::TryRecvError::Disconnected) => Err(lua_error(this.state.failure())),
            }
        });
    }
}

impl UserData for Sessions {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("new", |lua, this, cwd: String| {
            let request = request_for(&this.state, |reply| Command::NewSession {
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
            let request = request_for(&this.state, |reply| Command::Prompt {
                session_id: this.id.clone(),
                text,
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
            Some(Ok(response)) => response_values(lua, &this.state, response),
            Some(Err(error)) => error_values(lua, &error),
        });
    }
}

fn request_for(
    state: &Arc<ClientState>,
    make_command: impl FnOnce(oneshot::Sender<CommandResult>) -> Command,
) -> Result<Request, LuaError> {
    let (reply, receiver) = oneshot::channel();
    state.send(make_command(reply)).map_err(lua_error)?;
    Ok(Request::pending(Arc::clone(state), receiver))
}

fn response_values(
    lua: &Lua,
    state: &Arc<ClientState>,
    response: Response,
) -> LuaResult<MultiValue> {
    let value = match response {
        Response::Session(id) => Value::UserData(lua.create_userdata(Session {
            state: Arc::clone(state),
            id,
        })?),
        Response::PromptComplete(stop_reason) => {
            let result = lua.create_table()?;
            result.set("kind", "complete")?;
            result.set("stop_reason", stop_reason)?;
            Value::Table(result)
        }
    };
    Ok(MultiValue::from_vec(vec![value]))
}

fn error_values(lua: &Lua, error: &BindingError) -> LuaResult<MultiValue> {
    let result = lua.create_table()?;
    result.set("kind", error.kind.as_str())?;
    result.set("message", error.message.as_str())?;
    Ok(MultiValue::from_vec(vec![Value::Nil, Value::Table(result)]))
}

fn lua_error(error: BindingError) -> LuaError {
    LuaError::RuntimeError(format!("{}: {}", error.kind.as_str(), error.message))
}

fn capabilities(lua: &Lua) -> LuaResult<Table> {
    let descriptor = application_descriptor();
    let result = lua.create_table()?;
    for capability in descriptor.capabilities.keys() {
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
    let state = Arc::new(ClientState {
        commands,
        updates: Mutex::new(update_receiver),
        terminal_error: Mutex::new(None),
    });
    let worker_state = Arc::clone(&state);
    thread::Builder::new()
        .name("phenix-lua-acp".to_owned())
        .spawn(move || run_client(config, receiver, updates, worker_state))
        .map_err(|error| lua_error(BindingError::transport(error.to_string())))?;
    Ok(Client { state })
}

fn run_client(
    config: StdioConfig,
    mut commands: mpsc::UnboundedReceiver<Command>,
    updates: SessionUpdates,
    state: Arc<ClientState>,
) {
    let result = futures::executor::block_on(AcpClient::new(config).connect_with_updates(
        updates,
        move |connection| async move {
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
                    Command::Cancel { session_id } => {
                        connection.cancel(
                            agent_client_protocol::schema::v1::CancelNotification::new(session_id),
                        );
                    }
                }
            }
            Ok(())
        },
    ));
    if let Err(error) = result {
        state.record_failure(BindingError::from_client(error));
    }
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
    }

    #[test]
    fn client_errors_keep_a_lua_visible_kind() {
        let error = BindingError::from_client(ClientError::OutOfOrderUpdate {
            session_id: "session-1".to_owned(),
            expected: 1,
            received: 2,
        });
        assert_eq!(error.kind, ErrorKind::Protocol);
    }
}
