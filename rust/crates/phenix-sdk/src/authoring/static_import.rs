use super::InterfaceMarker;
use phenix_core::{EventTypeId, InterfaceId};
use std::marker::PhantomData;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Call<I, Request, Response> {
    marker: PhantomData<fn(I, Request) -> Response>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Required<T> {
    marker: PhantomData<fn() -> T>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Optional<T> {
    marker: PhantomData<fn() -> T>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Host<I> {
    marker: PhantomData<fn() -> I>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Emit<T> {
    marker: PhantomData<fn() -> T>,
}

pub trait StaticImportField {
    fn interface_id() -> InterfaceId;

    fn required() -> bool;
}

impl<I, Request, Response> StaticImportField for Required<Call<I, Request, Response>>
where
    I: InterfaceMarker,
{
    fn interface_id() -> InterfaceId {
        I::interface_id()
    }

    fn required() -> bool {
        true
    }
}

impl<I, Request, Response> StaticImportField for Optional<Call<I, Request, Response>>
where
    I: InterfaceMarker,
{
    fn interface_id() -> InterfaceId {
        I::interface_id()
    }

    fn required() -> bool {
        false
    }
}

pub trait StaticHostField {
    fn interface_id() -> InterfaceId;
}

impl<I: InterfaceMarker> StaticHostField for Host<I> {
    fn interface_id() -> InterfaceId {
        I::interface_id()
    }
}

pub trait StaticEventField {
    fn payload_type() -> &'static str;
}

impl<T> StaticEventField for Emit<T> {
    fn payload_type() -> &'static str {
        std::any::type_name::<T>()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaticComponentImport {
    pub interface: InterfaceId,
    pub field: &'static str,
    pub required: bool,
}

impl StaticComponentImport {
    #[must_use]
    pub fn of<F: StaticImportField>(field: &'static str) -> Self {
        Self {
            interface: F::interface_id(),
            field,
            required: F::required(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaticComponentHost {
    pub interface: InterfaceId,
    pub field: &'static str,
}

impl StaticComponentHost {
    #[must_use]
    pub fn of<F: StaticHostField>(field: &'static str) -> Self {
        Self {
            interface: F::interface_id(),
            field,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaticComponentEvent {
    pub event: EventTypeId,
    pub field: &'static str,
    pub payload_type: &'static str,
}

impl StaticComponentEvent {
    #[must_use]
    pub fn of<F: StaticEventField>(event: &str, field: &'static str) -> Self {
        Self {
            event: EventTypeId::parse(event)
                .expect("component attribute validated the static event type"),
            field,
            payload_type: F::payload_type(),
        }
    }
}

pub trait StaticComponentImports {
    fn imports() -> Vec<StaticComponentImport>;

    fn hosts() -> Vec<StaticComponentHost> {
        Vec::new()
    }

    fn events() -> Vec<StaticComponentEvent> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Models;
    struct Completed;

    impl InterfaceMarker for Models {
        fn interface_id() -> InterfaceId {
            InterfaceId::parse("fixture.models@1").unwrap()
        }
    }

    #[test]
    fn required_and_optional_imports_preserve_interface_identity_and_optionality() {
        type RequiredModels = Required<Call<Models, String, String>>;
        type OptionalModels = Optional<Call<Models, String, String>>;

        let required = StaticComponentImport::of::<RequiredModels>("models");
        let optional = StaticComponentImport::of::<OptionalModels>("fallback_models");

        assert_eq!(required.interface.as_str(), "fixture.models@1");
        assert!(required.required);
        assert_eq!(optional.interface.as_str(), "fixture.models@1");
        assert!(!optional.required);
    }

    #[test]
    fn host_and_event_fields_preserve_capability_and_payload_identity() {
        let host = StaticComponentHost::of::<Host<Models>>("models_host");
        let event = StaticComponentEvent::of::<Emit<Completed>>("fixture.completed", "completed");

        assert_eq!(host.interface.as_str(), "fixture.models@1");
        assert_eq!(host.field, "models_host");
        assert_eq!(event.event.as_str(), "fixture.completed");
        assert_eq!(event.field, "completed");
        assert!(event.payload_type.ends_with("::Completed"));
    }
}
