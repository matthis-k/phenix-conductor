use super::InterfaceMarker;
use phenix_core::InterfaceId;
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

pub trait StaticComponentImports {
    fn imports() -> Vec<StaticComponentImport>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Models;

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
}
