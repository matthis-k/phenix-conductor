//! Generic Phenix kernel mechanisms.
//!
//! This crate owns fundamental Phenix primitives and simple host mechanisms. Rich
//! behavior, policy, discovery, management, and product semantics belong to plugins.

mod activation;
mod agent;
mod authority;
mod component;
mod composition_metadata;
mod configuration;
#[cfg(test)]
mod configuration_regression;
mod contract;
mod contract_wire;
mod events;
mod frontend_metadata;
mod identity;
mod inspection;
mod live_reconciliation;
mod manifest;
mod metadata_input;
mod metadata_inspection;
mod metadata_reconciliation;
mod persistence;
mod persistence_value;
mod plugin_context;
mod reconciliation;
mod reconciliation_inspection;
mod registry;
mod resolver;
mod runtime;
mod sdk;
mod std_value;
mod structural_value;
mod tasks;
mod typed_component;

extern crate self as phenix_core;
#[cfg(test)]
mod component_endpoint_regression;
#[cfg(test)]
mod host_authority_regression;
#[cfg(test)]
mod invalid_candidate_activation_regression;
#[cfg(test)]
mod layer_regression;
#[cfg(test)]
mod metadata_semantic_identity_regression;
#[cfg(test)]
#[path = "../tests/persistence_backend_conformance.rs"]
mod persistence_backend_conformance;
#[cfg(test)]
mod provider_rebind_generation_regression;
#[cfg(test)]
mod runtime_component_parity_regression;
#[cfg(test)]
mod runtime_provider_regression;
#[cfg(test)]
mod service_layer_dispatch_regression;
#[cfg(test)]
mod third_party_component_regression;

pub use activation::{
    ActiveResolvedGraph, ResolvedHarnessActivation, ResolvedHarnessActivationError,
};
pub use agent::{
    context_service, model_inference_service, skill_service, tool_service, ContextCommand,
    ContextDescriptor, ContextResourceKind, ContextResourceRevision, ContextResponse, ContextScope,
    ModelInferenceInterface, ModelInferenceRequest, ModelInferenceResponse, SkillCommand,
    SkillDefinition, SkillResponse, ToolCommand, ToolDefinition, ToolResponse, CONTEXT_SERVICE,
    MODEL_INFERENCE_SERVICE, SKILL_SERVICE, TOOL_SERVICE,
};
pub use authority::Authority;
pub use component::{
    ComponentGraphError, ResolvedComponent, ResolvedComponentGraph, ResolvedImport,
    ResolvedImportHandle,
};
pub use composition_metadata::{
    CompatibilityMetadata, ComponentHostKind, ComponentRuntimeMetadata, ComponentStateClass,
    CompositionMetadataError, DurableMigrationMetadata, PluginPackageMetadata, ReloadPolicy,
    SkillResourceMetadata,
};
pub use configuration::{
    ConfigContribution, ConfigContributionSource, ConfigMergeError, ConfigNamespace,
    ConfigSourceClass, ConfigurationFrontendMetadata, FrontendConfigContribution,
    FrontendConfigError, ResolvedConfigContribution, ResolvedConfigContributions,
};
pub use contract::{
    Bytes, CallableRef, Contract, ContractId, ContractValue, Exact, HasPhenixSchema, Key,
    ObjectRef, PhenixContract, PhenixSchema, PhenixValue, Project, ReferenceId,
    SchemaCompatibility, SchemaMismatch, Type, TypeKind, ValueCodec, ValueError, ValueMatch,
};
pub use events::{
    EventBus, EventDispatchReport, EventEnvelope, EventError, EventFailurePolicy, EventHandler,
    EventSubscription, KernelEvent, SubscriptionSpec,
};
pub use frontend_metadata::FrontendMetadataResolutionError;
pub use identity::{
    CallableId, CapabilityId, ComponentId, ConfigurationFrontendId, ContextResourceId,
    ContextRevisionId, EventTypeId, InterfaceId, ModelId, PluginId, ResourceNamespace,
    RoutingProfileId, RuntimeId, SdkNamespace, SdkResourceId, ServiceId, SessionId, SkillId,
    SubscriptionId,
};
pub use inspection::ResolvedHarnessInspection;
pub use live_reconciliation::LiveReconciliationError;
pub use manifest::{
    ComponentExport, ComponentImport, ComponentManifest, PluginArtifact, PluginExecution,
    PluginManifest, ServiceContribution, ServiceRole,
};
pub use metadata_input::{CompositionMetadataInput, MetadataResolutionError};
pub use metadata_inspection::ResolvedCompositionMetadata;
pub use metadata_reconciliation::{
    ComponentMetadataChange, CompositionMetadataDiff, FrontendMetadataChange, MetadataChangeKind,
    MetadataReconciliationError, MetadataReconciliationPreview, PackageMetadataChange,
    ResourceMetadataChange,
};
pub use persistence::{
    BackendFeature, DurableSchema, LocalPersistence, NamespaceTransaction, PersistenceBackend,
    PersistenceError, SchemaMigration, TransactionOp,
};
pub use plugin_context::{
    CallContext, CurrentPlugin, KernelAccess, PluginContext, SdkClient, SdkContract, SdkObject,
};
pub use reconciliation::{
    BindingChange, ComponentChange, ComponentChangeKind, GraphDiff, GraphReconciler,
    ReconciliationAction, ReconciliationPreview, ReconciliationResult, ResourceChange,
    ResourceChangeKind,
};
pub use reconciliation_inspection::CandidateResolutionInspection;
pub use registry::{
    runtime_provider_runtime, runtime_provider_service, KernelConfig, KernelError,
    KernelPolicyIdentity, LayerPolicy, ProviderBinding, ResolvedServiceChain, RuntimeBinding,
    EMBEDDED_RUNTIME, RUNTIME_PROVIDER_SERVICE_PREFIX,
};
pub use resolver::{GraphGenerationId, ResolvedHarness, ResolvedHarnessError};
pub use runtime::{
    Kernel, LayerResult, PluginHost, PluginInstance, PluginRuntimeProvider, PluginState,
    RuntimePluginCandidate, ServiceInvocationProvenance, ServiceParticipantOutcome,
    ServiceParticipantProvenance,
};
pub use sdk::{ResolvedSdkContributions, SdkContribution, SdkResolutionError};
pub use tasks::{CancellationToken, TaskHandle, TaskRuntime, TaskScope};
pub use typed_component::{
    ComponentInterface, ComponentInvocationError, InterfaceCompatibility, InterfaceSchema,
    InterfaceSchemaMismatch,
};

/// Rule used by a configuration frontend for source identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceIdentityRule {
    RequiredNonEmpty,
}

/// Rule used for a configuration frontend's source revision identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceRevisionRule {
    RequiredExact,
}

/// Rule used when stable configuration is materialized before activation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StableMaterializationRule {
    MaterializedOnly,
}

impl ConfigurationFrontendMetadata {
    /// Stable configuration must identify its source explicitly.
    pub const fn source_identity_rule(&self) -> SourceIdentityRule {
        SourceIdentityRule::RequiredNonEmpty
    }

    /// Stable configuration must carry an exact source revision identity.
    pub const fn source_revision_rule(&self) -> SourceRevisionRule {
        SourceRevisionRule::RequiredExact
    }

    /// Environment bindings cannot alter the stable semantic configuration.
    pub const fn stable_materialization_rule(&self) -> StableMaterializationRule {
        StableMaterializationRule::MaterializedOnly
    }
}
