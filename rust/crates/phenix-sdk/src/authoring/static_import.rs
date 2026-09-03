use super::InterfaceMarker;
use phenix_core::{
    Authority, EventTypeId, HasPhenixSchema, InterfaceId, InterfaceSchema, PhenixSchema,
};
use std::marker::PhantomData;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Call<I, Request, Response> {
    marker: PhantomData<fn(I, Request) -> Response>,
}

impl<I, Request, Response> Default for Call<I, Request, Response> {
    fn default() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Required<T> {
    marker: PhantomData<fn() -> T>,
}

impl<T> Default for Required<T> {
    fn default() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Optional<T> {
    marker: PhantomData<fn() -> T>,
}

impl<T> Default for Optional<T> {
    fn default() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Host<I> {
    marker: PhantomData<fn() -> I>,
}

impl<I> Default for Host<I> {
    fn default() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Emit<T> {
    marker: PhantomData<fn() -> T>,
}

impl<T> Default for Emit<T> {
    fn default() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

pub trait StaticImportField {
    fn interface_id() -> InterfaceId;

    fn schema() -> InterfaceSchema;

    fn required() -> bool;
}

impl<I, Request, Response> StaticImportField for Required<Call<I, Request, Response>>
where
    I: InterfaceMarker,
    Request: HasPhenixSchema,
    Response: HasPhenixSchema,
{
    fn interface_id() -> InterfaceId {
        I::interface_id()
    }

    fn schema() -> InterfaceSchema {
        InterfaceSchema::of::<Request, Response>()
    }

    fn required() -> bool {
        true
    }
}

impl<I, Request, Response> StaticImportField for Optional<Call<I, Request, Response>>
where
    I: InterfaceMarker,
    Request: HasPhenixSchema,
    Response: HasPhenixSchema,
{
    fn interface_id() -> InterfaceId {
        I::interface_id()
    }

    fn schema() -> InterfaceSchema {
        InterfaceSchema::of::<Request, Response>()
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

    fn payload_schema() -> PhenixSchema;
}

impl<T: HasPhenixSchema> StaticEventField for Emit<T> {
    fn payload_type() -> &'static str {
        std::any::type_name::<T>()
    }

    fn payload_schema() -> PhenixSchema {
        T::phenix_schema()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaticComponentImport {
    pub interface: InterfaceId,
    pub schema: InterfaceSchema,
    pub field: &'static str,
    pub required: bool,
    pub authority: Authority,
}

impl StaticComponentImport {
    #[must_use]
    pub fn of<F: StaticImportField>(field: &'static str) -> Self {
        Self::with_authority::<F>(field, Authority::default())
    }

    #[must_use]
    pub fn with_authority<F: StaticImportField>(field: &'static str, authority: Authority) -> Self {
        Self {
            interface: F::interface_id(),
            schema: F::schema(),
            field,
            required: F::required(),
            authority,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaticComponentHost {
    pub interface: InterfaceId,
    pub field: &'static str,
    pub authority: Authority,
}

impl StaticComponentHost {
    #[must_use]
    pub fn of<F: StaticHostField>(field: &'static str) -> Self {
        Self::with_authority::<F>(field, Authority::default())
    }

    #[must_use]
    pub fn with_authority<F: StaticHostField>(field: &'static str, authority: Authority) -> Self {
        Self {
            interface: F::interface_id(),
            field,
            authority,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaticComponentEvent {
    pub event: EventTypeId,
    pub field: &'static str,
    pub payload_type: &'static str,
    pub payload_schema: PhenixSchema,
}

impl StaticComponentEvent {
    #[must_use]
    pub fn of<F: StaticEventField>(event: &str, field: &'static str) -> Self {
        Self {
            event: EventTypeId::parse(event)
                .expect("component attribute validated the static event type"),
            field,
            payload_type: F::payload_type(),
            payload_schema: F::payload_schema(),
        }
    }
}

pub trait StaticComponentImports {
    fn imports() -> Vec<StaticComponentImport> {
        Vec::new()
    }

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
    #[derive(crate::PhenixValue)]
    struct Completed;

    impl InterfaceMarker for Models {
        fn interface_id() -> InterfaceId {
            InterfaceId::parse("fixture.models@1").unwrap()
        }
    }

    #[test]
    fn marker_defaults_do_not_require_payload_defaults() {
        let _: Call<Models, Completed, Completed> = Call::default();
        let _: Required<Call<Models, Completed, Completed>> = Required::default();
        let _: Optional<Call<Models, Completed, Completed>> = Optional::default();
        let _: Host<Models> = Host::default();
        let _: Emit<Completed> = Emit::default();
    }

    #[test]
    fn required_and_optional_imports_preserve_interface_identity_and_optionality() {
        type RequiredModels = Required<Call<Models, String, String>>;
        type OptionalModels = Optional<Call<Models, String, String>>;

        let required = StaticComponentImport::of::<RequiredModels>("models");
        let optional = StaticComponentImport::of::<OptionalModels>("fallback_models");

        assert_eq!(required.interface.as_str(), "fixture.models@1");
        assert!(required.required);
        assert_eq!(required.schema, InterfaceSchema::of::<String, String>());
        assert_eq!(optional.interface.as_str(), "fixture.models@1");
        assert!(!optional.required);
        assert_eq!(optional.schema, InterfaceSchema::of::<String, String>());
    }

    #[test]
    fn host_and_event_fields_preserve_capability_and_payload_identity() {
        let host = StaticComponentHost::of::<Host<Models>>("models_host");
        let authority = Authority::new([phenix_core::CapabilityId::parse("clock.read").unwrap()]);
        let authorized_host = StaticComponentHost::with_authority::<Host<Models>>(
            "authorized_models_host",
            authority.clone(),
        );
        let event = StaticComponentEvent::of::<Emit<Completed>>("fixture.completed", "completed");

        assert_eq!(host.interface.as_str(), "fixture.models@1");
        assert_eq!(host.field, "models_host");
        assert_eq!(host.authority, Authority::default());
        assert_eq!(authorized_host.interface.as_str(), "fixture.models@1");
        assert_eq!(authorized_host.field, "authorized_models_host");
        assert_eq!(authorized_host.authority, authority);
        assert_eq!(event.event.as_str(), "fixture.completed");
        assert_eq!(event.field, "completed");
        assert!(event.payload_type.ends_with("::Completed"));
        assert_eq!(event.payload_schema, Completed::phenix_schema());
    }
}
