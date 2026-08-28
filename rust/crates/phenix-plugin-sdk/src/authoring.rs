use crate::{
    SdkConfigInterface, SdkSessionCommand, SdkSessionInterface, SdkSessionResponse,
    SdkSkillsInterface, SdkToolsInterface,
};
use phenix_core::{
    Authority, ComponentId, ComponentInterface, ComponentInvocationError, GraphGenerationId,
    PluginHost, PluginId,
};
use phenix_plugin_context::ContextInterface;
use phenix_plugin_models::ModelRoutingInterface;
use phenix_plugin_options::OptionsInterface;
use phenix_plugin_sessions::{SessionCommand, SessionInterface, SessionRecord, SessionResponse};
use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    marker::PhantomData,
};

/// Runtime view passed to plugin code.
///
/// The context separates the kernel, userspace SDK, current plugin data, and
/// current call data so ownership stays explicit at the call site.
pub struct PluginContext<'host, 'runtime, Sdk, Settings = (), State = ()> {
    pub kernel: KernelAccess<'host, 'runtime>,
    pub sdk: Sdk,
    pub plugin: CurrentPlugin<'host, Settings, State>,
    pub call: CallContext<'host>,
}

impl<'host, 'runtime, Sdk, Settings, State>
    PluginContext<'host, 'runtime, Sdk, Settings, State>
{
    pub fn new(
        host: &'host PluginHost<'runtime>,
        sdk: Sdk,
        settings: Settings,
        state: State,
    ) -> Self {
        Self {
            kernel: KernelAccess::new(host),
            sdk,
            plugin: CurrentPlugin {
                id: host.plugin(),
                settings,
                state,
            },
            call: CallContext {
                authority: host.authority(),
                graph_generation: host.graph_generation(),
            },
        }
    }
}

pub type PhenixPluginContext<'host, 'runtime, Settings = (), State = ()> =
    PluginContext<'host, 'runtime, PhenixSdk<'host, 'runtime>, Settings, State>;

impl<'host, 'runtime, Settings, State>
    PluginContext<'host, 'runtime, PhenixSdk<'host, 'runtime>, Settings, State>
{
    pub fn phenix(
        host: &'host PluginHost<'runtime>,
        component: ComponentId,
        settings: Settings,
        state: State,
    ) -> Self {
        Self::new(host, PhenixSdk::new(host, component), settings, state)
    }
}

/// Data owned by the current plugin instance.
pub struct CurrentPlugin<'host, Settings, State> {
    pub id: &'host PluginId,
    pub settings: Settings,
    pub state: State,
}

/// Data scoped to the current kernel-mediated call.
pub struct CallContext<'host> {
    pub authority: &'host Authority,
    pub graph_generation: Option<&'host GraphGenerationId>,
}

/// Scoped access to generic kernel mechanisms.
///
/// This intentionally wraps `PluginHost` rather than exposing a mutable kernel.
#[derive(Clone, Copy)]
pub struct KernelAccess<'host, 'runtime> {
    host: &'host PluginHost<'runtime>,
}

impl<'host, 'runtime> KernelAccess<'host, 'runtime> {
    fn new(host: &'host PluginHost<'runtime>) -> Self {
        Self { host }
    }

    pub fn authority(&self) -> &Authority {
        self.host.authority()
    }

    pub fn graph_generation(&self) -> Option<&GraphGenerationId> {
        self.host.graph_generation()
    }

    pub fn invoke<I: ComponentInterface>(
        &self,
        component: &ComponentId,
        request: &I::Request,
    ) -> Result<I::Response, ComponentInvocationError> {
        self.host.invoke_import::<I>(component, request)
    }
}

/// Marker implemented by a typed SDK contract supplied by another plugin.
pub trait SdkContract {
    type Interface: ComponentInterface;
}

/// Kernel-mediated client for one typed SDK interface.
pub struct SdkClient<'host, 'runtime, I: ComponentInterface> {
    host: &'host PluginHost<'runtime>,
    component: ComponentId,
    interface: PhantomData<fn() -> I>,
}

impl<'host, 'runtime, I: ComponentInterface> Clone for SdkClient<'host, 'runtime, I> {
    fn clone(&self) -> Self {
        Self {
            host: self.host,
            component: self.component.clone(),
            interface: PhantomData,
        }
    }
}

impl<'host, 'runtime, I: ComponentInterface> SdkClient<'host, 'runtime, I> {
    fn new(host: &'host PluginHost<'runtime>, component: ComponentId) -> Self {
        Self {
            host,
            component,
            interface: PhantomData,
        }
    }

    pub fn component(&self) -> &ComponentId {
        &self.component
    }

    pub fn invoke(&self, request: &I::Request) -> Result<I::Response, ComponentInvocationError> {
        self.host.invoke_import::<I>(&self.component, request)
    }
}

/// Consumer-side handle for a provider-owned SDK object.
///
/// The handle carries stable identity plus a scoped typed client. It never
/// contains a reference to provider-internal state.
pub struct SdkObject<'host, 'runtime, I: ComponentInterface, Id = String> {
    id: Id,
    client: SdkClient<'host, 'runtime, I>,
}

impl<'host, 'runtime, I, Id> Clone for SdkObject<'host, 'runtime, I, Id>
where
    I: ComponentInterface,
    Id: Clone,
{
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            client: self.client.clone(),
        }
    }
}

impl<'host, 'runtime, I: ComponentInterface, Id> SdkObject<'host, 'runtime, I, Id> {
    pub fn new(id: Id, client: SdkClient<'host, 'runtime, I>) -> Self {
        Self { id, client }
    }

    pub fn id(&self) -> &Id {
        &self.id
    }

    pub fn client(&self) -> &SdkClient<'host, 'runtime, I> {
        &self.client
    }

    pub fn into_id(self) -> Id {
        self.id
    }
}

struct SdkAccess<'host, 'runtime> {
    host: &'host PluginHost<'runtime>,
    component: ComponentId,
}

impl<'host, 'runtime> SdkAccess<'host, 'runtime> {
    fn new(host: &'host PluginHost<'runtime>, component: ComponentId) -> Self {
        Self { host, component }
    }

    fn client<I: ComponentInterface>(&self) -> SdkClient<'host, 'runtime, I> {
        SdkClient::new(self.host, self.component.clone())
    }

    fn require<C: SdkContract>(&self) -> SdkClient<'host, 'runtime, C::Interface> {
        self.client::<C::Interface>()
    }
}

/// Default Phenix userspace SDK available to a plugin.
pub struct PhenixSdk<'host, 'runtime> {
    pub sessions: Sessions<'host, 'runtime>,
    pub models: SdkClient<'host, 'runtime, ModelRoutingInterface>,
    pub tools: SdkClient<'host, 'runtime, SdkToolsInterface>,
    pub skills: SdkClient<'host, 'runtime, SdkSkillsInterface>,
    pub context: SdkClient<'host, 'runtime, ContextInterface>,
    pub options: SdkClient<'host, 'runtime, OptionsInterface>,
    pub config: SdkClient<'host, 'runtime, SdkConfigInterface>,
    extensions: SdkAccess<'host, 'runtime>,
}

impl<'host, 'runtime> PhenixSdk<'host, 'runtime> {
    pub fn new(host: &'host PluginHost<'runtime>, component: ComponentId) -> Self {
        let access = SdkAccess::new(host, component);
        Self {
            sessions: Sessions::new(
                access.client::<SdkSessionInterface>(),
                access.client::<SessionInterface>(),
            ),
            models: access.client::<ModelRoutingInterface>(),
            tools: access.client::<SdkToolsInterface>(),
            skills: access.client::<SdkSkillsInterface>(),
            context: access.client::<ContextInterface>(),
            options: access.client::<OptionsInterface>(),
            config: access.client::<SdkConfigInterface>(),
            extensions: SdkAccess::new(host, access.component.clone()),
        }
    }

    /// Bind an SDK contract declared by this plugin.
    ///
    /// Invocation still goes through the caller component import, so undeclared
    /// or unbound dependencies fail at the kernel boundary.
    pub fn require<C: SdkContract>(&self) -> SdkClient<'host, 'runtime, C::Interface> {
        self.extensions.require::<C>()
    }
}

#[derive(Debug)]
pub enum SdkError {
    Invocation(ComponentInvocationError),
    UnexpectedResponse { operation: &'static str },
}

impl Display for SdkError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invocation(error) => Display::fmt(error, f),
            Self::UnexpectedResponse { operation } => {
                write!(f, "unexpected SDK response while {operation}")
            }
        }
    }
}

impl Error for SdkError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Invocation(error) => Some(error),
            Self::UnexpectedResponse { .. } => None,
        }
    }
}

impl From<ComponentInvocationError> for SdkError {
    fn from(error: ComponentInvocationError) -> Self {
        Self::Invocation(error)
    }
}

#[derive(Clone)]
pub struct Sessions<'host, 'runtime> {
    policy: SdkClient<'host, 'runtime, SdkSessionInterface>,
    storage: SdkClient<'host, 'runtime, SessionInterface>,
}

impl<'host, 'runtime> Sessions<'host, 'runtime> {
    fn new(
        policy: SdkClient<'host, 'runtime, SdkSessionInterface>,
        storage: SdkClient<'host, 'runtime, SessionInterface>,
    ) -> Self {
        Self { policy, storage }
    }

    pub fn open(&self, id: impl Into<String>) -> Result<Session<'host, 'runtime>, SdkError> {
        Ok(self.open_with_status(id, None)?.session)
    }

    pub fn open_for_agent(
        &self,
        id: impl Into<String>,
        agent: impl Into<String>,
    ) -> Result<Session<'host, 'runtime>, SdkError> {
        Ok(self.open_with_status(id, Some(agent.into()))?.session)
    }

    pub fn open_with_status(
        &self,
        id: impl Into<String>,
        agent: Option<String>,
    ) -> Result<OpenedSession<'host, 'runtime>, SdkError> {
        let response = self
            .policy
            .invoke(&SdkSessionCommand::Open {
                id: id.into(),
                agent,
            })?;
        let SdkSessionResponse::Opened { session, created } = response;
        Ok(OpenedSession {
            session: Session::new(session, self.clone()),
            created,
        })
    }

    pub fn find(
        &self,
        id: impl Into<String>,
    ) -> Result<Option<Session<'host, 'runtime>>, SdkError> {
        let response = self.storage.invoke(&SessionCommand::Get { id: id.into() })?;
        let SessionResponse::Session { session } = response else {
            return Err(SdkError::UnexpectedResponse {
                operation: "finding session",
            });
        };
        Ok(session.map(|session| Session::new(session, self.clone())))
    }

    pub fn iter(&self) -> Result<SessionIter<'host, 'runtime>, SdkError> {
        let response = self.storage.invoke(&SessionCommand::List)?;
        let SessionResponse::Sessions { sessions } = response else {
            return Err(SdkError::UnexpectedResponse {
                operation: "listing sessions",
            });
        };
        Ok(sessions
            .into_iter()
            .map(|session| Session::new(session, self.clone()))
            .collect::<Vec<_>>()
            .into_iter())
    }
}

pub type SessionIter<'host, 'runtime> = std::vec::IntoIter<Session<'host, 'runtime>>;

pub struct OpenedSession<'host, 'runtime> {
    pub session: Session<'host, 'runtime>,
    pub created: bool,
}

#[derive(Clone)]
pub struct Session<'host, 'runtime> {
    record: SessionRecord,
    sessions: Sessions<'host, 'runtime>,
}

impl<'host, 'runtime> Session<'host, 'runtime> {
    fn new(record: SessionRecord, sessions: Sessions<'host, 'runtime>) -> Self {
        Self { record, sessions }
    }

    pub fn id(&self) -> &str {
        &self.record.id
    }

    pub fn record(&self) -> &SessionRecord {
        &self.record
    }

    pub fn into_record(self) -> SessionRecord {
        self.record
    }

    pub fn refresh(&self) -> Result<Option<Self>, SdkError> {
        self.sessions.find(self.id().to_owned())
    }
}
