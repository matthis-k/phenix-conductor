#![forbid(unsafe_code)]

pub use phenix_core::session_service;
pub use phenix_plugin_artifacts::{
    artifact_component_id, artifact_component_manifest, artifact_factory, artifact_manifest,
    artifact_service, ArtifactCommand, ArtifactInterface, ArtifactProvenance, ArtifactRecord,
    ArtifactResponse, NormalizedReadRequest, ReadProviderIdentity, ReadResultRecord,
    RevalidationRecord, RevalidationVerdict, ARTIFACT_SERVICE,
};
pub use phenix_plugin_basic_agent::{
    basic_context_component_manifest, basic_context_factory, basic_context_manifest,
    basic_model_component_manifest, basic_model_factory, basic_model_manifest,
    basic_skills_component_manifest, basic_skills_factory, basic_skills_manifest,
    basic_tools_component_manifest, basic_tools_factory, basic_tools_manifest,
    BasicContextInterface, BasicModelInterface, BasicSkillsInterface, BasicToolsInterface,
    BASIC_CONTEXT_COMPONENT, BASIC_CONTEXT_PLUGIN, BASIC_MODEL_COMPONENT, BASIC_MODEL_PLUGIN,
    BASIC_SKILLS_COMPONENT, BASIC_SKILLS_PLUGIN, BASIC_TOOLS_COMPONENT, BASIC_TOOLS_PLUGIN,
};
pub use phenix_plugin_cli::{
    cli_auth_state_service, cli_component_id, cli_component_manifest, cli_discover_service,
    cli_factory, cli_manifest, cli_version_service, CliAuthState, CliAuthStateInterface,
    CliAvailability, CliDescriptor, CliDiscoverInterface, CliProbeRequest, CliVersionInterface,
    CLI_AUTH_STATE_SERVICE, CLI_DISCOVER_SERVICE, CLI_VERSION_SERVICE,
};
pub use phenix_plugin_context::{
    context_component_id, context_component_manifest, context_factory, context_manifest,
    context_service, ContextCommand, ContextDescriptor, ContextInjection, ContextInjectionLifetime,
    ContextInjectionRequester, ContextInterface, ContextResourceKind, ContextResourceRevision,
    ContextResponse, ContextScope, ExactContextReference, ExecutionContextProjection,
    ProjectedContextEntry, RepositoryContextSource, CONTEXT_SERVICE,
};
pub use phenix_plugin_debug::{
    debug_component_id, debug_component_manifest, debug_factory, debug_manifest, debug_service,
    DebugCommand, DebugInterface, DebugResponse, DiagnosticEntry, DiagnosticSnapshot,
    DEBUG_SERVICE,
};
pub use phenix_plugin_execution::{
    execution_component_id, execution_component_manifest, execution_configuration_service,
    execution_factory, execution_manifest, execution_service, AgentDefinition, CallablePolicy,
    CallableRecord, ExecutionAuthority, ExecutionCommand, ExecutionConfigurationCommand,
    ExecutionConfigurationResponse, ExecutionInterface, ExecutionRecord, ExecutionResponse,
    ExecutionState, OrchestrationDefinition, OrchestrationNode, WorkerTaskRecord, WorkerTaskState,
    EXECUTION_CONFIGURATION_SERVICE, EXECUTION_SERVICE,
};
pub use phenix_plugin_frontend::{
    frontend_component_id, frontend_component_manifest, frontend_factory, frontend_manifest,
    frontend_service, FrontendCommand, FrontendInterface, FrontendProviderDescriptor,
    FrontendResponse, FrontendServiceRequest, FrontendServiceResult, LiveFrontendProvider,
    FRONTEND_SERVICE,
};
pub use phenix_plugin_hooks::{
    hook_component_id, hook_component_manifest, hook_factory, hook_manifest, hook_service,
    HookAction, HookCommand, HookConfiguration, HookDefinition, HookDispatch, HookFailurePolicy,
    HookInterface, HookResponse, HookWarning, LifecycleEvent, HOOK_SERVICE,
};
pub use phenix_plugin_jobs::{
    job_component_id, job_component_manifest, job_factory, job_manifest, job_service, JobCommand,
    JobInterface, JobResponse, RuntimeResourceKind, RuntimeResourceRecord, RuntimeResourceState,
    JOB_SERVICE,
};
pub use phenix_plugin_language::{
    language_component_id, language_component_manifest, language_factory, language_manifest,
    language_service, DocumentProvenance, LanguageCommand, LanguageDocumentIdentity,
    LanguageInterface, LanguageObservation, LanguageOperationKind, LanguageOperationResult,
    LanguageProviderEpoch, LanguageResponse, LANGUAGE_SERVICE,
};
pub use phenix_plugin_models::{
    model_inference_service, model_routing_component_id, model_routing_component_manifest,
    model_routing_factory, model_routing_manifest, model_routing_service, ModelCommand,
    ModelInferenceRequest, ModelInferenceResponse, ModelResponse, ModelRoutingInterface,
    ModelTarget, RoutingProfile, RoutingProfileDescriptor, MODEL_INFERENCE_SERVICE,
    MODEL_ROUTING_SERVICE,
};
pub use phenix_plugin_options::{
    default_option_definitions, options_component_id, options_component_manifest, options_factory,
    options_manifest, options_service, OptionAssignment, OptionCommand, OptionContext,
    OptionDefinition, OptionKey, OptionResponse, OptionScope, OptionScopeKind,
    OptionStartupPrecedence, OptionSubjectId, OptionValue, OptionValueLayer, OptionValueSource,
    OptionsInterface, ResolvedOption, OPTIONS_COMPONENT, OPTIONS_PLUGIN, OPTIONS_SERVICE,
};
pub use phenix_plugin_planning::{
    planning_component_id, planning_component_manifest, planning_factory, planning_manifest,
    planning_service, DecisionRecord, HistoryEntry, HistoryKind, ObjectiveRecord, PlanRecord,
    PlanStep, PlanningCommand, PlanningInterface, PlanningResponse, PLANNING_SERVICE,
};
pub use phenix_plugin_repository_workers::{
    repository_work_queue_service, repository_worker_component_id,
    repository_worker_component_manifest, repository_worker_factory, repository_worker_manifest,
    ReconstructedPullRequest, RepositoryCheckState, RepositoryChecklistEvidence,
    RepositoryDiscussionEvidence, RepositoryDiscussionKind, RepositoryFinding,
    RepositoryIssueCluster, RepositoryIssueEvidence, RepositoryPullRequestEvidence,
    RepositoryPullRequestState, RepositorySelectionReason, RepositoryValidation,
    RepositoryWorkPriority, RepositoryWorkSelection, RepositoryWorkSnapshot,
    RepositoryWorkerInterface, RepositoryWorkerQueue, REPOSITORY_WORK_QUEUE_SERVICE,
};
pub use phenix_plugin_sdk::{
    sdk_component_id, sdk_component_manifest, sdk_config_service, sdk_contribution, sdk_factory,
    sdk_manifest, sdk_session_service, sdk_skills_service, sdk_tools_service, SdkConfigCommand,
    SdkConfigInterface, SdkConfigResponse, SdkSessionCommand, SdkSessionInterface,
    SdkSessionResponse, SdkSkill, SdkSkillCommand, SdkSkillResponse, SdkSkillSummary,
    SdkSkillsInterface, SdkTool, SdkToolCommand, SdkToolResponse, SdkToolsInterface, SDK_COMPONENT,
    SDK_CONFIG_SERVICE, SDK_PLUGIN, SDK_SESSION_SERVICE, SDK_SKILLS_SERVICE, SDK_TOOLS_SERVICE,
};
pub use phenix_plugin_session_tree::{
    session_tree_component_id, session_tree_component_manifest, session_tree_factory,
    session_tree_manifest, session_tree_service, SessionLineage, SessionTreeCommand,
    SessionTreeInterface, SessionTreeResponse, SESSION_TREE_SERVICE,
};
pub use phenix_plugin_sessions::{
    session_component_manifest, session_factory, session_manifest, SessionCommand, SessionInput,
    SessionInputKind, SessionInterface, SessionRecord, SessionResponse, SESSION_SERVICE,
};
pub use phenix_plugin_workspace::{
    workspace_component_id, workspace_component_manifest, workspace_factory, workspace_factory_for,
    workspace_manifest, workspace_service, WorkspaceCommand, WorkspaceFileVersion,
    WorkspaceInterface, WorkspaceResponse, WorkspaceSearchMatch, WORKSPACE_SERVICE,
};
