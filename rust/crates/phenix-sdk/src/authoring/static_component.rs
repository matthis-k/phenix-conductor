use phenix_core::{ComponentId, EventTypeId, InterfaceId, PluginId};

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaticComponentLayer {
    pub interface: InterfaceId,
    pub method: &'static str,
    pub priority: i32,
}

impl StaticComponentLayer {
    #[must_use]
    pub fn of<I: InterfaceMarker>(method: &'static str, priority: i32) -> Self {
        Self {
            interface: I::interface_id(),
            method,
            priority,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaticComponentListener {
    pub event: EventTypeId,
    pub method: &'static str,
}

impl StaticComponentListener {
    #[must_use]
    pub fn new(event: &str, method: &'static str) -> Self {
        Self {
            event: EventTypeId::parse(event)
                .expect("component attribute validated the static event type"),
            method,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaticComponentValue {
    pub id: InterfaceId,
    pub method: &'static str,
    pub public: bool,
}

impl StaticComponentValue {
    #[must_use]
    pub fn new(id: &str, method: &'static str, public: bool) -> Self {
        Self {
            id: InterfaceId::parse(id).expect("component attribute validated the static value id"),
            method,
            public,
        }
    }
}

pub trait StaticComponentBehavior {
    fn exports() -> Vec<StaticComponentExport> {
        Vec::new()
    }

    fn layers() -> Vec<StaticComponentLayer> {
        Vec::new()
    }

    fn listeners() -> Vec<StaticComponentListener> {
        Vec::new()
    }

    fn values() -> Vec<StaticComponentValue> {
        Vec::new()
    }
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
        let id = ComponentId::parse(format!("{}.{field}", owner.as_str()))
            .expect("plugin id and Rust field name derive a valid component id");
        Self::new::<T>(id, field)
    }

    #[must_use]
    pub fn explicit<T: StaticComponentDefinition>(id: &str, field: &'static str) -> Self {
        let id =
            ComponentId::parse(id).expect("component attribute validated the static component id");
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
    struct Models;

    impl StaticComponentDefinition for Api {}

    impl InterfaceMarker for Models {
        fn interface_id() -> InterfaceId {
            InterfaceId::parse("fixture.models@1").unwrap()
        }
    }

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

    #[test]
    fn behavior_descriptors_preserve_semantic_identity_and_priority() {
        let layer = StaticComponentLayer::of::<Models>("policy", 17);
        let listener = StaticComponentListener::new("fixture.completed", "completed");
        let value = StaticComponentValue::new("fixture.status@1", "status", true);

        assert_eq!(layer.interface.as_str(), "fixture.models@1");
        assert_eq!(layer.priority, 17);
        assert_eq!(listener.event.as_str(), "fixture.completed");
        assert_eq!(value.id.as_str(), "fixture.status@1");
        assert!(value.public);
    }
}
