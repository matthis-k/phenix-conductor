#![forbid(unsafe_code)]

pub use phenix_plugin_artifacts::{
    artifact_factory, artifact_manifest, artifact_service, ArtifactCommand, ArtifactProvenance,
    ArtifactRecord, ArtifactResponse, NormalizedReadRequest, ReadProviderIdentity,
    ReadResultRecord, RevalidationRecord, RevalidationVerdict, ARTIFACT_SERVICE,
};
pub use phenix_plugin_cli::{
    cli_auth_state_service, cli_discover_service, cli_factory, cli_manifest, cli_version_service,
    CliAuthState, CliAvailability, CliDescriptor, CliProbeRequest, CLI_AUTH_STATE_SERVICE,
    CLI_DISCOVER_SERVICE, CLI_VERSION_SERVICE,
};
pub use phenix_plugin_context::{
    context_factory, context_manifest, context_service, ContextCommand, ContextDescriptor,
    ContextInjection, ContextInjectionLifetime, ContextInjectionRequester, ContextResourceKind,
    ContextResourceRevision, ContextResponse, ContextScope, ExactContextReference,
    ExecutionContextProjection, ProjectedContextEntry, RepositoryContextSource, CONTEXT_SERVICE,
};
pub use phenix_plugin_debug::{
    debug_factory, debug_manifest, debug_service, DebugCommand, DebugResponse, DiagnosticEntry,
    DiagnosticSnapshot, DEBUG_SERVICE,
};
pub use phenix_plugin_execution::{
    execution_configuration_service, execution_factory, execution_manifest, execution_service,
    AgentDefinition, CallablePolicy, CallableRecord, ExecutionAuthority, ExecutionCommand,
    ExecutionConfigurationCommand, ExecutionConfigurationResponse, ExecutionRecord,
    ExecutionResponse, ExecutionState, OrchestrationDefinition, OrchestrationNode,
    WorkerTaskRecord, WorkerTaskState, EXECUTION_CONFIGURATION_SERVICE, EXECUTION_SERVICE,
};
pub use phenix_plugin_frontend::{
    frontend_factory, frontend_manifest, frontend_service, FrontendCommand,
    FrontendProviderDescriptor, FrontendResponse, FrontendServiceRequest, FrontendServiceResult,
    LiveFrontendProvider, FRONTEND_SERVICE,
};
pub use phenix_plugin_hooks::{
    hook_factory, hook_manifest, hook_service, HookAction, HookCommand, HookConfiguration,
    HookDefinition, HookDispatch, HookFailurePolicy, HookResponse, HookWarning, LifecycleEvent,
    HOOK_SERVICE,
};
pub use phenix_plugin_jobs::{
    job_factory, job_manifest, job_service, JobCommand, JobResponse, RuntimeResourceKind,
    RuntimeResourceRecord, RuntimeResourceState, JOB_SERVICE,
};
pub use phenix_plugin_language::{
    language_factory, language_manifest, language_service, DocumentProvenance, LanguageCommand,
    LanguageDocumentIdentity, LanguageObservation, LanguageOperationKind, LanguageOperationResult,
    LanguageProviderEpoch, LanguageResponse, LANGUAGE_SERVICE,
};
pub use phenix_plugin_models::{
    model_inference_service, model_routing_factory, model_routing_manifest, model_routing_service,
    ModelCommand, ModelInferenceRequest, ModelInferenceResponse, ModelResponse, ModelTarget,
    RoutingProfile, RoutingProfileDescriptor, MODEL_INFERENCE_SERVICE, MODEL_ROUTING_SERVICE,
};
pub use phenix_plugin_planning::{
    planning_factory, planning_manifest, planning_service, DecisionRecord, HistoryEntry,
    HistoryKind, ObjectiveRecord, PlanRecord, PlanStep, PlanningCommand, PlanningResponse,
    PLANNING_SERVICE,
};
pub use phenix_plugin_repository_workers::{
    repository_work_queue_service, repository_worker_factory, repository_worker_manifest,
    ReconstructedPullRequest, RepositoryCheckState, RepositoryChecklistEvidence,
    RepositoryDiscussionEvidence, RepositoryDiscussionKind, RepositoryFinding,
    RepositoryIssueCluster, RepositoryIssueEvidence, RepositoryPullRequestEvidence,
    RepositoryPullRequestState, RepositorySelectionReason, RepositoryValidation,
    RepositoryWorkPriority, RepositoryWorkSelection, RepositoryWorkSnapshot, RepositoryWorkerQueue,
    REPOSITORY_WORK_QUEUE_SERVICE,
};
pub use phenix_plugin_sessions::{
    session_factory, session_manifest, session_service, SessionCommand, SessionRecord,
    SessionResponse, SESSION_SERVICE,
};
pub use phenix_plugin_workspace::{
    workspace_factory, workspace_factory_for, workspace_manifest, workspace_service,
    WorkspaceCommand, WorkspaceFileVersion, WorkspaceResponse, WorkspaceSearchMatch,
    WORKSPACE_SERVICE,
};
