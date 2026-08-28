use crate::{
    SdkConfigInterface, SdkSessionCommand, SdkSessionInterface, SdkSessionResponse,
    SdkSkillsInterface, SdkToolsInterface,
};
pub use phenix_core::{
    CallContext, CurrentPlugin, KernelAccess, PluginContext, SdkClient, SdkContract, SdkObject,
};
use phenix_core::{Authority, ComponentId, ComponentInterface, PluginHost};
use phenix_plugin_context::ContextInterface;
use phenix_plugin_models::ModelRoutingInterface;
use phenix_plugin_options::OptionsInterface;
use phenix_plugin_sessions::{SessionCommand, SessionInterface, SessionRecord, SessionResponse};
use std::{error::Error, fmt::{self, Display, Formatter}};

pub type PhenixPluginContext<'host, 'runtime, Settings = (), State = ()> =
    PluginContext<'host, 'runtime, PhenixSdk<'host, 'runtime>, Settings, State>;

pub fn phenix_context<'host, 'runtime, Settings, State>(
    host: &'host PluginHost<'runtime>,
    component: ComponentId,
    settings: Settings,
    state: State,
) -> PhenixPluginContext<'host, 'runtime, Settings, State> {
    PluginContext::new(host, PhenixSdk::new(host, component), settings, state)
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
    Invocation(phenix_core::ComponentInvocationError),
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

impl From<phenix_core::ComponentInvocationError> for SdkError {
    fn from(error: phenix_core::ComponentInvocationError) -> Self {
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
        let response = self.policy.invoke(&SdkSessionCommand::Open {
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
        let response = self
            .storage
            .invoke(&SessionCommand::Get { id: id.into() })?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{sdk_component_manifest, sdk_factory, sdk_manifest};
    use phenix_core::{
        CapabilityId, ComponentExport, ComponentImport, ComponentManifest, InterfaceId, Kernel,
        KernelConfig, PluginExecution, PluginId, PluginInstance, PluginManifest, ResolvedHarness,
        ResolvedHarnessActivation, ServiceContribution, ServiceId, ServiceRole,
    };
    use phenix_plugin_options::{options_component_manifest, options_factory, options_manifest};
    use phenix_plugin_sessions::{session_component_manifest, session_factory, session_manifest};
    use serde::{Deserialize, Serialize};

    const ECHO_PLUGIN: &str = "fixture.echo";
    const ECHO_COMPONENT: &str = "fixture.echo";
    const ECHO_SERVICE: &str = "fixture.echo@1";
    const CONSUMER_PLUGIN: &str = "fixture.consumer";
    const CONSUMER_COMPONENT: &str = "fixture.consumer";
    const CONSUMER_SERVICE: &str = "fixture.consumer.run@1";

    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    struct EchoRequest {
        value: String,
    }

    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    struct EchoResponse {
        value: String,
    }

    struct EchoInterface;

    impl ComponentInterface for EchoInterface {
        type Request = EchoRequest;
        type Response = EchoResponse;

        fn interface_id() -> InterfaceId {
            InterfaceId::parse(ECHO_SERVICE).unwrap()
        }
    }

    struct EchoSdk;

    impl SdkContract for EchoSdk {
        type Interface = EchoInterface;
    }

    struct EchoPlugin;

    impl PluginInstance for EchoPlugin {
        fn start(&mut self, _host: &PluginHost<'_>) -> Result<(), String> {
            Ok(())
        }

        fn invoke(
            &mut self,
            service: &ServiceId,
            input: &[u8],
            _host: &PluginHost<'_>,
        ) -> Result<Vec<u8>, String> {
            if service != &echo_service() {
                return Err(format!("unsupported echo service: {service}"));
            }
            let request: EchoRequest =
                serde_json::from_slice(input).map_err(|error| error.to_string())?;
            serde_json::to_vec(&EchoResponse {
                value: request.value,
            })
            .map_err(|error| error.to_string())
        }
    }

    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    struct ConsumerOutput {
        plugin: String,
        settings: String,
        state: u32,
        session: String,
        echo: String,
        has_persistence_authority: bool,
    }

    struct ConsumerPlugin;

    impl PluginInstance for ConsumerPlugin {
        fn start(&mut self, _host: &PluginHost<'_>) -> Result<(), String> {
            Ok(())
        }

        fn invoke(
            &mut self,
            service: &ServiceId,
            _input: &[u8],
            host: &PluginHost<'_>,
        ) -> Result<Vec<u8>, String> {
            if service != &consumer_service() {
                return Err(format!("unsupported consumer service: {service}"));
            }

            let context = phenix_context(
                host,
                consumer_component_id(),
                "configured".to_owned(),
                7_u32,
            );
            let PluginContext {
                sdk, plugin, call, ..
            } = context;

            let opened = sdk
                .sessions
                .open("root")
                .map_err(|error| error.to_string())?;
            let found = sdk
                .sessions
                .iter()
                .map_err(|error| error.to_string())?
                .find(|session| session.id() == opened.id())
                .ok_or("session missing from SDK iterator")?;
            let refreshed = found
                .refresh()
                .map_err(|error| error.to_string())?
                .ok_or("session disappeared while refreshing SDK object")?;

            let echo = SdkObject::new("echo", sdk.require::<EchoSdk>());
            let echo = echo
                .client()
                .invoke(&EchoRequest {
                    value: refreshed.id().to_owned(),
                })
                .map_err(|error| error.to_string())?;

            serde_json::to_vec(&ConsumerOutput {
                plugin: plugin.id.as_str().to_owned(),
                settings: plugin.settings,
                state: plugin.state,
                session: refreshed.id().to_owned(),
                echo: echo.value,
                has_persistence_authority: call
                    .authority
                    .permits(&CapabilityId::parse("kernel.persistence.read").unwrap()),
            })
            .map_err(|error| error.to_string())
        }
    }

    fn authority() -> Authority {
        Authority::new([
            CapabilityId::parse("kernel.persistence.schema").unwrap(),
            CapabilityId::parse("kernel.persistence.read").unwrap(),
            CapabilityId::parse("kernel.persistence.write").unwrap(),
        ])
    }

    fn echo_service() -> ServiceId {
        ServiceId::parse(ECHO_SERVICE).unwrap()
    }

    fn consumer_service() -> ServiceId {
        ServiceId::parse(CONSUMER_SERVICE).unwrap()
    }

    fn consumer_component_id() -> ComponentId {
        ComponentId::parse(CONSUMER_COMPONENT).unwrap()
    }

    fn echo_manifest() -> PluginManifest {
        PluginManifest {
            id: PluginId::parse(ECHO_PLUGIN).unwrap(),
            version: 1,
            execution: PluginExecution::Embedded,
            dependencies: Vec::new(),
            services: vec![ServiceContribution {
                role: ServiceRole::Terminal,
                service: echo_service(),
                priority: 100,
                required_authority: Authority::default(),
            }],
            resource_namespaces: Vec::new(),
            maximum_authority: Authority::default(),
        }
    }

    fn echo_component_manifest() -> ComponentManifest {
        ComponentManifest {
            id: ComponentId::parse(ECHO_COMPONENT).unwrap(),
            owner: PluginId::parse(ECHO_PLUGIN).unwrap(),
            imports: Vec::new(),
            exports: vec![ComponentExport {
                interface: EchoInterface::interface_id(),
                priority: 100,
                required_authority: Authority::default(),
            }],
            maximum_authority: Authority::default(),
        }
    }

    fn consumer_manifest(authority: Authority) -> PluginManifest {
        PluginManifest {
            id: PluginId::parse(CONSUMER_PLUGIN).unwrap(),
            version: 1,
            execution: PluginExecution::Embedded,
            dependencies: Vec::new(),
            services: vec![ServiceContribution {
                role: ServiceRole::Terminal,
                service: consumer_service(),
                priority: 100,
                required_authority: authority.clone(),
            }],
            resource_namespaces: Vec::new(),
            maximum_authority: authority,
        }
    }

    fn consumer_component_manifest(authority: Authority) -> ComponentManifest {
        let required = |interface| ComponentImport {
            interface,
            required: true,
            authority: authority.clone(),
        };
        ComponentManifest {
            id: consumer_component_id(),
            owner: PluginId::parse(CONSUMER_PLUGIN).unwrap(),
            imports: vec![
                required(SdkSessionInterface::interface_id()),
                required(SessionInterface::interface_id()),
                required(EchoInterface::interface_id()),
            ],
            exports: Vec::new(),
            maximum_authority: authority,
        }
    }

    #[test]
    fn context_sdk_and_provider_owned_objects_remain_kernel_mediated() {
        let authority = authority();
        let session = session_manifest();
        let options = options_manifest();
        let sdk = sdk_manifest(authority.clone());
        let echo = echo_manifest();
        let consumer = consumer_manifest(authority.clone());
        let manifests = vec![
            session.clone(),
            options.clone(),
            sdk.clone(),
            echo.clone(),
            consumer.clone(),
        ];
        let resolved = ResolvedHarness::resolve(
            manifests.clone(),
            [
                session_component_manifest(),
                options_component_manifest(),
                sdk_component_manifest(authority.clone()),
                echo_component_manifest(),
                consumer_component_manifest(authority.clone()),
            ],
            [],
            &authority,
        )
        .unwrap();
        let mut kernel = Kernel::new(KernelConfig::new(manifests).unwrap());
        kernel
            .register_embedded_factory(session.id, session_factory)
            .unwrap();
        kernel
            .register_embedded_factory(options.id, options_factory)
            .unwrap();
        kernel
            .register_embedded_factory(sdk.id, sdk_factory)
            .unwrap();
        kernel
            .register_embedded_factory(echo.id, || Box::new(EchoPlugin))
            .unwrap();
        kernel
            .register_embedded_factory(consumer.id, || Box::new(ConsumerPlugin))
            .unwrap();
        kernel.activate_resolved_harness(&resolved).unwrap();
        kernel.activate_all().unwrap();

        let output = kernel
            .invoke(&consumer_service(), &[], &authority, None)
            .unwrap();
        let output: ConsumerOutput = serde_json::from_slice(&output).unwrap();

        assert_eq!(
            output,
            ConsumerOutput {
                plugin: CONSUMER_PLUGIN.into(),
                settings: "configured".into(),
                state: 7,
                session: "root".into(),
                echo: "root".into(),
                has_persistence_authority: true,
            }
        );
    }
}
