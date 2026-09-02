mod context;
mod plugin;
mod static_component;
mod static_resource;

pub use context::*;
pub use plugin::{
    EventEmitError, EventEmitter, EventName, StaticPluginDefinition, StaticPluginDependency,
    StaticPluginDescriptor, StaticPluginGraph, StaticPluginGraphError, TypedSdkClient,
};
pub use static_component::{
    InterfaceMarker, StaticComponentBehavior, StaticComponentDefinition, StaticComponentDescriptor,
    StaticComponentExport, StaticPluginComponents,
};
pub use static_resource::{
    Durable, StaticPluginResources, StaticResourceDefinition, StaticResourceDescriptor,
    StaticResourceField, StaticResourceMigration,
};

impl std::fmt::Debug for StaticPluginGraph {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_list().entries(self.ids()).finish()
    }
}

#[doc(hidden)]
pub use plugin::{
    __phenix_plugin, dispatch_exact_provider, dispatch_projected_provider, listener_subscription,
    HookName, ListenerDeclaration, ListenerProjection,
};
