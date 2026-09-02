use phenix_core::{BackendFeature, DurableSchema, PluginId, ResourceNamespace};
use std::marker::PhantomData;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Durable<T> {
    marker: PhantomData<fn() -> T>,
}

impl<T> Durable<T> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

impl<T> Default for Durable<T> {
    fn default() -> Self {
        Self::new()
    }
}

pub trait StaticResourceField {
    type Resource: StaticResourceDefinition;
}

impl<T: StaticResourceDefinition> StaticResourceField for Durable<T> {
    type Resource = T;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaticResourceMigration {
    pub from_version: u32,
    pub to_version: u32,
    pub method: &'static str,
}

pub trait StaticResourceDefinition {
    fn schema_version() -> u32;

    fn migrations() -> Vec<StaticResourceMigration> {
        Vec::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaticResourceDescriptor {
    pub schema: DurableSchema,
    pub field: &'static str,
    pub resource_type: &'static str,
    pub migrations: Vec<StaticResourceMigration>,
}

impl StaticResourceDescriptor {
    #[must_use]
    pub fn derived<F: StaticResourceField>(
        owner: &PluginId,
        field: &'static str,
        features: impl IntoIterator<Item = BackendFeature>,
    ) -> Self {
        let namespace = ResourceNamespace::parse(&format!("{}.{field}", owner.as_str()))
            .expect("plugin id and Rust field name derive a valid resource namespace");
        Self::new::<F>(namespace, field, features)
    }

    #[must_use]
    pub fn explicit<F: StaticResourceField>(
        namespace: &str,
        field: &'static str,
        features: impl IntoIterator<Item = BackendFeature>,
    ) -> Self {
        let namespace = ResourceNamespace::parse(namespace)
            .expect("resource attribute validated the static resource namespace");
        Self::new::<F>(namespace, field, features)
    }

    fn new<F: StaticResourceField>(
        namespace: ResourceNamespace,
        field: &'static str,
        features: impl IntoIterator<Item = BackendFeature>,
    ) -> Self {
        let resource_type = std::any::type_name::<<F as StaticResourceField>::Resource>();
        let version = <F::Resource as StaticResourceDefinition>::schema_version();
        Self {
            schema: DurableSchema::requiring(namespace, version, features),
            field,
            resource_type,
            migrations: <F::Resource as StaticResourceDefinition>::migrations(),
        }
    }
}

pub trait StaticPluginResources {
    fn resources() -> Vec<StaticResourceDescriptor>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Store;

    impl StaticResourceDefinition for Store {
        fn schema_version() -> u32 {
            3
        }
    }

    #[test]
    fn resource_namespace_derives_from_owner_and_field() {
        let owner = PluginId::parse("fixture.resource-owner").unwrap();
        let resource = StaticResourceDescriptor::derived::<Durable<Store>>(
            &owner,
            "plans",
            [BackendFeature::Transactions],
        );

        assert_eq!(
            resource.schema.namespace.as_str(),
            "fixture.resource-owner.plans"
        );
        assert_eq!(resource.schema.version, 3);
        assert!(resource
            .schema
            .required_features
            .contains(&BackendFeature::Transactions));
        assert!(resource.resource_type.ends_with("::Store"));
    }
}
