use crate::{
    CompiledConfiguration, ConductorError, ConductorRuntime, DomainEvent, ExecutionPayload,
    ExecutionProvider, ExecutionProviderError, ExecutionProviderEvent, ExecutionProviderHost,
    ExecutionProviderKind, ObjectiveError, PersistenceError, PlanError, SqliteStore,
};
use phenix_backend::{
    Backend, BackendError, BackendEvent, BackendHost, BackendSession, ToolInvocation, ToolResult,
};
use phenix_core::{
    AuthenticationInput, AuthenticationMethodId, BackendCatalog, BackendId, CallableId,
    ExecutionEventKind, ExecutionId, ExecutionState, ExecutionTarget, FileVersion, SessionId,
    SessionState, WorkspaceDescriptor, WorkspaceId, WorkspaceLeaseMode,
};
use phenix_protocol::{
    ClientMessage, Command, ErrorCode, ProtocolError, Reply, ResponsePayload, ServerMessage,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::{
    mpsc::{self, SyncSender},
    Arc, Condvar, Mutex, MutexGuard,
};
use std::thread;

#[path = "semantic_tools.rs"]
mod semantic_tools;
#[path = "workspace_consistency.rs"]
mod workspace_consistency;
#[path = "workspace_lease.rs"]
mod workspace_lease;
#[path = "workspace_tools.rs"]
mod workspace_tools;

use workspace_consistency::{WorkspaceConsistency, WorkspaceConsistencyError};
use workspace_lease::{WorkspaceLeaseError, WorkspaceLeaseManager};

const EVENT_BUFFER: usize = 256;
const OUTPUT_BUFFER: usize = 256;
const EXECUTION_WORKERS: usize = 4;
const IN_MEMORY_WORKSPACE_ID: &str = "workspace:in-memory";

type SharedBackend = Arc<Mutex<Box<dyn Backend>>>;
type SharedRuntime = Arc<Mutex<ConductorRuntime>>;
type ActiveScopes = Arc<Mutex<BTreeMap<ExecutionId, LiveExecutionScope>>>;
type WorkspacePhases = Arc<Mutex<BTreeMap<ExecutionId, WorkspacePhase>>>;
type RootAcceptedHook<'a> = &'a mut dyn FnMut(&ExecutionId) -> Result<(), ServerError>;

#[derive(Clone)]
struct ExecutionWorkerContext {
    runtime: SharedRuntime,
    backends: BTreeMap<BackendId, SharedBackend>,
    active_scopes: ActiveScopes,
    workspace_leases: WorkspaceLeaseManager,
    workspace_phases: WorkspacePhases,
    workspace_consistency: Option<WorkspaceConsistency>,
    store: Option<SqliteStore>,
    persist_lock: Arc<Mutex<()>>,
}

struct ExecutionJob {
    execution_id: ExecutionId,
    session_id: SessionId,
    group_id: ExecutionId,
}

#[derive(Default)]
struct WorkspacePhase {
    writing: bool,
}

impl WorkspacePhase {
    fn enter(&mut self, mode: WorkspaceLeaseMode) -> bool {
        match mode {
            WorkspaceLeaseMode::Read => {
                self.writing = false;
                false
            }
            WorkspaceLeaseMode::Write => {
                let starts_write_phase = !self.writing;
                self.writing = true;
                starts_write_phase
            }
        }
    }
}

#[derive(Default)]
struct ExecutionQueueState {
    pending: VecDeque<ExecutionJob>,
    active_sessions: BTreeMap<SessionId, ExecutionId>,
    active_groups: BTreeMap<ExecutionId, usize>,
    scheduled: BTreeSet<ExecutionId>,
    releasing_groups: BTreeSet<ExecutionId>,
    closed: bool,
}

#[derive(Clone, Default)]
struct ExecutionQueue {
    state: Arc<(Mutex<ExecutionQueueState>, Condvar)>,
}

impl ExecutionQueue {
    fn enqueue(&self, job: ExecutionJob) -> Result<(), ServerError> {
        let (lock, ready) = &*self.state;
        let mut state = lock
            .lock()
            .map_err(|_| ServerError::StatePoisoned("execution queue"))?;
        if !state.scheduled.insert(job.execution_id.clone()) {
            return Ok(());
        }
        state.pending.push_back(job);
        ready.notify_all();
        Ok(())
    }

    fn next(&self) -> Result<Option<ExecutionJob>, ServerError> {
        let (lock, ready) = &*self.state;
        let mut state = lock
            .lock()
            .map_err(|_| ServerError::StatePoisoned("execution queue"))?;
        loop {
            if let Some(index) = state.pending.iter().position(|job| {
                state
                    .active_sessions
                    .get(&job.session_id)
                    .is_none_or(|group| group == &job.group_id)
            }) {
                let job = state
                    .pending
                    .remove(index)
                    .expect("pending execution index was selected");
                state
                    .active_sessions
                    .entry(job.session_id.clone())
                    .or_insert_with(|| job.group_id.clone());
                *state.active_groups.entry(job.group_id.clone()).or_default() += 1;
                return Ok(Some(job));
            }
            if state.closed && state.pending.is_empty() && state.active_groups.is_empty() {
                return Ok(None);
            }
            state = ready
                .wait(state)
                .map_err(|_| ServerError::StatePoisoned("execution queue"))?;
        }
    }

    fn complete(&self, job: &ExecutionJob, release_group: bool) -> Result<bool, ServerError> {
        let (lock, ready) = &*self.state;
        let mut state = lock
            .lock()
            .map_err(|_| ServerError::StatePoisoned("execution queue"))?;
        if release_group {
            state.releasing_groups.insert(job.group_id.clone());
        }
        let remove_group = {
            let active = state
                .active_groups
                .get_mut(&job.group_id)
                .expect("completed execution belongs to an active group");
            *active -= 1;
            *active == 0
        };
        if remove_group {
            state.active_groups.remove(&job.group_id);
        }
        let group_released = state.releasing_groups.contains(&job.group_id)
            && !state.active_groups.contains_key(&job.group_id)
            && !state
                .pending
                .iter()
                .any(|pending| pending.group_id == job.group_id)
            && state.active_sessions.get(&job.session_id) == Some(&job.group_id);
        if group_released {
            state.active_sessions.remove(&job.session_id);
            state.releasing_groups.remove(&job.group_id);
        }
        ready.notify_all();
        Ok(group_released)
    }

    fn close(&self) -> Result<(), ServerError> {
        let (lock, ready) = &*self.state;
        let mut state = lock
            .lock()
            .map_err(|_| ServerError::StatePoisoned("execution queue"))?;
        state.closed = true;
        ready.notify_all();
        Ok(())
    }
}

/// Process-local resources owned by one live execution. Durable execution state
/// remains in `ConductorRuntime`; this scope is deliberately not persisted.
#[derive(Clone)]
enum LiveExecutionScope {
    Backend(Arc<dyn BackendSession>),
    Provider(Arc<dyn ExecutionProvider>),
}

impl LiveExecutionScope {
    fn cancel(&self, execution_id: &ExecutionId) -> Result<(), ProtocolError> {
        match self {
            Self::Backend(session) => match session.cancel(execution_id) {
                Ok(()) | Err(BackendError::Unsupported(_)) => Ok(()),
                Err(error) => Err(map_backend_error(error)),
            },
            Self::Provider(provider) => match provider.cancel(execution_id) {
                Ok(()) | Err(ExecutionProviderError::Unsupported(_)) => Ok(()),
                Err(error) => Err(map_execution_provider_error(error)),
            },
        }
    }
}

/// RAII lease for one live execution scope. Once installed, every return path
/// from the worker tears the process-local scope down, including error returns
/// and unwinding. Durable execution state remains owned by `ConductorRuntime`.
struct LiveExecutionLease {
    scopes: ActiveScopes,
    execution_id: ExecutionId,
}

impl Drop for LiveExecutionLease {
    fn drop(&mut self) {
        if let Ok(mut scopes) = self.scopes.lock() {
            scopes.remove(&self.execution_id);
        }
    }
}

pub struct ConductorServer {
    runtime: SharedRuntime,
    backends: BTreeMap<BackendId, SharedBackend>,
    catalogs: BTreeMap<BackendId, BackendCatalog>,
    active_scopes: ActiveScopes,
    workspace_leases: WorkspaceLeaseManager,
    workspace_consistency: Option<WorkspaceConsistency>,
    store: Option<SqliteStore>,
    persist_lock: Arc<Mutex<()>>,
}

#[derive(Clone)]
pub struct ConductorService {
    inner: Arc<ConductorServiceInner>,
}

struct ConductorServiceInner {
    server: Mutex<ConductorServer>,
    executions: ExecutionQueue,
    workers: Mutex<Vec<thread::JoinHandle<Result<(), ServerError>>>>,
}

impl Drop for ConductorServiceInner {
    fn drop(&mut self) {
        let _ = self.executions.close();
        if let Ok(workers) = self.workers.get_mut() {
            for worker in workers.drain(..) {
                let _ = worker.join();
            }
        }
    }
}

impl ConductorService {
    pub fn new(server: ConductorServer) -> Result<Self, ServerError> {
        let executions = ExecutionQueue::default();
        for root in server.lock_runtime()?.pending_roots_in_ingress_order() {
            executions.enqueue(ExecutionJob {
                execution_id: root.id.clone(),
                session_id: root.session_id,
                group_id: root.id,
            })?;
        }
        let context = server.worker_context();
        let workers = (0..EXECUTION_WORKERS)
            .map(|_| {
                let executions = executions.clone();
                let context = context.clone();
                thread::spawn(move || execution_loop(executions, context))
            })
            .collect();
        Ok(Self {
            inner: Arc::new(ConductorServiceInner {
                server: Mutex::new(server),
                executions,
                workers: Mutex::new(workers),
            }),
        })
    }

    pub fn serve_connection<R, W>(&self, input: R, output: W) -> Result<(), ServerError>
    where
        R: BufRead,
        W: Write + Send,
    {
        let mut on_root = |_: &ExecutionId| Ok(());
        self.serve_connection_with_root_hook(input, output, &mut on_root)
    }

    pub(super) fn serve_connection_with_root_hook<R, W>(
        &self,
        input: R,
        output: W,
        on_root: RootAcceptedHook<'_>,
    ) -> Result<(), ServerError>
    where
        R: BufRead,
        W: Write + Send,
    {
        let (subscription, event_receiver) = {
            let server = self
                .inner
                .server
                .lock()
                .map_err(|_| ServerError::StatePoisoned("conductor service"))?;
            let subscription = server
                .runtime
                .lock()
                .map_err(|_| ServerError::StatePoisoned("conductor runtime"))?
                .subscribe_events_with_id(EVENT_BUFFER);
            subscription
        };
        let (output_sender, output_receiver) = mpsc::sync_channel(OUTPUT_BUFFER);

        thread::scope(|scope| {
            let writer = scope.spawn(move || -> Result<(), ServerError> {
                let mut output = output;
                while let Ok(message) = output_receiver.recv() {
                    serde_json::to_writer(&mut output, &message)?;
                    output.write_all(b"\n")?;
                    output.flush()?;
                }
                Ok(())
            });
            let event_output = output_sender.clone();
            let relay = scope.spawn(move || {
                while let Ok(event) = event_receiver.recv() {
                    if event_output.send(ServerMessage::Event { event }).is_err() {
                        break;
                    }
                }
            });

            let result = (|| {
                for line in input.lines() {
                    let line = line?;
                    if line.trim().is_empty() {
                        continue;
                    }
                    let mut server = self
                        .inner
                        .server
                        .lock()
                        .map_err(|_| ServerError::StatePoisoned("conductor service"))?;
                    match serde_json::from_str::<ClientMessage>(&line) {
                        Ok(message) => server.handle_message(
                            message,
                            &output_sender,
                            &self.inner.executions,
                            on_root,
                        )?,
                        Err(error) => server.respond(
                            &output_sender,
                            0,
                            Err(protocol_error(
                                ErrorCode::InvalidRequest,
                                format!("invalid client message: {error}"),
                            )),
                        )?,
                    }
                }
                Ok(())
            })();

            if let Ok(server) = self.inner.server.lock() {
                if let Ok(mut runtime) = server.runtime.lock() {
                    runtime.unsubscribe_event_subscription(subscription);
                }
            }
            drop(output_sender);
            relay.join().map_err(|_| ServerError::WorkerPanicked)?;
            let writer_result = writer.join().map_err(|_| ServerError::WorkerPanicked)?;
            normal_frontend_disconnect(result).and(normal_frontend_disconnect(writer_result))
        })
    }

    pub(super) fn execution_group_id_for(
        &self,
        execution_id: &ExecutionId,
    ) -> Result<ExecutionId, ProtocolError> {
        let server = self.inner.server.lock().map_err(|_| {
            protocol_error(
                ErrorCode::BackendProtocol,
                "conductor service lock poisoned",
            )
        })?;
        server.execution_group_id_for(execution_id)
    }
}

include!("server_base/backends.rs");
include!("server_base/lifecycle.rs");
include!("server_base/persistence.rs");
include!("server_base/protocol.rs");
include!("server_base/scheduling.rs");
include!("server_base/setup.rs");
include!("server_base/support.rs");
include!("server_base/workspace.rs");

struct SharedRuntimeHost {
    runtime: SharedRuntime,
    execution_id: ExecutionId,
    allowed_tools: BTreeSet<CallableId>,
    workspace_id: WorkspaceId,
    workspace_leases: WorkspaceLeaseManager,
    workspace_consistency: Option<WorkspaceConsistency>,
    store: Option<SqliteStore>,
    persist_lock: Arc<Mutex<()>>,
}

impl SharedRuntimeHost {
    fn persist(&self) -> Result<(), BackendError> {
        persist_shared(&self.runtime, self.store.as_ref(), &self.persist_lock).map_err(|error| {
            BackendError::Transport(format!("failed to persist conductor state: {error}"))
        })
    }

    fn lock_runtime(&self) -> Result<MutexGuard<'_, ConductorRuntime>, BackendError> {
        self.runtime
            .lock()
            .map_err(|_| BackendError::Protocol("conductor runtime lock poisoned".to_owned()))
    }
}

impl BackendHost for SharedRuntimeHost {
    fn emit(&mut self, event: BackendEvent) -> Result<(), BackendError> {
        {
            let mut runtime = self.lock_runtime()?;
            if runtime.execution_state(&self.execution_id) != Some(ExecutionState::Running) {
                return Err(BackendError::Protocol(format!(
                    "backend emitted an event after execution {} became terminal",
                    self.execution_id
                )));
            }
            let kind = match event {
                BackendEvent::ContentDelta(text) => {
                    ExecutionEventKind::AssistantContentDelta { text }
                }
                BackendEvent::ReasoningDelta(text) => ExecutionEventKind::ReasoningDelta { text },
            };
            runtime
                .push_event(&self.execution_id, kind)
                .map_err(|error| BackendError::Protocol(error.to_string()))?;
        }
        self.persist()
    }

    fn invoke_tool(&mut self, invocation: ToolInvocation) -> Result<ToolResult, BackendError> {
        let workspace_checkpoint = if invocation.callable.as_str()
            == semantic_tools::WORKSPACE_CHECKPOINT_ID
            && self.allowed_tools.contains(&invocation.callable)
        {
            let holds_write = self
                .workspace_leases
                .holds_write(&self.workspace_id, &self.execution_id)
                .map_err(|error| BackendError::Protocol(error.to_string()))?;
            if !holds_write {
                None
            } else {
                let files = self
                    .workspace_consistency
                    .as_ref()
                    .ok_or_else(|| {
                        BackendError::Protocol("workspace consistency is not installed".to_owned())
                    })?
                    .checkpoint_baseline()
                    .map_err(|error| BackendError::Protocol(error.to_string()))?;
                Some((self.workspace_id.clone(), files))
            }
        } else {
            None
        };
        let result = {
            let mut runtime = self.lock_runtime()?;
            if runtime.execution_state(&self.execution_id) != Some(ExecutionState::Running) {
                return Err(BackendError::Protocol(format!(
                    "backend invoked a tool after execution {} became terminal",
                    self.execution_id
                )));
            }
            if semantic_tools::is_semantic_tool(&invocation.callable) {
                semantic_tools::invoke(
                    &mut runtime,
                    &self.execution_id,
                    &self.allowed_tools,
                    invocation,
                    workspace_checkpoint,
                )?
            } else {
                runtime.invoke_tool(&self.execution_id, &self.allowed_tools, invocation)?
            }
        };
        self.persist()?;
        Ok(result)
    }
}

struct SharedProviderHost {
    runtime: SharedRuntime,
    execution_id: ExecutionId,
    store: Option<SqliteStore>,
    persist_lock: Arc<Mutex<()>>,
}

impl SharedProviderHost {
    fn persist(&self) -> Result<(), ExecutionProviderError> {
        persist_shared(&self.runtime, self.store.as_ref(), &self.persist_lock).map_err(|error| {
            ExecutionProviderError::Failed(format!("failed to persist conductor state: {error}"))
        })
    }

    fn lock_runtime(&self) -> Result<MutexGuard<'_, ConductorRuntime>, ExecutionProviderError> {
        self.runtime.lock().map_err(|_| {
            ExecutionProviderError::Protocol("conductor runtime lock poisoned".to_owned())
        })
    }
}

impl ExecutionProviderHost for SharedProviderHost {
    fn emit(&mut self, event: ExecutionProviderEvent) -> Result<(), ExecutionProviderError> {
        {
            let mut runtime = self.lock_runtime()?;
            if runtime.execution_state(&self.execution_id) != Some(ExecutionState::Running) {
                return Err(ExecutionProviderError::Protocol(format!(
                    "provider emitted an event after execution {} became terminal",
                    self.execution_id
                )));
            }
            let kind = match event {
                ExecutionProviderEvent::ContentDelta(text) => {
                    ExecutionEventKind::AssistantContentDelta { text }
                }
                ExecutionProviderEvent::ReasoningDelta(text) => {
                    ExecutionEventKind::ReasoningDelta { text }
                }
            };
            runtime
                .push_event(&self.execution_id, kind)
                .map_err(|error| ExecutionProviderError::Protocol(error.to_string()))?;
        }
        self.persist()
    }
}

impl ConductorRuntime {
    fn rename_session(
        &mut self,
        session_id: &SessionId,
        name: String,
    ) -> Result<phenix_core::SessionSummary, ConductorError> {
        self.ensure_session_active(session_id)?;
        self.record_domain_event(DomainEvent::SessionRenamed {
            session_id: session_id.clone(),
            name,
        })?;
        Ok(self
            .sessions
            .get(session_id)
            .expect("renamed session remains present")
            .summary
            .clone())
    }

    fn set_session_target(
        &mut self,
        session_id: &SessionId,
        target: ExecutionTarget,
    ) -> Result<phenix_core::SessionSummary, ConductorError> {
        self.ensure_session_active(session_id)?;
        self.record_domain_event(DomainEvent::SessionTargetChanged {
            session_id: session_id.clone(),
            target,
        })?;
        Ok(self
            .sessions
            .get(session_id)
            .expect("retargeted session remains present")
            .summary
            .clone())
    }

    fn record_workspace_checkpoint(
        &mut self,
        execution_id: &ExecutionId,
        workspace_id: WorkspaceId,
        files: BTreeMap<PathBuf, FileVersion>,
    ) -> Result<(), ConductorError> {
        self.record_domain_event(DomainEvent::WorkspaceCheckpointCaptured {
            execution_id: execution_id.clone(),
            workspace_id,
            files,
        })
    }

    fn interrupt_non_resumable_executions(&mut self) -> Result<(), ConductorError> {
        let running_invocations = self
            .executions
            .iter()
            .filter(|(_, record)| {
                record.summary.state == ExecutionState::Running
                    && matches!(record.payload, ExecutionPayload::Invocation { .. })
            })
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        for execution_id in running_invocations {
            self.set_state(&execution_id, ExecutionState::Interrupted)?;
        }
        Ok(())
    }

    fn execution_state(&self, execution_id: &ExecutionId) -> Option<ExecutionState> {
        self.executions
            .get(execution_id)
            .map(|record| record.summary.state.clone())
    }
}

#[derive(Debug)]
pub enum ServerError {
    Io(io::Error),
    Json(serde_json::Error),
    Persistence(PersistenceError),
    Runtime(ConductorError),
    WorkspaceConsistency(WorkspaceConsistencyError),
    WorkspaceConsistencyNotInstalled,
    WorkspaceConsistencyUnavailable(WorkspaceId),
    WorkspaceLeaseStatePoisoned,
    DuplicateBackend(BackendId),
    OutputClosed,
    StatePoisoned(&'static str),
    WorkerPanicked,
}

impl Display for ServerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => Display::fmt(error, f),
            Self::Json(error) => Display::fmt(error, f),
            Self::Persistence(error) => Display::fmt(error, f),
            Self::Runtime(error) => Display::fmt(error, f),
            Self::WorkspaceConsistency(error) => Display::fmt(error, f),
            Self::WorkspaceConsistencyNotInstalled => {
                f.write_str("workspace consistency is not installed")
            }
            Self::WorkspaceConsistencyUnavailable(workspace_id) => write!(
                f,
                "workspace consistency is not installed for writable workspace {workspace_id}"
            ),
            Self::WorkspaceLeaseStatePoisoned => f.write_str("workspace lease state lock poisoned"),
            Self::DuplicateBackend(id) => write!(f, "backend already registered: {id}"),
            Self::OutputClosed => f.write_str("frontend output channel closed"),
            Self::StatePoisoned(name) => write!(f, "{name} lock poisoned"),
            Self::WorkerPanicked => f.write_str("frontend server worker panicked"),
        }
    }
}

impl Error for ServerError {}

impl From<io::Error> for ServerError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for ServerError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

impl From<PersistenceError> for ServerError {
    fn from(value: PersistenceError) -> Self {
        Self::Persistence(value)
    }
}

impl From<ConductorError> for ServerError {
    fn from(value: ConductorError) -> Self {
        Self::Runtime(value)
    }
}

impl From<WorkspaceConsistencyError> for ServerError {
    fn from(value: WorkspaceConsistencyError) -> Self {
        Self::WorkspaceConsistency(value)
    }
}

impl From<WorkspaceLeaseError> for ServerError {
    fn from(_: WorkspaceLeaseError) -> Self {
        Self::WorkspaceLeaseStatePoisoned
    }
}

include!("server_base/helpers/frontend.rs");
include!("server_base/helpers/protocol_errors.rs");
include!("server_base/helpers/support.rs");
include!("server_base/helpers/workers.rs");
include!("server_base/helpers/workers_2.rs");
include!("server_base/helpers/workspace.rs");

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_backend::{BackendExecutionRequest, BackendSessionRequest, ToolPresentation};
    use phenix_core::{
        AgentDefinition, AuthenticationState, CallableDescriptor, CallableKind, CallablePolicy,
        CapabilitySet, ExecutionAuthority, FilesystemAuthority, InferenceOptions, ModelDescriptor,
        ModelId, ModelTarget, OrchestrationDefinition, OrchestrationNode, OrchestrationNodeId,
        ProviderId,
    };
    use rusqlite::params;
    use serde_json::json;
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{mpsc, Condvar};
    use std::time::Duration;

    struct CancelOnlySession {
        calls: Arc<AtomicUsize>,
    }

    struct ImmediateBackend;

    impl Backend for ImmediateBackend {
        fn capabilities(&self) -> phenix_backend::BackendCapabilities {
            phenix_backend::BackendCapabilities {
                tool_presentations: BTreeSet::new(),
                images: false,
                persistent_sessions: false,
            }
        }

        fn catalog(&mut self) -> Result<BackendCatalog, BackendError> {
            Ok(BackendCatalog {
                backend: BackendId::parse("fixture").unwrap(),
                models: vec![ModelDescriptor {
                    target: model_target(),
                    name: "Fixture".to_owned(),
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
            Ok(Arc::new(CancelOnlySession {
                calls: Arc::new(AtomicUsize::new(0)),
            }))
        }
    }

    impl BackendSession for CancelOnlySession {
        fn execute(
            &self,
            _request: BackendExecutionRequest,
            _host: &mut dyn BackendHost,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn cancel(&self, _execution_id: &ExecutionId) -> Result<(), BackendError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
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
            description: "server cancellation fixture".to_owned(),
            input_schema: json!({"type": "object"}),
            output_schema: json!({"type": "object"}),
            capabilities: CapabilitySet::default(),
            policy: CallablePolicy::default(),
        }
    }

    fn node(id: &str, callable: &str, dependencies: &[&str]) -> OrchestrationNode {
        OrchestrationNode {
            input_bindings: Default::default(),
            id: OrchestrationNodeId::parse(id).unwrap(),
            callable: CallableId::parse(callable).unwrap(),
            depends_on: dependencies
                .iter()
                .map(|dependency| OrchestrationNodeId::parse(*dependency).unwrap())
                .collect(),
            objective: None,
        }
    }

    fn job(execution: &str, session: &SessionId, group: &str) -> ExecutionJob {
        ExecutionJob {
            execution_id: ExecutionId::parse(execution).unwrap(),
            session_id: session.clone(),
            group_id: ExecutionId::parse(group).unwrap(),
        }
    }

    fn temporary_database() -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "phenix-conductor-multi-client-{}-{unique}.sqlite3",
            std::process::id()
        ))
    }

    fn connection_request(service: ConductorService, message: ClientMessage) -> Reply {
        let id = message.id;
        let (mut client, server) = UnixStream::pair().unwrap();
        let writer = server.try_clone().unwrap();
        let connection =
            thread::spawn(move || service.serve_connection(BufReader::new(server), writer));
        serde_json::to_writer(&mut client, &message).unwrap();
        client.write_all(b"\n").unwrap();
        client.flush().unwrap();

        let mut reader = BufReader::new(client.try_clone().unwrap());
        loop {
            let mut line = String::new();
            assert!(reader.read_line(&mut line).unwrap() > 0);
            match serde_json::from_str::<ServerMessage>(&line).unwrap() {
                ServerMessage::Response {
                    id: response_id,
                    response: ResponsePayload::Ok { result },
                } if response_id == id => {
                    drop(reader);
                    drop(client);
                    connection.join().unwrap().unwrap();
                    return result;
                }
                ServerMessage::Response {
                    id: response_id,
                    response: ResponsePayload::Error { error },
                } if response_id == id => panic!("request failed: {error:?}"),
                _ => {}
            }
        }
    }

    #[derive(Clone)]
    struct ConcurrentGate {
        state: Arc<(Mutex<usize>, Condvar)>,
    }

    struct ConcurrentBackend {
        gate: ConcurrentGate,
    }

    struct ConcurrentSession {
        gate: ConcurrentGate,
    }

    impl Backend for ConcurrentBackend {
        fn capabilities(&self) -> phenix_backend::BackendCapabilities {
            phenix_backend::BackendCapabilities {
                tool_presentations: BTreeSet::from([ToolPresentation::Native]),
                images: false,
                persistent_sessions: false,
            }
        }

        fn open_session(
            &mut self,
            _request: BackendSessionRequest,
        ) -> Result<Arc<dyn BackendSession>, BackendError> {
            Ok(Arc::new(ConcurrentSession {
                gate: self.gate.clone(),
            }))
        }
    }

    impl BackendSession for ConcurrentSession {
        fn execute(
            &self,
            _request: BackendExecutionRequest,
            _host: &mut dyn BackendHost,
        ) -> Result<(), BackendError> {
            let (lock, ready) = &*self.gate.state;
            let mut active = lock.lock().unwrap();
            *active += 1;
            ready.notify_all();
            let (active, _) = ready
                .wait_timeout_while(active, Duration::from_secs(10), |active| *active < 2)
                .unwrap();
            if *active < 2 {
                return Err(BackendError::Transport(
                    "executions did not execute concurrently".to_owned(),
                ));
            }
            Ok(())
        }

        fn cancel(&self, _execution_id: &ExecutionId) -> Result<(), BackendError> {
            Ok(())
        }
    }

    include!("server_base/tests/backends.rs");
    include!("server_base/tests/lifecycle.rs");
    include!("server_base/tests/persistence.rs");
    include!("server_base/tests/support.rs");
    include!("server_base/tests/workspace.rs");
}
