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
mod events;
mod external;
mod frontend_metadata;
mod identity;
mod inspection;
mod live_reconciliation;
mod manifest;
mod metadata_input;
mod metadata_inspection;
mod metadata_reconciliation;
mod persistence;
mod reconciliation;
mod reconciliation_inspection;
mod registry;
mod resolver;
mod runtime;
mod tasks;
mod typed_component;

#[cfg(test)]
extern crate self as phenix_core;
#[cfg(test)]
mod component_endpoint_regression;
#[cfg(test)]
mod component_host_parity_regression;
#[cfg(test)]
mod external_layer_conformance_regression;
#[cfg(test)]
mod external_typed_import_runtime_regression;
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
mod service_layer_dispatch_regression;
#[cfg(test)]
mod third_party_component_regression;

pub use activation::{
    ActiveResolvedGraph, ResolvedHarnessActivation, ResolvedHarnessActivationError,
};
pub use agent::{
    context_service, model_inference_service, session_service, skill_service, tool_service,
    ContextCommand, ContextDescriptor, ContextResourceKind, ContextResourceRevision,
    ContextResponse, ContextScope, ModelInferenceRequest, ModelInferenceResponse, SessionCommand,
    SessionInput, SessionInputKind, SessionRecord, SessionResponse, SkillCommand, SkillDefinition,
    SkillResponse, ToolCommand, ToolDefinition, ToolResponse, CONTEXT_SERVICE,
    MODEL_INFERENCE_SERVICE, SESSION_SERVICE, SKILL_SERVICE, TOOL_SERVICE,
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
pub use events::{
    EventBus, EventDispatchReport, EventEnvelope, EventError, EventFailurePolicy, EventHandler,
    EventSubscription, KernelEvent, SubscriptionSpec,
};
pub use external::{
    ExternalPluginProcess, ExternalSandbox, ExternalTransportConfig, ExternalTransportError,
    EXTERNAL_PROTOCOL_VERSION,
};
pub use frontend_metadata::FrontendMetadataResolutionError;
pub use identity::{
    CapabilityId, ComponentId, ConfigurationFrontendId, EventTypeId, InterfaceId, PluginId,
    ResourceNamespace, ServiceId, SubscriptionId,
};
pub use inspection::ResolvedHarnessInspection;
pub use live_reconciliation::LiveReconciliationError;
pub use manifest::{
    ComponentExport, ComponentImport, ComponentManifest, PluginExecution, PluginManifest,
    ServiceContribution, ServiceRole,
};
pub use metadata_input::{CompositionMetadataInput, MetadataResolutionError};
pub use metadata_inspection::ResolvedCompositionMetadata;
pub use metadata_reconciliation::{
    ComponentMetadataChange, CompositionMetadataDiff, FrontendMetadataChange, MetadataChangeKind,
    MetadataReconciliationError, MetadataReconciliationPreview, PackageMetadataChange,
};
pub use persistence::{
    BackendFeature, DurableSchema, LocalPersistence, NamespaceTransaction, PersistenceBackend,
    PersistenceError, SchemaMigration, TransactionOp,
};
pub use reconciliation::{
    BindingChange, ComponentChange, ComponentChangeKind, GraphDiff, GraphReconciler,
    ReconciliationAction, ReconciliationPreview, ReconciliationResult, ResourceChange,
    ResourceChangeKind,
};
pub use reconciliation_inspection::CandidateResolutionInspection;
pub use registry::{
    KernelConfig, KernelError, KernelPolicyIdentity, LayerPolicy, ProviderBinding,
    ResolvedServiceChain,
};
pub use resolver::{GraphGenerationId, ResolvedHarness, ResolvedHarnessError};
pub use runtime::{
    Kernel, LayerResult, PluginHost, PluginInstance, PluginState, ServiceInvocationProvenance,
    ServiceParticipantOutcome, ServiceParticipantProvenance,
};
pub use tasks::{CancellationToken, TaskHandle, TaskRuntime};
pub use typed_component::{ComponentInterface, ComponentInvocationError};

/// Rule used by a configuration frontend for source identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceIdentityRule {
    RequiredNonEmpty,
}

/// Rule used by a configuration frontend for source revision identity.
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

    /// Stable configuration must carry an exact source revision.
    pub const fn source_revision_rule(&self) -> SourceRevisionRule {
        SourceRevisionRule::RequiredExact
    }

    /// Environment bindings cannot alter the stable semantic configuration.
    pub const fn stable_materialization_rule(&self) -> StableMaterializationRule {
        StableMaterializationRule::MaterializedOnly
    }
}

#[cfg(test)]
mod host_component_import_regression;

#[cfg(test)]
mod configuration_frontend_metadata_contract {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn frontend_metadata_exposes_source_and_stable_materialization_rules() {
        let metadata = ConfigurationFrontendMetadata {
            id: ConfigurationFrontendId::parse("fixture-config").unwrap(),
            version: 1,
            accepted_source_kinds: BTreeSet::from(["fixture".into()]),
            exposed_namespaces: BTreeSet::from([
                ConfigNamespace::parse("fixture.policy@1").unwrap()
            ]),
            watch: true,
            required_authority: Authority::default(),
        };

        assert_eq!(
            metadata.source_identity_rule(),
            SourceIdentityRule::RequiredNonEmpty
        );
        assert_eq!(
            metadata.source_revision_rule(),
            SourceRevisionRule::RequiredExact
        );
        assert_eq!(
            metadata.stable_materialization_rule(),
            StableMaterializationRule::MaterializedOnly
        );
    }
}
