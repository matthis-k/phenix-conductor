from pathlib import Path

models = Path("rust/crates/phenix-plugin-suite/src/models.rs")
text = models.read_text()
text = text.replace(
    '#[serde(tag = "response", rename_all = "snake_case")]\npub enum ModelResponse',
    '#[serde(tag = "kind", rename_all = "snake_case")]\npub enum ModelResponse',
)
old = '#[must_use]\npub fn model_routing_manifest() -> PluginManifest {\n    PluginManifest {'
new = '''#[must_use]
pub fn model_routing_manifest(maximum_authority: Authority) -> PluginManifest {
    let persistence = Authority::new([
        capability(PERSISTENCE_SCHEMA),
        capability(PERSISTENCE_READ),
        capability(PERSISTENCE_WRITE),
    ]);
    let maximum_authority = Authority::new(
        maximum_authority
            .capabilities()
            .cloned()
            .chain(persistence.capabilities().cloned()),
    );
    PluginManifest {'''
if old in text:
    text = text.replace(old, new, 1)
    text = text.replace(
        '''        maximum_authority: Authority::new([
            capability(PERSISTENCE_SCHEMA),
            capability(PERSISTENCE_READ),
            capability(PERSISTENCE_WRITE),
        ]),''',
        "        maximum_authority,",
        1,
    )
text = text.replace(
    "model_routing_manifest().maximum_authority",
    "model_routing_manifest(Authority::default()).maximum_authority",
)
text = text.replace(
    "let manifest = model_routing_manifest();",
    "let manifest = model_routing_manifest(Authority::default());",
)
models.write_text(text)

hooks = Path("rust/crates/phenix-plugin-suite/src/hooks.rs")
text = hooks.read_text().replace(
    "ContextInjectionRequester::Execution",
    "ContextInjectionRequester::Hook",
)
hooks.write_text(text)

lib = Path("rust/crates/phenix-plugin-suite/src/lib.rs")
text = lib.read_text()
modules = ["debug", "frontend", "hooks", "jobs", "models", "workspace"]
for module in modules:
    line = f"mod {module};\n"
    if line not in text:
        insert_after = {
            "debug": "mod context;\n",
            "frontend": "mod execution;\n",
            "hooks": "mod frontend;\n",
            "jobs": "mod hooks;\n",
            "models": "mod language;\n",
            "workspace": "mod sessions;\n",
        }[module]
        if insert_after not in text:
            raise SystemExit(f"missing module anchor for {module}")
        text = text.replace(insert_after, insert_after + line, 1)

if "pub use debug::" not in text:
    anchor = "pub use execution::{"
    text = text.replace(
        anchor,
        '''pub use debug::{
    debug_factory, debug_manifest, debug_service, DebugCommand, DebugResponse, DiagnosticEntry,
    DiagnosticSnapshot, DEBUG_SERVICE,
};
'''
        + anchor,
        1,
    )
if "pub use frontend::" not in text:
    anchor = "pub use language::{"
    text = text.replace(
        anchor,
        '''pub use frontend::{
    frontend_factory, frontend_manifest, frontend_service, FrontendCommand,
    FrontendProviderDescriptor, FrontendResponse, FrontendServiceRequest, FrontendServiceResult,
    LiveFrontendProvider, FRONTEND_SERVICE,
};
pub use hooks::{
    hook_factory, hook_manifest, hook_service, HookAction, HookCommand, HookConfiguration,
    HookDefinition, HookDispatch, HookFailurePolicy, HookResponse, HookWarning, LifecycleEvent,
    HOOK_SERVICE,
};
pub use jobs::{
    job_factory, job_manifest, job_service, JobCommand, JobResponse, RuntimeResourceKind,
    RuntimeResourceRecord, RuntimeResourceState, JOB_SERVICE,
};
'''
        + anchor,
        1,
    )
if "pub use models::" not in text:
    anchor = "pub use planning::{"
    text = text.replace(
        anchor,
        '''pub use models::{
    model_inference_service, model_routing_factory, model_routing_manifest, model_routing_service,
    ModelCommand, ModelInferenceRequest, ModelInferenceResponse, ModelResponse, ModelTarget,
    RoutingProfile, RoutingProfileDescriptor, MODEL_INFERENCE_SERVICE, MODEL_ROUTING_SERVICE,
};
'''
        + anchor,
        1,
    )
if "pub use workspace::" not in text:
    anchor = "\nuse phenix_kernel::{"
    text = text.replace(
        anchor,
        '''pub use workspace::{
    workspace_factory, workspace_factory_for, workspace_manifest, workspace_service,
    WorkspaceCommand, WorkspaceFileVersion, WorkspaceResponse, WorkspaceSearchMatch,
    WORKSPACE_SERVICE,
};

use phenix_kernel::{''',
        1,
    )
lib.write_text(text)

harness = Path("rust/crates/phenix-harness/src/lib.rs")
text = harness.read_text()
if "CapabilityId" not in text.split("};", 1)[0]:
    text = text.replace(
        "Authority, Kernel, KernelConfig, KernelError,",
        "Authority, CapabilityId, Kernel, KernelConfig, KernelError,",
        1,
    )
if "debug_factory" not in text.split("};", 2)[1]:
    old = '''use phenix_plugin_suite::{
    artifact_factory, artifact_manifest, context_factory, context_manifest, execution_factory,
    execution_manifest, language_factory, language_manifest, planning_factory, planning_manifest,
    repository_worker_factory, repository_worker_manifest, session_factory, session_manifest,
};'''
    new = '''use phenix_plugin_suite::{
    artifact_factory, artifact_manifest, context_factory, context_manifest, debug_factory,
    debug_manifest, execution_factory, execution_manifest, frontend_factory, frontend_manifest,
    hook_factory, hook_manifest, job_factory, job_manifest, language_factory, language_manifest,
    model_routing_factory, model_routing_manifest, planning_factory, planning_manifest,
    repository_worker_factory, repository_worker_manifest, session_factory, session_manifest,
    workspace_factory, workspace_manifest,
};'''
    if old not in text:
        raise SystemExit("harness suite import anchor missing")
    text = text.replace(old, new, 1)
marker = "type EmbeddedFactory = Arc<dyn Fn() -> Box<dyn PluginInstance> + Send + Sync>;\n"
if "fn default_suite_authority()" not in text:
    helper = marker + '''
fn default_suite_authority() -> Authority {
    Authority::new([
        CapabilityId::parse("kernel.persistence.schema").expect("static capability"),
        CapabilityId::parse("kernel.persistence.read").expect("static capability"),
        CapabilityId::parse("kernel.persistence.write").expect("static capability"),
        CapabilityId::parse("workspace.read").expect("static capability"),
        CapabilityId::parse("workspace.write").expect("static capability"),
        CapabilityId::parse("workspace.shell").expect("static capability"),
        CapabilityId::parse("workspace.git").expect("static capability"),
    ])
}
'''
    text = text.replace(marker, helper, 1)
old = '''        builder.add_embedded(repository_worker_manifest(), repository_worker_factory)?;
        builder.add_embedded(session_manifest(), session_factory)?;
        builder.add_embedded(artifact_manifest(), artifact_factory)?;
        builder.add_embedded(context_manifest(), context_factory)?;
        builder.add_embedded(execution_manifest(Authority::default()), execution_factory)?;
        builder.add_embedded(language_manifest(), language_factory)?;
        builder.add_embedded(planning_manifest(), planning_factory)?;
        Ok(builder)'''
new = '''        let authority = default_suite_authority();
        builder.add_embedded(repository_worker_manifest(), repository_worker_factory)?;
        builder.add_embedded(session_manifest(), session_factory)?;
        builder.add_embedded(artifact_manifest(), artifact_factory)?;
        builder.add_embedded(context_manifest(), context_factory)?;
        builder.add_embedded(execution_manifest(authority.clone()), execution_factory)?;
        builder.add_embedded(language_manifest(), language_factory)?;
        builder.add_embedded(planning_manifest(), planning_factory)?;
        builder.add_embedded(workspace_manifest(), workspace_factory)?;
        builder.add_embedded(model_routing_manifest(authority.clone()), model_routing_factory)?;
        builder.add_embedded(job_manifest(), job_factory)?;
        builder.add_embedded(frontend_manifest(authority.clone()), frontend_factory)?;
        builder.add_embedded(hook_manifest(authority.clone()), hook_factory)?;
        builder.add_embedded(debug_manifest(authority), debug_factory)?;
        Ok(builder)'''
if old in text:
    text = text.replace(old, new, 1)
elif "builder.add_embedded(debug_manifest" not in text:
    raise SystemExit("default suite composition marker missing")
text = text.replace("manifests().count(), 7", "manifests().count(), 13")
harness.write_text(text)
