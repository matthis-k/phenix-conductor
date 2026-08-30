#![forbid(unsafe_code)]

mod providers;

pub use phenix_core::{
    Authority, Bytes, CallableRef, ComponentExport, ComponentId, ComponentImport,
    ComponentInterface, ComponentManifest, Contract, ContractId, ContractValue,
    EventDispatchReport, EventError, EventTypeId, Exact, InterfaceId, InterfaceSchema, Key,
    ObjectRef, PhenixContract, PhenixSchema, PhenixValue, PluginExecution, PluginHost, PluginId,
    PluginManifest, Project, ReferenceId, ServiceContribution, ServiceId, ServiceRole, Type,
    TypeKind, ValueError,
};
pub use phenix_plugin_sdk::*;
pub use phenix_provider_sdk::{
    ApiTokenSource, Auth, AuthDescriptor, AuthKind, EnvironmentVariable, ProviderAuthCommand,
    ProviderAuthInterface, ProviderAuthResponse, ProviderError, RateLimits,
};
pub use phenix_sdk_macros::{PhenixContract, PhenixValue};
pub use providers::{Provider, ProviderSdkError, ProviderSdkExt, Providers};

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

pub mod auth {
    pub use phenix_provider_sdk::auth::*;
}

pub mod provider {
    pub use phenix_provider_sdk::provider::*;
}

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

#[doc(hidden)]
#[macro_export]
macro_rules! __phenix_decode_listener {
    (Project, $ty:ty, $value:expr) => {
        <$ty>::try_from($crate::Project($value))
    };
    (Exact, $ty:ty, $value:expr) => {
        <$ty>::try_from($crate::Exact($value))
    };
}

#[macro_export]
macro_rules! phenix_plugin {
    (
        $name:literal;
        $(
            uses {
                $( $dependency:ident : $dependency_id:literal ),* $(,)?
            }
        )?
        $(
            provides {
                $( $provided:ident : $provided_id:literal ),* $(,)?
            }
        )?
        $(
            emits {
                $( $emitted:ident : $event_id:literal ),* $(,)?
            }
        )?
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
                        $( $hook_provided:ident : $hook_provided_id:literal ),* $(,)?
                    }
                )?
                $(
                    uses {
                        $( $hook_used:ident : $hook_used_id:literal ),* $(,)?
                    }
                )?
            }
        )?
        $(,)?
    ) => {
        $crate::phenix_plugin! {
            name: $name,
            dependencies: {
                $( $( $dependency: $dependency_id, )* )?
            },
            provides: {
                $( $( $provided: $provided_id, )* )?
            },
            events: {
                emits: {
                    $( $( $emitted: $event_id, )* )?
                },
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
                    $( $( $( $hook_provided: $hook_provided_id, )* )? )?
                },
                uses: {
                    $( $( $( $hook_used: $hook_used_id, )* )? )?
                },
            },
        }
    };

    (
        name: $name:literal,
        dependencies: { $( $dependency:ident : $dependency_id:literal ),* $(,)? },
        provides: { $( $provided:ident : $provided_id:literal ),* $(,)? },
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
            provides: { $( $hook_provided:ident : $hook_provided_id:literal ),* $(,)? },
            uses: { $( $hook_used:ident : $hook_used_id:literal ),* $(,)? },
        } $(,)?
    ) => {
        pub mod phenix_plugin {
            #[allow(unused_imports)]
            use super::*;

            pub const NAME: &str = $name;

            #[must_use]
            pub fn plugin_id() -> $crate::PluginId {
                $crate::PluginId::parse(NAME).expect("phenix_plugin! validated static plugin id")
            }

            #[must_use]
            pub fn component_id() -> $crate::ComponentId {
                $crate::ComponentId::parse(NAME)
                    .expect("phenix_plugin! validated static component id")
            }

            pub mod dependencies {
                $(
                    pub mod $dependency {
                        pub struct Interface;

                        impl $crate::ComponentInterface for Interface {
                            fn interface_id() -> $crate::InterfaceId {
                                $crate::InterfaceId::parse($dependency_id)
                                    .expect("phenix_plugin! validated static dependency interface id")
                            }
                        }
                    }
                )*
            }

            pub mod provides {
                $(
                    pub mod $provided {
                        pub struct Interface;

                        impl $crate::ComponentInterface for Interface {
                            fn interface_id() -> $crate::InterfaceId {
                                $crate::InterfaceId::parse($provided_id)
                                    .expect("phenix_plugin! validated static provided interface id")
                            }
                        }
                    }
                )*
            }

            pub mod hook_providers {
                $(
                    pub mod $hook_provided {
                        pub struct Interface;

                        impl $crate::ComponentInterface for Interface {
                            fn interface_id() -> $crate::InterfaceId {
                                $crate::InterfaceId::parse($hook_provided_id)
                                    .expect("phenix_plugin! validated static provided hook id")
                            }
                        }
                    }
                )*
            }

            pub mod hook_consumers {
                $(
                    pub mod $hook_used {
                        pub struct Interface;

                        impl $crate::ComponentInterface for Interface {
                            fn interface_id() -> $crate::InterfaceId {
                                $crate::InterfaceId::parse($hook_used_id)
                                    .expect("phenix_plugin! validated static used hook id")
                            }
                        }
                    }
                )*
            }

            pub struct Hooks<'host, 'runtime> {
                $(
                    pub $hook_used: $crate::SdkClient<
                        'host,
                        'runtime,
                        hook_consumers::$hook_used::Interface,
                    >,
                )*
                _marker: ::std::marker::PhantomData<&'host $crate::PluginHost<'runtime>>,
            }

            pub struct Events<'host, 'runtime> {
                $(pub $emitted: $crate::EventEmitter<'host, 'runtime>,)*
                _marker: ::std::marker::PhantomData<&'host $crate::PluginHost<'runtime>>,
            }

            pub struct Sdk<'host, 'runtime> {
                $(
                    pub $dependency: $crate::SdkClient<
                        'host,
                        'runtime,
                        dependencies::$dependency::Interface,
                    >,
                )*
                pub events: Events<'host, 'runtime>,
                pub hooks: Hooks<'host, 'runtime>,
            }

            impl<'host, 'runtime> Sdk<'host, 'runtime> {
                #[must_use]
                pub fn new(host: &'host $crate::PluginHost<'runtime>) -> Self {
                    let component = component_id();
                    Self {
                        $(
                            $dependency: $crate::SdkClient::new(host, component.clone()),
                        )*
                        events: Events {
                            $(
                                $emitted: $crate::EventEmitter::new(
                                    host,
                                    $crate::EventName::parse($event_id)
                                        .expect("phenix_plugin! validated static event name"),
                                ),
                            )*
                            _marker: ::std::marker::PhantomData,
                        },
                        hooks: Hooks {
                            $(
                                $hook_used: $crate::SdkClient::new(host, component.clone()),
                            )*
                            _marker: ::std::marker::PhantomData,
                        },
                    }
                }
            }

            pub type Context<'host, 'runtime, Settings = (), State = ()> =
                $crate::PluginContext<'host, 'runtime, Sdk<'host, 'runtime>, Settings, State>;

            #[must_use]
            pub fn context<'host, 'runtime, Settings, State>(
                host: &'host $crate::PluginHost<'runtime>,
                settings: Settings,
                state: State,
            ) -> Context<'host, 'runtime, Settings, State> {
                $crate::PluginContext::new(host, Sdk::new(host), settings, state)
            }

            #[must_use]
            pub fn plugin_manifest(maximum_authority: $crate::Authority) -> $crate::PluginManifest {
                $crate::PluginManifest {
                    id: plugin_id(),
                    version: 1,
                    execution: $crate::PluginExecution::Embedded,
                    dependencies: Vec::new(),
                    services: vec![
                        $(
                            $crate::ServiceContribution {
                                role: $crate::ServiceRole::Terminal,
                                service: $crate::ServiceId::parse($provided_id)
                                    .expect("phenix_plugin! validated static provided service id"),
                                priority: 100,
                                required_authority: $crate::Authority::default(),
                            },
                        )*
                        $(
                            $crate::ServiceContribution {
                                role: $crate::ServiceRole::Terminal,
                                service: $crate::ServiceId::parse($hook_provided_id)
                                    .expect("phenix_plugin! validated static provided hook service id"),
                                priority: 100,
                                required_authority: $crate::Authority::default(),
                            },
                        )*
                    ],
                    resource_namespaces: Vec::new(),
                    maximum_authority,
                }
            }

            #[must_use]
            pub fn component_manifest(
                maximum_authority: $crate::Authority,
            ) -> $crate::ComponentManifest {
                $crate::ComponentManifest {
                    id: component_id(),
                    owner: plugin_id(),
                    imports: vec![
                        $(
                            $crate::ComponentImport {
                                interface: <dependencies::$dependency::Interface as $crate::ComponentInterface>::interface_id(),
                                schema: <dependencies::$dependency::Interface as $crate::ComponentInterface>::schema(),
                                required: true,
                                authority: maximum_authority.clone(),
                            },
                        )*
                        $(
                            $crate::ComponentImport {
                                interface: <hook_consumers::$hook_used::Interface as $crate::ComponentInterface>::interface_id(),
                                schema: <hook_consumers::$hook_used::Interface as $crate::ComponentInterface>::schema(),
                                required: true,
                                authority: maximum_authority.clone(),
                            },
                        )*
                    ],
                    exports: vec![
                        $(
                            $crate::ComponentExport {
                                interface: <provides::$provided::Interface as $crate::ComponentInterface>::interface_id(),
                                schema: <provides::$provided::Interface as $crate::ComponentInterface>::schema(),
                                priority: 100,
                                required_authority: $crate::Authority::default(),
                            },
                        )*
                        $(
                            $crate::ComponentExport {
                                interface: <hook_providers::$hook_provided::Interface as $crate::ComponentInterface>::interface_id(),
                                schema: <hook_providers::$hook_provided::Interface as $crate::ComponentInterface>::schema(),
                                priority: 100,
                                required_authority: $crate::Authority::default(),
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

            pub fn dispatch_listener(
                event: &$crate::EventName,
                value: &$crate::PhenixValue,
            ) -> Result<bool, String> {
                $(
                    if event.as_str() == $listen_event {
                        let payload: $listener_ty =
                            $crate::__phenix_decode_listener!($projection, $listener_ty, value)
                                .map_err(|error| error.to_string())?;
                        $handler(payload).map_err(|error| error.to_string())?;
                        return Ok(true);
                    }
                )*
                Ok(false)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_core::ValueCodec;
    use std::collections::BTreeMap;

    #[derive(Clone, Debug, PartialEq, PhenixValue, PhenixContract)]
    #[phenix(id = "fixture.coverage@1")]
    struct Coverage {
        covered: u64,
        total: u64,
        label: String,
        previous: Option<u64>,
    }

    #[derive(Clone, Debug, PartialEq, PhenixValue)]
    struct CoverageProjection {
        covered: u64,
        label: String,
    }

    #[derive(Clone, Debug, PartialEq, PhenixValue)]
    struct NestedProjection {
        coverage: CoverageProjection,
    }

    #[derive(Clone, Debug, PartialEq, PhenixValue)]
    enum RunResult {
        Passed { output: String },
        Failed(String),
        Cancelled,
    }

    #[derive(Clone, Debug, PartialEq, PhenixValue)]
    enum ErrorVariant {
        Error,
        Value(String),
    }

    fn key(value: &str) -> Key {
        Key::parse(value).unwrap()
    }

    #[test]
    fn derived_native_type_round_trips_through_core_only_contract_value() {
        let coverage = Coverage {
            covered: 90,
            total: 100,
            label: "unit".into(),
            previous: None,
        };
        let value = coverage.to_contract_value().unwrap();

        assert_eq!(value.contract().id().as_str(), "fixture.coverage@1");
        assert_eq!(value.get("covered").unwrap().value::<u64>().unwrap(), 90);
        assert_eq!(Coverage::from_contract_value(&value).unwrap(), coverage);
    }

    #[test]
    fn exact_try_from_rejects_extra_fields() {
        let value = PhenixValue::Table(BTreeMap::from([
            (key("covered"), PhenixValue::U64(90)),
            (key("total"), PhenixValue::U64(100)),
            (key("label"), PhenixValue::String("unit".into())),
            (key("previous"), PhenixValue::Option(None)),
            (key("unexpected"), PhenixValue::Bool(true)),
        ]));

        assert_eq!(
            Coverage::try_from(Exact(&value)).unwrap_err(),
            ValueError::UnexpectedKey(key("unexpected"))
        );
    }

    #[test]
    fn same_derived_type_supports_exact_and_projected_conversion() {
        let value = Coverage {
            covered: 90,
            total: 100,
            label: "unit".into(),
            previous: None,
        }
        .to_value();

        assert!(matches!(
            CoverageProjection::try_from(Exact(&value)),
            Err(ValueError::UnexpectedKey(_))
        ));
        assert_eq!(
            CoverageProjection::try_from(Project(&value)).unwrap(),
            CoverageProjection {
                covered: 90,
                label: "unit".into(),
            }
        );

        let missing = PhenixValue::Table(BTreeMap::from([(key("covered"), PhenixValue::U64(90))]));
        assert_eq!(
            CoverageProjection::try_from(Project(&missing)).unwrap_err(),
            ValueError::MissingKey(key("label"))
        );
    }

    #[test]
    fn projection_applies_recursively() {
        let value = PhenixValue::Table(BTreeMap::from([
            (
                key("coverage"),
                Coverage {
                    covered: 90,
                    total: 100,
                    label: "unit".into(),
                    previous: None,
                }
                .to_value(),
            ),
            (key("outer_extra"), PhenixValue::Bool(true)),
        ]));

        assert_eq!(
            NestedProjection::try_from(Project(&value)).unwrap(),
            NestedProjection {
                coverage: CoverageProjection {
                    covered: 90,
                    label: "unit".into(),
                },
            }
        );
    }

    #[test]
    fn error_named_variant_does_not_collide_with_try_from_error() {
        let value = ErrorVariant::Error.to_value();
        assert_eq!(
            ErrorVariant::try_from(Exact(&value)).unwrap(),
            ErrorVariant::Error
        );
        let value = ErrorVariant::Value("ok".into()).to_value();
        assert_eq!(
            ErrorVariant::try_from(Project(&value)).unwrap(),
            ErrorVariant::Value("ok".into())
        );
    }

    #[test]
    fn derived_enum_has_a_tagged_structural_shape() {
        let result = RunResult::Passed {
            output: "ok".into(),
        };
        let value = result.to_value();
        let (tag, payload) = value.variant().unwrap();

        assert_eq!(tag.as_str(), "Passed");
        assert_eq!(
            payload.get("output").unwrap().value::<String>().unwrap(),
            "ok"
        );
        assert_eq!(RunResult::try_from(Exact(&value)).unwrap(), result);

        assert_eq!(
            RunResult::try_from(Exact(&RunResult::Failed("boom".into()).to_value())).unwrap(),
            RunResult::Failed("boom".into())
        );
        assert_eq!(
            RunResult::try_from(Exact(&RunResult::Cancelled.to_value())).unwrap(),
            RunResult::Cancelled
        );
    }
}
