use phenix_core::{ComponentId, InterfaceId, PluginId};

pub trait InterfaceMarker {
    fn interface_id() -> InterfaceId;
}

pub trait StaticComponentDefinition {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaticComponentExport {
    pub interface: InterfaceId,
    pub method: &'static str,
    pub public: bool,
}

pub trait StaticComponentBehavior {
    fn exports() -> Vec<StaticComponentExport>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaticComponentDescriptor {
    pub id: ComponentId,
    pub field: &'static str,
    pub component_type: &'static str,
}

impl StaticComponentDescriptor {
    #[must_use]
    pub fn derived<T: StaticComponentDefinition>(owner: &PluginId, field: &'static str) -> Self {
        let id = ComponentId::parse(&format!("{}.{field}", owner.as_str()))
            .expect("plugin id and Rust field name derive a valid component id");
        Self::new::<T>(id, field)
    }

    #[must_use]
    pub fn explicit<T: StaticComponentDefinition>(id: &str, field: &'static str) -> Self {
        let id = ComponentId::parse(id).expect("component attribute validated the static component id");
        Self::new::<T>(id, field)
    }

    fn new<T: StaticComponentDefinition>(id: ComponentId, field: &'static str) -> Self {
        Self {
            id,
            field,
            component_type: std::any::type_name::<T>(),
        }
    }
}

pub trait StaticPluginComponents {
    fn components() -> Vec<StaticComponentDescriptor>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Api;

    impl StaticComponentDefinition for Api {}

    #[test]
    fn component_id_derives_from_owner_and_field() {
        let owner = PluginId::parse("fixture.component-owner").unwrap();
        let component = StaticComponentDescriptor::derived::<Api>(&owner, "api");

        assert_eq!(component.id.as_str(), "fixture.component-owner.api");
        assert_eq!(component.field, "api");
        assert!(component.component_type.ends_with("::Api"));
    }

    #[test]
    fn explicit_component_id_preserves_stable_identity() {
        let component = StaticComponentDescriptor::explicit::<Api>("legacy.component", "api");

        assert_eq!(component.id.as_str(), "legacy.component");
        assert_eq!(component.field, "api");
    }
}
