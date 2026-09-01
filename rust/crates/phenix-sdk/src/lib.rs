#![forbid(unsafe_code)]

mod api;
mod authoring;
pub mod contracts;
mod providers;

pub use api::*;
pub use authoring::*;
pub use contracts::*;
pub use phenix_core::{
    Bytes, CallableRef, Contract, ContractId, ContractValue, Exact, Key, ObjectRef, PhenixContract,
    PhenixSchema, PhenixValue, PluginId, Project, ReferenceId, Type, TypeKind, ValueError,
};
pub use phenix_provider_sdk::{
    ApiTokenSource, Auth, AuthDescriptor, AuthKind, EnvironmentVariable, ProviderAuthCommand,
    ProviderAuthInterface, ProviderAuthResponse, ProviderError, RateLimits,
};
pub use phenix_sdk_macros::{PhenixContract, PhenixValue};
pub use providers::{Provider, ProviderSdkError, ProviderSdkExt, Providers};

#[cfg(test)]
pub(crate) use phenix_plugin_api::{sdk_component_manifest, sdk_factory, sdk_manifest};
#[cfg(test)]
pub(crate) use phenix_plugin_options_test::{
    options_component_manifest, options_factory, options_manifest,
};
#[cfg(test)]
pub(crate) use phenix_plugin_sessions_test::{
    session_component_manifest, session_factory, session_manifest,
};

pub mod auth {
    pub use phenix_provider_sdk::auth::*;
}

pub mod provider {
    pub use phenix_provider_sdk::provider::*;
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
