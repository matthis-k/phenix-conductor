from pathlib import Path

path = Path("rust/crates/phenix-plugin-suite/src/context.rs")
text = path.read_text()

if not text.startswith("use crate::{"):
    text = "use crate::{execution_factory, execution_manifest, execution_service, ExecutionAuthority, ExecutionCommand, ExecutionResponse, ExecutionState};\n" + text

old = '''fn load_context(
    host: &PluginHost<'_>,
    execution_id: String,
    resource_id: String,
    revision: String,
    requester: ContextInjectionRequester,
    lifetime: ContextInjectionLifetime,
    reason: String,
) -> Result<(ContextInjection, ContextResourceRevision), String> {
    validate_identity("execution id", &execution_id)?;
    validate_identity("context load reason", &reason)?;'''
new = '''fn load_context(
    host: &PluginHost<'_>,
    execution_id: String,
    resource_id: String,
    revision: String,
    requester: ContextInjectionRequester,
    lifetime: ContextInjectionLifetime,
    reason: String,
) -> Result<(ContextInjection, ContextResourceRevision), String> {
    validate_identity("execution id", &execution_id)?;
    validate_identity("context load reason", &reason)?;
    require_active_execution(host, &execution_id)?;'''
if old in text:
    text = text.replace(old, new, 1)
elif "require_active_execution(host, &execution_id)?;" not in text:
    raise SystemExit("load_context anchor missing")

anchor = '''fn project_context(
    host: &PluginHost<'_>,'''
helper = '''fn require_active_execution(host: &PluginHost<'_>, execution_id: &str) -> Result<(), String> {
    let command = ExecutionCommand::GetExecution {
        id: execution_id.to_owned(),
    };
    let output = host
        .invoke_service(
            &execution_service(),
            &serde_json::to_vec(&command).map_err(|error| error.to_string())?,
            host.authority(),
            None,
        )
        .map_err(|error| error.to_string())?;
    match serde_json::from_slice::<ExecutionResponse>(&output).map_err(|error| error.to_string())? {
        ExecutionResponse::ExecutionLookup {
            execution: Some(execution),
        } if execution.state == ExecutionState::Active => Ok(()),
        ExecutionResponse::ExecutionLookup {
            execution: Some(execution),
        } => Err(format!(
            "context target execution is not active: {execution_id} ({:?})",
            execution.state
        )),
        ExecutionResponse::ExecutionLookup { execution: None } => {
            Err(format!("unknown context target execution: {execution_id}"))
        }
        other => Err(format!("unexpected execution lookup response: {other:?}")),
    }
}

'''
if helper not in text:
    if anchor not in text:
        raise SystemExit("project_context anchor missing")
    text = text.replace(anchor, helper + anchor, 1)

old_kernel = '''    fn kernel_with(path: &PathBuf) -> Kernel {
        let manifest = context_manifest();
        let plugin = manifest.id.clone();
        let persistence = LocalPersistence::open(path).unwrap();
        let mut kernel =
            Kernel::with_persistence(KernelConfig::new([manifest]).unwrap(), persistence);
        kernel
            .register_embedded_factory(plugin.clone(), context_factory)
            .unwrap();
        kernel.activate_all().unwrap();
        assert_eq!(kernel.state(&plugin), Some(PluginState::Active));
        kernel
    }
'''
new_kernel = '''    fn kernel_with(path: &PathBuf) -> Kernel {
        let context_manifest = context_manifest();
        let context_plugin = context_manifest.id.clone();
        let execution_manifest = execution_manifest(authority());
        let execution_plugin = execution_manifest.id.clone();
        let persistence = LocalPersistence::open(path).unwrap();
        let mut kernel = Kernel::with_persistence(
            KernelConfig::new([execution_manifest, context_manifest]).unwrap(),
            persistence,
        );
        kernel
            .register_embedded_factory(execution_plugin.clone(), execution_factory)
            .unwrap();
        kernel
            .register_embedded_factory(context_plugin.clone(), context_factory)
            .unwrap();
        kernel.activate_all().unwrap();
        assert_eq!(kernel.state(&execution_plugin), Some(PluginState::Active));
        assert_eq!(kernel.state(&context_plugin), Some(PluginState::Active));
        kernel
    }

    fn create_execution(kernel: &mut Kernel, execution_id: &str) {
        let command = ExecutionCommand::CreateExecution {
            id: execution_id.to_owned(),
            requested_authority: ExecutionAuthority::new(Vec::<String>::new()),
        };
        kernel
            .invoke(
                &execution_service(),
                &serde_json::to_vec(&command).unwrap(),
                &authority(),
                None,
            )
            .unwrap();
    }
'''
if old_kernel in text:
    text = text.replace(old_kernel, new_kernel, 1)
elif "fn create_execution(kernel: &mut Kernel" not in text:
    raise SystemExit("context test kernel anchor missing")

load_anchor = '''            descriptor = match response {
                ContextResponse::Registered { resource } => resource.descriptor,
                other => panic!("unexpected response: {other:?}"),
            };
            invoke(
                &mut kernel,
                &ContextCommand::Load {'''
load_replacement = '''            descriptor = match response {
                ContextResponse::Registered { resource } => resource.descriptor,
                other => panic!("unexpected response: {other:?}"),
            };
            create_execution(&mut kernel, "exec-1");
            invoke(
                &mut kernel,
                &ContextCommand::Load {'''
if load_anchor in text:
    text = text.replace(load_anchor, load_replacement, 1)
elif 'create_execution(&mut kernel, "exec-1");' not in text:
    raise SystemExit("context load test anchor missing")

last = '''    #[test]
    fn context_and_skill_activation_require_no_unrelated_caller_authority() {'''
test = '''    #[test]
    fn context_load_rejects_unknown_or_finished_execution() {
        let path = temp_db("context-execution-provenance");
        let mut kernel = kernel_with(&path);
        let registered = invoke(
            &mut kernel,
            &ContextCommand::Register {
                resource_id: "skill:bounded".into(),
                kind: ContextResourceKind::Skill,
                source: "skills/bounded/SKILL.md".into(),
                scope: ContextScope::Workspace,
                content: b"bounded".to_vec(),
            },
        )
        .unwrap();
        let descriptor = match registered {
            ContextResponse::Registered { resource } => resource.descriptor,
            other => panic!("unexpected response: {other:?}"),
        };

        let unknown = invoke(
            &mut kernel,
            &ContextCommand::Load {
                execution_id: "missing".into(),
                resource_id: descriptor.resource_id.clone(),
                revision: descriptor.revision.clone(),
                requester: ContextInjectionRequester::User,
                lifetime: ContextInjectionLifetime::Execution,
                reason: "must be execution-bound".into(),
            },
        )
        .unwrap_err();
        assert!(unknown.contains("unknown context target execution"));

        create_execution(&mut kernel, "finished");
        kernel
            .invoke(
                &execution_service(),
                &serde_json::to_vec(&ExecutionCommand::FinishExecution {
                    id: "finished".into(),
                    success: true,
                })
                .unwrap(),
                &authority(),
                None,
            )
            .unwrap();
        let finished = invoke(
            &mut kernel,
            &ContextCommand::Load {
                execution_id: "finished".into(),
                resource_id: descriptor.resource_id,
                revision: descriptor.revision,
                requester: ContextInjectionRequester::User,
                lifetime: ContextInjectionLifetime::Execution,
                reason: "must still be active".into(),
            },
        )
        .unwrap_err();
        assert!(finished.contains("context target execution is not active"));
        let _ = fs::remove_file(path);
    }

'''
if test not in text:
    if last not in text:
        raise SystemExit("context final test anchor missing")
    text = text.replace(last, test + last, 1)

path.write_text(text)
