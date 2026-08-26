//! Generic Phenix kernel mechanisms.
//!
//! Agent-domain semantics belong to userspace plugins. This crate owns only plugin
//! identity, authority, contribution resolution, lifecycle, events, namespaced
//! resources, persistence mechanics, and blocking task mechanics.

mod authority;
mod events;
mod external;
mod identity;
mod manifest;
mod persistence;
mod registry;
mod runtime;
mod tasks;

pub use authority::Authority;
pub use events::{
    EventBus, EventDispatchReport, EventEnvelope, EventError, EventFailurePolicy, EventHandler,
    EventSubscription, KernelEvent, SubscriptionSpec,
};
pub use external::{
    ExternalPluginProcess, ExternalSandbox, ExternalTransportConfig, ExternalTransportError,
    EXTERNAL_PROTOCOL_VERSION,
};
pub use identity::{
    CapabilityId, EventTypeId, PluginId, ResourceNamespace, ServiceId, SubscriptionId,
};
pub use manifest::{PluginExecution, PluginManifest, ServiceContribution};
pub use persistence::{
    BackendFeature, DurableSchema, LocalPersistence, NamespaceTransaction, PersistenceBackend,
    PersistenceError, SchemaMigration, TransactionOp,
};
pub use registry::{KernelConfig, KernelError, ProviderBinding};
pub use runtime::{Kernel, PluginHost, PluginInstance, PluginState};
pub use tasks::{CancellationToken, TaskHandle, TaskRuntime};
