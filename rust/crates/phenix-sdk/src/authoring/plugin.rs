use phenix_core::{
    Authority, ComponentId, ComponentInterface, ComponentInvocationError, EventBus,
    EventDispatchReport, EventEnvelope, EventError, EventFailurePolicy, EventSubscription,
    EventTypeId, Exact, InterfaceId, KernelAccess, PhenixValue, PluginContext, PluginHost,
    PluginId, Project, SdkClient, SubscriptionId, SubscriptionSpec, ValueError,
};
use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    marker::PhantomData,
    sync::{Arc, Weak},
};

#[doc(hidden)]
pub mod __phenix_plugin {
    pub use phenix_core::{
        Authority, ComponentExport, ComponentId, ComponentImport, ComponentInterface,
        ComponentInvocationError, ComponentManifest, EventBus, EventSubscription, InterfaceId,
        InterfaceSchema, PhenixValue, PluginContext, PluginExecution, PluginHost, PluginId,
        PluginManifest, ServiceContribution, ServiceId, ServiceRole,
    };
}

const STRUCTURAL_MISMATCH_EVENT: &str = "kernel.structural_value_mismatch";
const STRUCTURAL_MISMATCH_EVENT_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct EventName(EventTypeId);

impl EventName {
    pub fn parse(value: impl Into<String>) -> Result<Self, &'static str> {
        EventTypeId::parse(value).map(Self)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    fn event_type(&self) -> EventTypeId {
        self.0.clone()
    }
}

impl Display for EventName {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct HookName(InterfaceId);

impl HookName {
    pub fn parse(value: impl Into<String>) -> Result<Self, &'static str> {
        InterfaceId::parse(value).map(Self)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    #[must_use]
    pub fn interface_id(&self) -> InterfaceId {
        self.0.clone()
    }
}

impl Display for HookName {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

#[derive(Debug)]
pub enum EventEmitError {
    Encode(String),
    Dispatch(EventError),
}

impl Display for EventEmitError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Encode(message) => write!(f, "event payload encoding failed: {message}"),
            Self::Dispatch(error) => Display::fmt(error, f),
        }
    }
}

impl Error for EventEmitError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Encode(_) => None,
            Self::Dispatch(error) => Some(error),
        }
    }
}

#[derive(Clone)]
pub struct EventEmitter<'host, 'runtime> {
    kernel: KernelAccess<'host, 'runtime>,
    name: EventName,
}

impl<'host, 'runtime> EventEmitter<'host, 'runtime> {
    #[must_use]
    pub fn new(host: &'host PluginHost<'runtime>, name: EventName) -> Self {
        let kernel = PluginContext::new(host, (), (), ()).kernel;
        Self { kernel, name }
    }

    #[must_use]
    pub fn name(&self) -> &EventName {
        &self.name
    }

    pub fn emit<T>(&self, payload: &T) -> Result<EventDispatchReport, EventEmitError>
    where
        for<'value> PhenixValue: From<&'value T>,
    {
        self.emit_value(&PhenixValue::from(payload))
    }

    pub fn emit_value(&self, payload: &PhenixValue) -> Result<EventDispatchReport, EventEmitError> {
        let payload = serde_json::to_vec(payload)
            .map_err(|error| EventEmitError::Encode(error.to_string()))?;
        self.kernel
            .dispatch_event(self.name.event_type(), 1, 0, 0, payload)
            .map_err(EventEmitError::Dispatch)
    }
}

pub struct TypedSdkClient<'host, 'runtime, I, Request, Response>
where
    I: ComponentInterface,
{
    inner: SdkClient<'host, 'runtime, I>,
    marker: PhantomData<fn(Request) -> Response>,
}

impl<'host, 'runtime, I, Request, Response> Clone
    for TypedSdkClient<'host, 'runtime, I, Request, Response>
where
    I: ComponentInterface,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            marker: PhantomData,
        }
    }
}

impl<'host, 'runtime, I, Request, Response> TypedSdkClient<'host, 'runtime, I, Request, Response>
where
    I: ComponentInterface,
{
    #[must_use]
    pub fn new(host: &'host PluginHost<'runtime>, component: ComponentId) -> Self {
        Self {
            inner: SdkClient::new(host, component),
            marker: PhantomData,
        }
    }

    pub fn invoke(&self, request: &Request) -> Result<Response, ComponentInvocationError>
    where
        for<'value> PhenixValue: From<&'value Request>,
        for<'value> Response: TryFrom<Project<&'value PhenixValue>, Error = ValueError>,
    {
        self.inner.invoke_projected(request)
    }

    pub fn invoke_exact(&self, request: &Request) -> Result<Response, ComponentInvocationError>
    where
        for<'value> PhenixValue: From<&'value Request>,
        for<'value> Response: TryFrom<Exact<&'value PhenixValue>, Error = ValueError>,
    {
        self.inner.invoke_exact(request)
    }

    pub fn invoke_value(
        &self,
        request: &PhenixValue,
    ) -> Result<PhenixValue, ComponentInvocationError> {
        self.inner.invoke_value(request)
    }
}

pub fn dispatch_projected_provider<Request, Response, E, F>(
    host: &PluginHost<'_>,
    interface: &InterfaceId,
    input: &[u8],
    handler: F,
) -> Result<Vec<u8>, String>
where
    for<'value> Request: TryFrom<Project<&'value PhenixValue>, Error = ValueError>,
    for<'value> PhenixValue: From<&'value Response>,
    E: Display,
    F: FnOnce(Request) -> Result<Response, E>,
{
    let kernel = PluginContext::new(host, (), (), ()).kernel;
    let request = kernel
        .decode_projected::<Request>(interface, input)
        .map_err(|error| error.to_string())?;
    let response = handler(request).map_err(|error| error.to_string())?;
    kernel
        .encode_value(&response)
        .map_err(|error| error.to_string())
}

pub fn dispatch_exact_provider<Request, Response, E, F>(
    host: &PluginHost<'_>,
    interface: &InterfaceId,
    input: &[u8],
    handler: F,
) -> Result<Vec<u8>, String>
where
    for<'value> Request: TryFrom<Exact<&'value PhenixValue>, Error = ValueError>,
    for<'value> PhenixValue: From<&'value Response>,
    E: Display,
    F: FnOnce(Request) -> Result<Response, E>,
{
    let kernel = PluginContext::new(host, (), (), ()).kernel;
    let request = kernel
        .decode_exact::<Request>(interface, input)
        .map_err(|error| error.to_string())?;
    let response = handler(request).map_err(|error| error.to_string())?;
    kernel
        .encode_value(&response)
        .map_err(|error| error.to_string())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ListenerProjection {
    Project,
    Exact,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListenerDeclaration {
    pub local_name: &'static str,
    pub event: &'static str,
    pub projection: ListenerProjection,
}

pub fn listener_subscription<T, E, F>(
    events: &Arc<EventBus>,
    owner: PluginId,
    local_name: &'static str,
    event: &'static str,
    projection: ListenerProjection,
    maximum_authority: Authority,
    handler: F,
) -> EventSubscription
where
    for<'value> T: TryFrom<Project<&'value PhenixValue>, Error = ValueError>,
    for<'value> T: TryFrom<Exact<&'value PhenixValue>, Error = ValueError>,
    E: Display,
    F: Fn(T) -> Result<(), E> + Send + Sync + 'static,
{
    let subscription = SubscriptionId::parse(format!("{}/listener/{local_name}", owner.as_str()))
        .expect("phenix_plugin! generated listener subscription id is valid");
    let event_type =
        EventTypeId::parse(event).expect("phenix_plugin! validated static listener event type");
    let diagnostic_events = Arc::downgrade(events);
    let diagnostic_owner = owner.clone();

    EventSubscription {
        spec: SubscriptionSpec {
            id: subscription,
            owner,
            event_type,
            event_version: 1,
            dependencies: Vec::new(),
            failure_policy: EventFailurePolicy::Warn,
            required_authority: Authority::default(),
            maximum_authority,
            kernel_policy_revision: 0,
        },
        handler: Arc::new(move |envelope: &EventEnvelope, authority: &Authority| {
            let value = serde_json::from_slice::<PhenixValue>(&envelope.payload)
                .map_err(|error| format!("listener payload decoding failed: {error}"))?;
            let payload = match projection {
                ListenerProjection::Project => T::try_from(Project(&value)),
                ListenerProjection::Exact => T::try_from(Exact(&value)),
            };
            let payload = payload.map_err(|error| {
                report_listener_mismatch(
                    &diagnostic_events,
                    &diagnostic_owner,
                    local_name,
                    envelope,
                    authority,
                    &error,
                );
                error.to_string()
            })?;
            handler(payload).map_err(|error| error.to_string())
        }),
    }
}

fn report_listener_mismatch(
    events: &Weak<EventBus>,
    owner: &PluginId,
    listener: &str,
    source: &EventEnvelope,
    authority: &Authority,
    error: &ValueError,
) {
    let Some(events) = events.upgrade() else {
        return;
    };
    let Ok(payload) = serde_json::to_vec(&serde_json::json!({
        "event": source.event_type.as_str(),
        "listener": listener,
        "direction": "listener",
        "error": error.to_string(),
    })) else {
        return;
    };
    let diagnostic = EventEnvelope {
        event_type: EventTypeId::parse(STRUCTURAL_MISMATCH_EVENT)
            .expect("static structural mismatch event type is valid"),
        version: STRUCTURAL_MISMATCH_EVENT_VERSION,
        emitter: owner.clone(),
        causality_id: source.causality_id,
        kernel_policy_revision: source.kernel_policy_revision,
        payload,
    };
    let _ = events.dispatch(&diagnostic, authority);
}

#[doc(hidden)]
#[macro_export]
macro_rules! __phenix_listener_projection {
    (Project) => {
        $crate::ListenerProjection::Project
    };
    (Exact) => {
        $crate::ListenerProjection::Exact
    };
}

#[macro_export]
macro_rules! phenix_plugin {
    (
        $name:literal;
        $(
            uses {
                $(
                    $dependency:ident : $dependency_id:literal
                        => $dependency_request:ty => $dependency_response:ty
                ),* $(,)?
            }
        )?
        $(
            provides {
                $(
                    $provided:ident : $provided_id:literal
                        => $provided_request:ty => $provided_response:ty
                ),* $(,)?
            }
        )?
        $(emits { $( $emitted:ident : $event_id:literal ),* $(,)? })?
        $(
            listens {
                $( $listener:ident : $listen_event:literal => $listener_ty:ty => $handler:path ),* $(,)?
            }
        )?
        $(
            exact_listens {
                $( $exact_listener:ident : $exact_event:literal => $exact_ty:ty => $exact_handler:path ),* $(,)?
            }
        )?
        $(
            hooks {
                $(
                    provides {
                        $(
                            $hook_provided:ident : $hook_provided_id:literal
                                => $hook_provided_request:ty => $hook_provided_response:ty
                        ),* $(,)?
                    }
                )?
                $(
                    uses {
                        $(
                            $hook_used:ident : $hook_used_id:literal
                                => $hook_used_request:ty => $hook_used_response:ty
                        ),* $(,)?
                    }
                )?
            }
        )?
        $(,)?
    ) => {
        $crate::phenix_plugin! {
            name: $name,
            dependencies: {
                $(
                    $(
                        $dependency: {
                            id: $dependency_id,
                            request: $dependency_request,
                            response: $dependency_response,
                        },
                    )*
                )?
            },
            provides: {
                $(
                    $(
                        $provided: {
                            id: $provided_id,
                            request: $provided_request,
                            response: $provided_response,
                        },
                    )*
                )?
            },
            events: {
                emits: { $( $( $emitted: $event_id, )* )? },
                listens: {
                    $(
                        $(
                            $listener: {
                                event: $listen_event,
                                as: Project<$listener_ty>,
                                handler: $handler,
                            },
                        )*
                    )?
                    $(
                        $(
                            $exact_listener: {
                                event: $exact_event,
                                as: Exact<$exact_ty>,
                                handler: $exact_handler,
                            },
                        )*
                    )?
                },
            },
            hooks: {
                provides: {
                    $(
                        $(
                            $(
                                $hook_provided: {
                                    id: $hook_provided_id,
                                    request: $hook_provided_request,
                                    response: $hook_provided_response,
                                },
                            )*
                        )?
                    )?
                },
                uses: {
                    $(
                        $(
                            $(
                                $hook_used: {
                                    id: $hook_used_id,
                                    request: $hook_used_request,
                                    response: $hook_used_response,
                                },
                            )*
                        )?
                    )?
                },
            },
        }
    };

    (
        name: $name:literal,
        dependencies: {
            $(
                $dependency:ident : {
                    id: $dependency_id:literal,
                    request: $dependency_request:ty,
                    response: $dependency_response:ty $(,)?
                }
            ),* $(,)?
        },
        provides: {
            $(
                $provided:ident : {
                    id: $provided_id:literal,
                    request: $provided_request:ty,
                    response: $provided_response:ty $(,)?
                }
            ),* $(,)?
        },
        events: {
            emits: { $( $emitted:ident : $event_id:literal ),* $(,)? },
            listens: {
                $(
                    $listener:ident : {
                        event: $listen_event:literal,
                        as: $projection:ident < $listener_ty:ty >,
                        handler: $handler:path $(,)?
                    }
                ),* $(,)?
            },
        },
        hooks: {
            provides: {
                $(
                    $hook_provided:ident : {
                        id: $hook_provided_id:literal,
                        request: $hook_provided_request:ty,
                        response: $hook_provided_response:ty $(,)?
                    }
                ),* $(,)?
            },
            uses: {
                $(
                    $hook_used:ident : {
                        id: $hook_used_id:literal,
                        request: $hook_used_request:ty,
                        response: $hook_used_response:ty $(,)?
                    }
                ),* $(,)?
            },
        } $(,)?
    ) => {
        pub mod phenix_plugin {
            #[allow(unused_imports)]
            use super::*;

            pub const NAME: &str = $name;

            #[must_use]
            pub fn plugin_id() -> $crate::__phenix_plugin::PluginId {
                $crate::__phenix_plugin::PluginId::parse(NAME)
                    .expect("phenix_plugin! validated static plugin id")
            }

            #[must_use]
            pub fn component_id() -> $crate::__phenix_plugin::ComponentId {
                $crate::__phenix_plugin::ComponentId::parse(NAME)
                    .expect("phenix_plugin! validated static component id")
            }

            pub mod dependencies {
                $(
                    pub mod $dependency {
                        #[allow(unused_imports)]
                        use super::super::super::*;

                        pub struct Interface;

                        impl $crate::__phenix_plugin::ComponentInterface for Interface {
                            fn interface_id() -> $crate::__phenix_plugin::InterfaceId {
                                $crate::__phenix_plugin::InterfaceId::parse($dependency_id)
                                    .expect("phenix_plugin! validated static dependency interface id")
                            }

                            fn schema() -> $crate::__phenix_plugin::InterfaceSchema {
                                $crate::__phenix_plugin::InterfaceSchema::of::<
                                    $dependency_request,
                                    $dependency_response,
                                >()
                            }
                        }

                        pub type Client<'host, 'runtime> = $crate::TypedSdkClient<
                            'host,
                            'runtime,
                            Interface,
                            $dependency_request,
                            $dependency_response,
                        >;
                    }
                )*
            }

            pub mod provides {
                $(
                    pub mod $provided {
                        #[allow(unused_imports)]
                        use super::super::super::*;

                        pub struct Interface;

                        impl $crate::__phenix_plugin::ComponentInterface for Interface {
                            fn interface_id() -> $crate::__phenix_plugin::InterfaceId {
                                $crate::__phenix_plugin::InterfaceId::parse($provided_id)
                                    .expect("phenix_plugin! validated static provided interface id")
                            }

                            fn schema() -> $crate::__phenix_plugin::InterfaceSchema {
                                $crate::__phenix_plugin::InterfaceSchema::of::<
                                    $provided_request,
                                    $provided_response,
                                >()
                            }
                        }

                        pub fn dispatch<E, F>(
                            host: &$crate::__phenix_plugin::PluginHost<'_>,
                            input: &[u8],
                            handler: F,
                        ) -> Result<Vec<u8>, String>
                        where
                            E: ::std::fmt::Display,
                            F: FnOnce($provided_request) -> Result<$provided_response, E>,
                        {
                            $crate::dispatch_projected_provider(
                                host,
                                &<Interface as $crate::__phenix_plugin::ComponentInterface>::interface_id(),
                                input,
                                handler,
                            )
                        }

                        pub fn dispatch_exact<E, F>(
                            host: &$crate::__phenix_plugin::PluginHost<'_>,
                            input: &[u8],
                            handler: F,
                        ) -> Result<Vec<u8>, String>
                        where
                            E: ::std::fmt::Display,
                            F: FnOnce($provided_request) -> Result<$provided_response, E>,
                        {
                            $crate::dispatch_exact_provider(
                                host,
                                &<Interface as $crate::__phenix_plugin::ComponentInterface>::interface_id(),
                                input,
                                handler,
                            )
                        }
                    }
                )*
            }

            pub mod hook_providers {
                $(
                    pub mod $hook_provided {
                        #[allow(unused_imports)]
                        use super::super::super::*;

                        pub struct Interface;

                        impl $crate::__phenix_plugin::ComponentInterface for Interface {
                            fn interface_id() -> $crate::__phenix_plugin::InterfaceId {
                                $crate::__phenix_plugin::InterfaceId::parse($hook_provided_id)
                                    .expect("phenix_plugin! validated static provided hook id")
                            }

                            fn schema() -> $crate::__phenix_plugin::InterfaceSchema {
                                $crate::__phenix_plugin::InterfaceSchema::of::<
                                    $hook_provided_request,
                                    $hook_provided_response,
                                >()
                            }
                        }

                        pub fn dispatch<E, F>(
                            host: &$crate::__phenix_plugin::PluginHost<'_>,
                            input: &[u8],
                            handler: F,
                        ) -> Result<Vec<u8>, String>
                        where
                            E: ::std::fmt::Display,
                            F: FnOnce($hook_provided_request) -> Result<$hook_provided_response, E>,
                        {
                            $crate::dispatch_projected_provider(
                                host,
                                &<Interface as $crate::__phenix_plugin::ComponentInterface>::interface_id(),
                                input,
                                handler,
                            )
                        }
                    }
                )*
            }

            pub mod hook_consumers {
                $(
                    pub mod $hook_used {
                        #[allow(unused_imports)]
                        use super::super::super::*;

                        pub struct Interface;

                        impl $crate::__phenix_plugin::ComponentInterface for Interface {
                            fn interface_id() -> $crate::__phenix_plugin::InterfaceId {
                                $crate::__phenix_plugin::InterfaceId::parse($hook_used_id)
                                    .expect("phenix_plugin! validated static used hook id")
                            }

                            fn schema() -> $crate::__phenix_plugin::InterfaceSchema {
                                $crate::__phenix_plugin::InterfaceSchema::of::<
                                    $hook_used_request,
                                    $hook_used_response,
                                >()
                            }
                        }

                        pub type Client<'host, 'runtime> = $crate::TypedSdkClient<
                            'host,
                            'runtime,
                            Interface,
                            $hook_used_request,
                            $hook_used_response,
                        >;
                    }
                )*
            }

            pub struct Hooks<'host, 'runtime> {
                $(
                    pub $hook_used: hook_consumers::$hook_used::Client<'host, 'runtime>,
                )*
                marker: ::std::marker::PhantomData<
                    &'host $crate::__phenix_plugin::PluginHost<'runtime>,
                >,
            }

            pub struct Events<'host, 'runtime> {
                $(pub $emitted: $crate::EventEmitter<'host, 'runtime>,)*
                marker: ::std::marker::PhantomData<
                    &'host $crate::__phenix_plugin::PluginHost<'runtime>,
                >,
            }

            pub struct Sdk<'host, 'runtime> {
                $(pub $dependency: dependencies::$dependency::Client<'host, 'runtime>,)*
                pub events: Events<'host, 'runtime>,
                pub hooks: Hooks<'host, 'runtime>,
            }

            impl<'host, 'runtime> Sdk<'host, 'runtime> {
                #[must_use]
                pub fn new(host: &'host $crate::__phenix_plugin::PluginHost<'runtime>) -> Self {
                    let component = component_id();
                    Self {
                        $(
                            $dependency: $crate::TypedSdkClient::new(host, component.clone()),
                        )*
                        events: Events {
                            $(
                                $emitted: $crate::EventEmitter::new(
                                    host,
                                    $crate::EventName::parse($event_id)
                                        .expect("phenix_plugin! validated static event name"),
                                ),
                            )*
                            marker: ::std::marker::PhantomData,
                        },
                        hooks: Hooks {
                            $(
                                $hook_used: $crate::TypedSdkClient::new(host, component.clone()),
                            )*
                            marker: ::std::marker::PhantomData,
                        },
                    }
                }
            }

            pub type Context<'host, 'runtime, Settings = (), State = ()> =
                $crate::__phenix_plugin::PluginContext<
                    'host,
                    'runtime,
                    Sdk<'host, 'runtime>,
                    Settings,
                    State,
                >;

            #[must_use]
            pub fn context<'host, 'runtime, Settings, State>(
                host: &'host $crate::__phenix_plugin::PluginHost<'runtime>,
                settings: Settings,
                state: State,
            ) -> Context<'host, 'runtime, Settings, State> {
                $crate::__phenix_plugin::PluginContext::new(
                    host,
                    Sdk::new(host),
                    settings,
                    state,
                )
            }

            #[must_use]
            pub fn plugin_manifest(
                maximum_authority: $crate::__phenix_plugin::Authority,
            ) -> $crate::__phenix_plugin::PluginManifest {
                $crate::__phenix_plugin::PluginManifest {
                    id: plugin_id(),
                    version: 1,
                    execution: $crate::__phenix_plugin::PluginExecution::Embedded,
                    dependencies: Vec::new(),
                    services: vec![
                        $(
                            $crate::__phenix_plugin::ServiceContribution {
                                role: $crate::__phenix_plugin::ServiceRole::Terminal,
                                service: $crate::__phenix_plugin::ServiceId::parse($provided_id)
                                    .expect("phenix_plugin! validated static provided service id"),
                                priority: 100,
                                required_authority: $crate::__phenix_plugin::Authority::default(),
                            },
                        )*
                        $(
                            $crate::__phenix_plugin::ServiceContribution {
                                role: $crate::__phenix_plugin::ServiceRole::Terminal,
                                service: $crate::__phenix_plugin::ServiceId::parse($hook_provided_id)
                                    .expect("phenix_plugin! validated static provided hook service id"),
                                priority: 100,
                                required_authority: $crate::__phenix_plugin::Authority::default(),
                            },
                        )*
                    ],
                    resource_namespaces: Vec::new(),
                    maximum_authority,
                }
            }

            #[must_use]
            pub fn component_manifest(
                maximum_authority: $crate::__phenix_plugin::Authority,
            ) -> $crate::__phenix_plugin::ComponentManifest {
                $crate::__phenix_plugin::ComponentManifest {
                    id: component_id(),
                    owner: plugin_id(),
                    imports: vec![
                        $(
                            $crate::__phenix_plugin::ComponentImport {
                                interface: <dependencies::$dependency::Interface as $crate::__phenix_plugin::ComponentInterface>::interface_id(),
                                schema: <dependencies::$dependency::Interface as $crate::__phenix_plugin::ComponentInterface>::schema(),
                                required: true,
                                authority: maximum_authority.clone(),
                            },
                        )*
                        $(
                            $crate::__phenix_plugin::ComponentImport {
                                interface: <hook_consumers::$hook_used::Interface as $crate::__phenix_plugin::ComponentInterface>::interface_id(),
                                schema: <hook_consumers::$hook_used::Interface as $crate::__phenix_plugin::ComponentInterface>::schema(),
                                required: true,
                                authority: maximum_authority.clone(),
                            },
                        )*
                    ],
                    exports: vec![
                        $(
                            $crate::__phenix_plugin::ComponentExport {
                                interface: <provides::$provided::Interface as $crate::__phenix_plugin::ComponentInterface>::interface_id(),
                                schema: <provides::$provided::Interface as $crate::__phenix_plugin::ComponentInterface>::schema(),
                                priority: 100,
                                required_authority: $crate::__phenix_plugin::Authority::default(),
                            },
                        )*
                        $(
                            $crate::__phenix_plugin::ComponentExport {
                                interface: <hook_providers::$hook_provided::Interface as $crate::__phenix_plugin::ComponentInterface>::interface_id(),
                                schema: <hook_providers::$hook_provided::Interface as $crate::__phenix_plugin::ComponentInterface>::schema(),
                                priority: 100,
                                required_authority: $crate::__phenix_plugin::Authority::default(),
                            },
                        )*
                    ],
                    maximum_authority,
                }
            }

            #[must_use]
            pub fn listeners() -> Vec<$crate::ListenerDeclaration> {
                vec![
                    $(
                        $crate::ListenerDeclaration {
                            local_name: stringify!($listener),
                            event: $listen_event,
                            projection: $crate::__phenix_listener_projection!($projection),
                        },
                    )*
                ]
            }

            #[must_use]
            pub fn event_subscriptions(
                events: &::std::sync::Arc<$crate::__phenix_plugin::EventBus>,
                maximum_authority: $crate::__phenix_plugin::Authority,
            ) -> Vec<$crate::__phenix_plugin::EventSubscription> {
                vec![
                    $(
                        $crate::listener_subscription::<$listener_ty, _, _>(
                            events,
                            plugin_id(),
                            stringify!($listener),
                            $listen_event,
                            $crate::__phenix_listener_projection!($projection),
                            maximum_authority.clone(),
                            $handler,
                        ),
                    )*
                ]
            }
        }
    };
}
