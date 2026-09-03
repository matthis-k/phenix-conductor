use phenix_core::{PhenixSchema, ValueCodec};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaticPluginConfigDescriptor {
    pub field: &'static str,
    pub config_type: &'static str,
    pub schema: PhenixSchema,
}

impl StaticPluginConfigDescriptor {
    #[must_use]
    pub fn of<T: ValueCodec>(field: &'static str) -> Self {
        Self {
            field,
            config_type: std::any::type_name::<T>(),
            schema: T::phenix_type(),
        }
    }
}

pub trait StaticPluginConfiguration {
    fn configuration() -> Option<StaticPluginConfigDescriptor>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, Eq, PartialEq, phenix_sdk_macros::PhenixValue)]
    struct Settings {
        retries: u64,
    }

    #[test]
    fn typed_configuration_exposes_its_structural_schema() {
        let descriptor = StaticPluginConfigDescriptor::of::<Settings>("config");

        assert_eq!(descriptor.field, "config");
        assert!(descriptor.config_type.ends_with("::Settings"));
        assert!(matches!(descriptor.schema, PhenixSchema::Table(_)));
    }
}
