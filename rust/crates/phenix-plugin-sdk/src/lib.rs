#![forbid(unsafe_code)]

mod authoring;
pub use authoring::*;

use phenix_core::{
    Authority, ComponentExport, ComponentId, ComponentImport, ComponentInterface,
    ComponentManifest, InterfaceId, PluginExecution, PluginHost, PluginId, PluginInstance,
    PluginManifest, SdkContribution, SdkNamespace, SdkResourceId, ServiceContribution, ServiceId,
    ServiceRole,
};
use phenix_plugin_context::{
    ContextCommand, ContextDescriptor, ContextInterface, ContextResourceKind,
    ContextResourceRevision, ContextResponse, ContextScope,
};
use phenix_plugin_execution::{
    ExecutionAuthority, ExecutionCommand, ExecutionInterface, ExecutionResponse,
};
use phenix_plugin_models::ModelRoutingInterface;
use phenix_plugin_options::{
    OptionCommand, OptionContext, OptionKey, OptionResponse, OptionSubjectId, OptionValue,
    OptionsInterface,
};
use phenix_plugin_sessions::{SessionCommand, SessionInterface, SessionRecord, SessionResponse};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    env, fs,
    path::{Component, Path, PathBuf},
};

pub const SDK_PLUGIN: &str = "phenix.sdk";
pub const SDK_COMPONENT: &str = "phenix.sdk";
pub const SDK_SESSION_SERVICE: &str = "phenix.sdk.sessions@1";
pub const SDK_TOOLS_SERVICE: &str = "phenix.sdk.tools@1";
pub const SDK_SKILLS_SERVICE: &str = "phenix.sdk.skills@1";
pub const SDK_CONFIG_SERVICE: &str = "phenix.sdk.config@1";

struct SdkDependencies<'host, 'runtime> {
    sessions: SdkClient<'host, 'runtime, SessionInterface>,
    options: SdkClient<'host, 'runtime, OptionsInterface>,
    execution: SdkClient<'host, 'runtime, ExecutionInterface>,
    context: SdkClient<'host, 'runtime, ContextInterface>,
}

type SdkRuntimeContext<'host, 'runtime, 'plugin> =
    PluginContext<'host, 'runtime, SdkDependencies<'host, 'runtime>, &'plugin Option<PathBuf>>;

fn plugin_context<'host, 'runtime, 'plugin>(
    host: &'host PluginHost<'runtime>,
    config_root: &'plugin Option<PathBuf>,
) -> SdkRuntimeContext<'host, 'runtime, 'plugin> {
    let component = sdk_component_id();
    PluginContext::new(
        host,
        SdkDependencies {
            sessions: SdkClient::new(host, component.clone()),
            options: SdkClient::new(host, component.clone()),
            execution: SdkClient::new(host, component.clone()),
            context: SdkClient::new(host, component),
        },
        config_root,
        (),
    )
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum SdkSessionCommand {
    Open {
        id: String,
        #[serde(default)]
        agent: Option<String>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "result", rename_all = "snake_case", deny_unknown_fields)]
pub enum SdkSessionResponse {
    Opened {
        session: SessionRecord,
        created: bool,
    },
}

pub struct SdkSessionInterface;

impl ComponentInterface for SdkSessionInterface {
    type Request = SdkSessionCommand;
    type Response = SdkSessionResponse;

    fn interface_id() -> InterfaceId {
        InterfaceId::parse(SDK_SESSION_SERVICE).expect("static SDK session interface id is valid")
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SdkTool {
    pub id: String,
    pub service: String,
    pub required_capabilities: BTreeSet<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum SdkToolCommand {
    Register {
        id: String,
        service: String,
        #[serde(default)]
        required_capabilities: BTreeSet<String>,
    },
    Invoke {
        execution_id: String,
        id: String,
        input: Vec<u8>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "result", rename_all = "snake_case", deny_unknown_fields)]
pub enum SdkToolResponse {
    Tool { tool: SdkTool },
    Output { output: Vec<u8> },
}

pub struct SdkToolsInterface;

impl ComponentInterface for SdkToolsInterface {
    type Request = SdkToolCommand;
    type Response = SdkToolResponse;

    fn interface_id() -> InterfaceId {
        InterfaceId::parse(SDK_TOOLS_SERVICE).expect("static SDK tools interface id is valid")
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SdkSkill {
    pub id: String,
    pub content: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SdkSkillSummary {
    pub id: String,
    pub revision: String,
    pub source: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum SdkSkillCommand {
    Register { id: String, content: Vec<u8> },
    Get { id: String },
    List,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "result", rename_all = "snake_case", deny_unknown_fields)]
pub enum SdkSkillResponse {
    Skill { skill: Option<SdkSkill> },
    Skills { skills: Vec<SdkSkillSummary> },
}

pub struct SdkSkillsInterface;

impl ComponentInterface for SdkSkillsInterface {
    type Request = SdkSkillCommand;
    type Response = SdkSkillResponse;

    fn interface_id() -> InterfaceId {
        InterfaceId::parse(SDK_SKILLS_SERVICE).expect("static SDK skills interface id is valid")
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum SdkConfigCommand {
    Read { path: SdkConfigPath },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "String")]
pub struct SdkConfigPath(String);

impl SdkConfigPath {
    pub fn parse(value: impl Into<String>) -> Result<Self, &'static str> {
        value.into().try_into()
    }

    #[must_use]
    pub fn as_path(&self) -> &Path {
        Path::new(&self.0)
    }
}

impl TryFrom<String> for SdkConfigPath {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err("config path must not be empty");
        }
        if value
            .split(std::path::MAIN_SEPARATOR)
            .any(|segment| segment.is_empty() || matches!(segment, "." | ".."))
            || !Path::new(&value)
                .components()
                .all(|component| matches!(component, Component::Normal(_)))
        {
            return Err("config path must be relative and contain no . or .. components");
        }
        Ok(Self(value))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "result", rename_all = "snake_case", deny_unknown_fields)]
pub enum SdkConfigResponse {
    File { content: Vec<u8> },
}

pub struct SdkConfigInterface;

impl ComponentInterface for SdkConfigInterface {
    type Request = SdkConfigCommand;
    type Response = SdkConfigResponse;

    fn interface_id() -> InterfaceId {
        InterfaceId::parse(SDK_CONFIG_SERVICE).expect("static SDK config interface id is valid")
    }
}

#[must_use]
pub fn sdk_config_service() -> ServiceId {
    ServiceId::parse(SDK_CONFIG_SERVICE).expect("static SDK config service id is valid")
}

#[must_use]
pub fn sdk_session_service() -> ServiceId {
    ServiceId::parse(SDK_SESSION_SERVICE).expect("static SDK session service id is valid")
}

#[must_use]
pub fn sdk_tools_service() -> ServiceId {
    ServiceId::parse(SDK_TOOLS_SERVICE).expect("static SDK tools service id is valid")
}

#[must_use]
pub fn sdk_skills_service() -> ServiceId {
    ServiceId::parse(SDK_SKILLS_SERVICE).expect("static SDK skills service id is valid")
}

#[must_use]
pub fn sdk_component_id() -> ComponentId {
    ComponentId::parse(SDK_COMPONENT).expect("static SDK component id is valid")
}

#[must_use]
pub fn sdk_manifest(maximum_authority: Authority) -> PluginManifest {
    PluginManifest {
        id: PluginId::parse(SDK_PLUGIN).expect("static SDK plugin id is valid"),
        version: 1,
        execution: PluginExecution::Embedded,
        dependencies: Vec::new(),
        services: [
            sdk_session_service(),
            sdk_tools_service(),
            sdk_skills_service(),
            sdk_config_service(),
        ]
        .into_iter()
        .map(|service| ServiceContribution {
            role: ServiceRole::Terminal,
            service,
            priority: 100,
            required_authority: Authority::default(),
        })
        .collect(),
        resource_namespaces: Vec::new(),
        maximum_authority,
    }
}

#[must_use]
pub fn sdk_component_manifest(maximum_authority: Authority) -> ComponentManifest {
    let optional_import = |interface: InterfaceId| ComponentImport {
        interface,
        required: false,
        authority: maximum_authority.clone(),
    };
    ComponentManifest {
        id: sdk_component_id(),
        owner: PluginId::parse(SDK_PLUGIN).expect("static SDK plugin id is valid"),
        imports: vec![
            optional_import(SessionInterface::interface_id()),
            optional_import(OptionsInterface::interface_id()),
            optional_import(ExecutionInterface::interface_id()),
            optional_import(ContextInterface::interface_id()),
        ],
        exports: vec![
            ComponentExport {
                interface: SdkSessionInterface::interface_id(),
                priority: 100,
                required_authority: Authority::default(),
            },
            ComponentExport {
                interface: SdkToolsInterface::interface_id(),
                priority: 100,
                required_authority: Authority::default(),
            },
            ComponentExport {
                interface: SdkSkillsInterface::interface_id(),
                priority: 100,
                required_authority: Authority::default(),
            },
            ComponentExport {
                interface: SdkConfigInterface::interface_id(),
                priority: 100,
                required_authority: Authority::default(),
            },
        ],
        maximum_authority,
    }
}

#[must_use]
pub fn sdk_factory() -> Box<dyn PluginInstance> {
    Box::new(SdkPlugin {
        config_root: env::var_os("PHENIX_CONFIG_DIR").map(PathBuf::from),
    })
}

#[must_use]
pub fn sdk_contribution() -> SdkContribution {
    let mut contribution = SdkContribution::new(
        PluginId::parse(SDK_PLUGIN).expect("static SDK plugin id is valid"),
        SdkNamespace::parse("phenix").expect("static SDK namespace is valid"),
    );
    contribution.interfaces = BTreeSet::from([
        SdkSessionInterface::interface_id(),
        ModelRoutingInterface::interface_id(),
        SdkToolsInterface::interface_id(),
        SdkSkillsInterface::interface_id(),
        SdkConfigInterface::interface_id(),
        ContextInterface::interface_id(),
        OptionsInterface::interface_id(),
    ]);
    contribution.resources = BTreeSet::from([
        SdkResourceId::parse("sdk/phenix/sessions").expect("static SDK resource id is valid"),
        SdkResourceId::parse("sdk/phenix/models").expect("static SDK resource id is valid"),
        SdkResourceId::parse("sdk/phenix/tools").expect("static SDK resource id is valid"),
        SdkResourceId::parse("sdk/phenix/skills").expect("static SDK resource id is valid"),
        SdkResourceId::parse("sdk/phenix/context").expect("static SDK resource id is valid"),
        SdkResourceId::parse("sdk/phenix/options").expect("static SDK resource id is valid"),
        SdkResourceId::parse("sdk/phenix/config").expect("static SDK resource id is valid"),
    ]);
    contribution
}

struct SdkPlugin {
    config_root: Option<PathBuf>,
}

impl PluginInstance for SdkPlugin {
    fn start(&mut self, _host: &PluginHost<'_>) -> Result<(), String> {
        Ok(())
    }

    fn invoke(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        let context = plugin_context(host, &self.config_root);
        if service == &sdk_session_service() {
            let command = serde_json::from_slice(input).map_err(|error| error.to_string())?;
            return serde_json::to_vec(&session_command(&context, command)?)
                .map_err(|error| error.to_string());
        }
        if service == &sdk_tools_service() {
            let command = serde_json::from_slice(input).map_err(|error| error.to_string())?;
            return serde_json::to_vec(&tool_command(&context, command)?)
                .map_err(|error| error.to_string());
        }
        if service == &sdk_skills_service() {
            let command = serde_json::from_slice(input).map_err(|error| error.to_string())?;
            return serde_json::to_vec(&skill_command(&context, command)?)
                .map_err(|error| error.to_string());
        }
        if service == &sdk_config_service() {
            let command = serde_json::from_slice(input).map_err(|error| error.to_string())?;
            let root = context
                .plugin
                .settings
                .as_deref()
                .ok_or("PHENIX_CONFIG_DIR is not configured")?;
            return serde_json::to_vec(&config_command(root, command)?)
                .map_err(|error| error.to_string());
        }
        Err(format!("unsupported SDK service: {service}"))
    }
}

fn session_command(
    context: &SdkRuntimeContext<'_, '_, '_>,
    command: SdkSessionCommand,
) -> Result<SdkSessionResponse, String> {
    match command {
        SdkSessionCommand::Open { id, agent } => open_session(context, id, agent),
    }
}

fn config_command(root: &Path, command: SdkConfigCommand) -> Result<SdkConfigResponse, String> {
    match command {
        SdkConfigCommand::Read { path } => {
            let path = config_path(root, &path)?;
            let content = fs::read(&path).map_err(|error| {
                format!("failed to read config file {}: {error}", path.display())
            })?;
            Ok(SdkConfigResponse::File { content })
        }
    }
}

fn config_path(root: &Path, relative: &SdkConfigPath) -> Result<PathBuf, String> {
    let root = fs::canonicalize(root)
        .map_err(|error| format!("failed to resolve config root {}: {error}", root.display()))?;
    if !root.is_dir() {
        return Err(format!(
            "config root is not a directory: {}",
            root.display()
        ));
    }
    let path = root.join(relative.as_path());
    let path = fs::canonicalize(&path)
        .map_err(|error| format!("failed to resolve config file {}: {error}", path.display()))?;
    if !path.starts_with(&root) {
        return Err(format!(
            "config path escapes config root: {}",
            relative.as_path().display()
        ));
    }
    Ok(path)
}

fn open_session(
    context: &SdkRuntimeContext<'_, '_, '_>,
    id: String,
    agent: Option<String>,
) -> Result<SdkSessionResponse, String> {
    if id.trim().is_empty() {
        return Err("session id must not be empty".into());
    }
    let option_context = option_context(&id, agent)?;
    let existing = match context
        .sdk
        .sessions
        .invoke(&SessionCommand::Get { id: id.clone() })
        .map_err(|error| error.to_string())?
    {
        SessionResponse::Session { session } => session,
        response => return Err(format!("unexpected session lookup response: {response:?}")),
    };

    if let Some(session) = existing {
        if resolve_bool(context, "session.reuse_existing", &option_context)? {
            return Ok(SdkSessionResponse::Opened {
                session,
                created: false,
            });
        }
        return Err(format!(
            "session already exists and reuse is disabled: {id}"
        ));
    }

    if !resolve_bool(context, "session.auto_create", &option_context)? {
        return Err(format!(
            "session does not exist and auto-create is disabled: {id}"
        ));
    }

    match context
        .sdk
        .sessions
        .invoke(&SessionCommand::Create { id })
        .map_err(|error| error.to_string())?
    {
        SessionResponse::Created { session } => Ok(SdkSessionResponse::Opened {
            session,
            created: true,
        }),
        response => Err(format!("unexpected session create response: {response:?}")),
    }
}

fn tool_command(
    context: &SdkRuntimeContext<'_, '_, '_>,
    command: SdkToolCommand,
) -> Result<SdkToolResponse, String> {
    match command {
        SdkToolCommand::Register {
            id,
            service,
            required_capabilities,
        } => {
            let response = context
                .sdk
                .execution
                .invoke(&ExecutionCommand::RegisterCallable {
                    id,
                    service,
                    required_authority: ExecutionAuthority::new(required_capabilities),
                })
                .map_err(|error| error.to_string())?;
            let ExecutionResponse::Callable { callable } = response else {
                return Err("unexpected execution response while registering SDK tool".into());
            };
            Ok(SdkToolResponse::Tool {
                tool: SdkTool {
                    id: callable.id,
                    service: callable.service,
                    required_capabilities: callable.required_authority.capabilities,
                },
            })
        }
        SdkToolCommand::Invoke {
            execution_id,
            id,
            input,
        } => {
            let response = context
                .sdk
                .execution
                .invoke(&ExecutionCommand::InvokeCallable {
                    execution_id,
                    callable_id: id,
                    input,
                })
                .map_err(|error| error.to_string())?;
            let ExecutionResponse::Invocation { output } = response else {
                return Err("unexpected execution response while invoking SDK tool".into());
            };
            Ok(SdkToolResponse::Output { output })
        }
    }
}

fn skill_command(
    context: &SdkRuntimeContext<'_, '_, '_>,
    command: SdkSkillCommand,
) -> Result<SdkSkillResponse, String> {
    match command {
        SdkSkillCommand::Register { id, content } => {
            require_non_empty("skill id", &id)?;
            let resource_id = skill_resource_id(&id);
            let response = invoke_context(
                context,
                ContextCommand::Register {
                    resource_id,
                    kind: ContextResourceKind::Skill,
                    source: format!("sdk:{id}"),
                    scope: ContextScope::Workspace,
                    content,
                },
            )?;
            let ContextResponse::Registered { resource } = response else {
                return Err("unexpected context response while registering SDK skill".into());
            };
            Ok(SdkSkillResponse::Skill {
                skill: Some(skill_from_revision(resource)),
            })
        }
        SdkSkillCommand::Get { id } => {
            require_non_empty("skill id", &id)?;
            let Some(descriptor) = find_skill_descriptor(context, &id)? else {
                return Ok(SdkSkillResponse::Skill { skill: None });
            };
            let response = invoke_context(
                context,
                ContextCommand::Get {
                    resource_id: descriptor.resource_id,
                    revision: descriptor.revision,
                },
            )?;
            let ContextResponse::Resource { resource } = response else {
                return Err("unexpected context response while reading SDK skill".into());
            };
            Ok(SdkSkillResponse::Skill {
                skill: resource.map(skill_from_revision),
            })
        }
        SdkSkillCommand::List => {
            let response = invoke_context(context, ContextCommand::List)?;
            let ContextResponse::Resources { descriptors } = response else {
                return Err("unexpected context response while listing SDK skills".into());
            };
            Ok(SdkSkillResponse::Skills {
                skills: descriptors
                    .into_iter()
                    .filter(|descriptor| descriptor.kind == ContextResourceKind::Skill)
                    .map(skill_summary)
                    .collect(),
            })
        }
    }
}

fn invoke_context(
    context: &SdkRuntimeContext<'_, '_, '_>,
    command: ContextCommand,
) -> Result<ContextResponse, String> {
    context
        .sdk
        .context
        .invoke(&command)
        .map_err(|error| error.to_string())
}

fn find_skill_descriptor(
    context: &SdkRuntimeContext<'_, '_, '_>,
    id: &str,
) -> Result<Option<ContextDescriptor>, String> {
    let response = invoke_context(context, ContextCommand::List)?;
    let ContextResponse::Resources { descriptors } = response else {
        return Err("unexpected context response while locating SDK skill".into());
    };
    Ok(descriptors.into_iter().find(|descriptor| {
        descriptor.kind == ContextResourceKind::Skill && skill_id(&descriptor.resource_id) == id
    }))
}

fn skill_from_revision(resource: ContextResourceRevision) -> SdkSkill {
    SdkSkill {
        id: skill_id(&resource.descriptor.resource_id).to_owned(),
        content: resource.content,
    }
}

fn skill_summary(descriptor: ContextDescriptor) -> SdkSkillSummary {
    SdkSkillSummary {
        id: skill_id(&descriptor.resource_id).to_owned(),
        revision: descriptor.revision,
        source: descriptor.source,
    }
}

fn skill_resource_id(id: &str) -> String {
    format!("skill:{id}")
}

fn skill_id(resource_id: &str) -> &str {
    resource_id.strip_prefix("skill:").unwrap_or(resource_id)
}

fn require_non_empty(name: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{name} must not be empty"));
    }
    Ok(())
}

fn option_context(id: &str, agent: Option<String>) -> Result<OptionContext, String> {
    Ok(OptionContext {
        session: Some(OptionSubjectId::parse(id).map_err(|error| error.to_owned())?),
        agent: agent
            .map(OptionSubjectId::parse)
            .transpose()
            .map_err(|error| error.to_owned())?,
    })
}

fn resolve_bool(
    context: &SdkRuntimeContext<'_, '_, '_>,
    key: &str,
    option_context: &OptionContext,
) -> Result<bool, String> {
    let key = OptionKey::parse(key).expect("static SDK option key is valid");
    let response = context
        .sdk
        .options
        .invoke(&OptionCommand::Resolve {
            key: key.clone(),
            context: option_context.clone(),
        })
        .map_err(|error| error.to_string())?;
    let OptionResponse::Value { option } = response else {
        return Err(format!("unexpected option resolution response for {key}"));
    };
    match option.value {
        OptionValue::Bool(value) => Ok(value),
        _ => Err(format!("option {key} is not boolean")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_core::{
        CapabilityId, Kernel, KernelConfig, ResolvedHarness, ResolvedHarnessActivation,
    };
    use phenix_plugin_context::{context_component_manifest, context_factory, context_manifest};
    use phenix_plugin_execution::{
        execution_component_manifest, execution_factory, execution_manifest,
    };
    use phenix_plugin_options::{
        options_component_manifest, options_factory, options_manifest, options_service, OptionScope,
    };
    use phenix_plugin_sessions::{session_component_manifest, session_factory, session_manifest};
    use std::sync::atomic::{AtomicU64, Ordering};

    struct TempDirectory(PathBuf);

    impl TempDirectory {
        fn new(name: &str) -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let nonce = NEXT.fetch_add(1, Ordering::Relaxed);
            let path =
                env::temp_dir().join(format!("phenix-sdk-{name}-{}-{nonce}", std::process::id()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn authority() -> Authority {
        Authority::new([
            CapabilityId::parse("kernel.persistence.schema").unwrap(),
            CapabilityId::parse("kernel.persistence.read").unwrap(),
            CapabilityId::parse("kernel.persistence.write").unwrap(),
        ])
    }

    #[test]
    fn default_sdk_contribution_exposes_standard_modules() {
        let contribution = sdk_contribution();
        assert_eq!(contribution.namespace.as_str(), "phenix");
        for interface in [
            SdkSessionInterface::interface_id(),
            ModelRoutingInterface::interface_id(),
            SdkToolsInterface::interface_id(),
            SdkSkillsInterface::interface_id(),
            ContextInterface::interface_id(),
            OptionsInterface::interface_id(),
        ] {
            assert!(contribution.interfaces.contains(&interface));
        }
    }

    #[test]
    fn config_paths_parse_before_dispatch() {
        assert_eq!(
            serde_json::from_str::<SdkConfigCommand>(
                r#"{"operation":"read","path":"nested/settings.json"}"#,
            )
            .unwrap(),
            SdkConfigCommand::Read {
                path: SdkConfigPath::parse("nested/settings.json").unwrap(),
            }
        );
        for path in ["", ".", "../settings.json", "/settings.json"] {
            let input = format!(r#"{{"operation":"read","path":{path:?}}}"#);
            assert!(serde_json::from_str::<SdkConfigCommand>(&input).is_err());
        }
    }

    #[test]
    fn config_reads_stay_under_the_config_root() {
        let directory = TempDirectory::new("config-read");
        fs::write(directory.path().join("settings.json"), b"settings").unwrap();

        assert_eq!(
            config_command(
                directory.path(),
                SdkConfigCommand::Read {
                    path: SdkConfigPath::parse("settings.json").unwrap(),
                },
            )
            .unwrap(),
            SdkConfigResponse::File {
                content: b"settings".to_vec(),
            }
        );
    }

    #[cfg(unix)]
    #[test]
    fn config_reads_reject_symlink_escapes() {
        use std::os::unix::fs::symlink;

        let directory = TempDirectory::new("config-symlink");
        let config = directory.path().join("config");
        let outside = directory.path().join("outside");
        fs::create_dir_all(&config).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("secret.json"), b"secret").unwrap();
        symlink(outside.join("secret.json"), config.join("settings.json")).unwrap();

        let error = config_command(
            &config,
            SdkConfigCommand::Read {
                path: SdkConfigPath::parse("settings.json").unwrap(),
            },
        )
        .unwrap_err();
        assert!(error.contains("escapes config root"));
    }

    #[test]
    fn session_open_uses_scoped_options() {
        let authority = authority();
        let session_manifest = session_manifest();
        let options_manifest = options_manifest();
        let sdk_manifest = sdk_manifest(authority.clone());
        let manifests = vec![
            session_manifest.clone(),
            options_manifest.clone(),
            sdk_manifest.clone(),
        ];
        let resolved = ResolvedHarness::resolve(
            manifests.clone(),
            [
                session_component_manifest(),
                options_component_manifest(),
                sdk_component_manifest(authority.clone()),
            ],
            [],
            &authority,
        )
        .unwrap();
        let mut kernel = Kernel::new(KernelConfig::new(manifests).unwrap());
        kernel
            .register_embedded_factory(session_manifest.id, session_factory)
            .unwrap();
        kernel
            .register_embedded_factory(options_manifest.id, options_factory)
            .unwrap();
        kernel
            .register_embedded_factory(sdk_manifest.id, sdk_factory)
            .unwrap();
        kernel.activate_resolved_harness(&resolved).unwrap();
        kernel.activate_all().unwrap();

        let open = |kernel: &mut Kernel| {
            let output = kernel
                .invoke(
                    &sdk_session_service(),
                    &serde_json::to_vec(&SdkSessionCommand::Open {
                        id: "root".into(),
                        agent: Some("planner".into()),
                    })
                    .unwrap(),
                    &authority,
                    None,
                )
                .unwrap();
            serde_json::from_slice::<SdkSessionResponse>(&output).unwrap()
        };

        assert!(matches!(
            open(&mut kernel),
            SdkSessionResponse::Opened { created: true, .. }
        ));
        assert!(matches!(
            open(&mut kernel),
            SdkSessionResponse::Opened { created: false, .. }
        ));

        kernel
            .invoke(
                &options_service(),
                &serde_json::to_vec(&OptionCommand::Set {
                    key: OptionKey::parse("session.reuse_existing").unwrap(),
                    scope: OptionScope::Agent(OptionSubjectId::parse("planner").unwrap()),
                    value: OptionValue::Bool(false),
                })
                .unwrap(),
                &authority,
                None,
            )
            .unwrap();

        assert!(kernel
            .invoke(
                &sdk_session_service(),
                &serde_json::to_vec(&SdkSessionCommand::Open {
                    id: "root".into(),
                    agent: Some("planner".into()),
                })
                .unwrap(),
                &authority,
                None,
            )
            .is_err());
    }

    #[test]
    fn sdk_tools_wrap_execution_callables() {
        let authority = authority();
        let execution_manifest = execution_manifest(authority.clone());
        let sdk_manifest = sdk_manifest(authority.clone());
        let manifests = vec![execution_manifest.clone(), sdk_manifest.clone()];
        let resolved = ResolvedHarness::resolve(
            manifests.clone(),
            [
                execution_component_manifest(authority.clone()),
                sdk_component_manifest(authority.clone()),
            ],
            [],
            &authority,
        )
        .unwrap();
        let mut kernel = Kernel::new(KernelConfig::new(manifests).unwrap());
        kernel
            .register_embedded_factory(execution_manifest.id, execution_factory)
            .unwrap();
        kernel
            .register_embedded_factory(sdk_manifest.id, sdk_factory)
            .unwrap();
        kernel.activate_resolved_harness(&resolved).unwrap();
        kernel.activate_all().unwrap();

        let output = kernel
            .invoke(
                &sdk_tools_service(),
                &serde_json::to_vec(&SdkToolCommand::Register {
                    id: "read".into(),
                    service: "fixture.read@1".into(),
                    required_capabilities: BTreeSet::new(),
                })
                .unwrap(),
                &authority,
                None,
            )
            .unwrap();
        assert!(matches!(
            serde_json::from_slice::<SdkToolResponse>(&output).unwrap(),
            SdkToolResponse::Tool { tool } if tool.id == "read" && tool.service == "fixture.read@1"
        ));
    }

    #[test]
    fn sdk_skills_wrap_context_resources() {
        let authority = authority();
        let execution_manifest = execution_manifest(authority.clone());
        let context_manifest = context_manifest();
        let sdk_manifest = sdk_manifest(authority.clone());
        let manifests = vec![
            execution_manifest.clone(),
            context_manifest.clone(),
            sdk_manifest.clone(),
        ];
        let resolved = ResolvedHarness::resolve(
            manifests.clone(),
            [
                execution_component_manifest(authority.clone()),
                context_component_manifest(),
                sdk_component_manifest(authority.clone()),
            ],
            [],
            &authority,
        )
        .unwrap();
        let mut kernel = Kernel::new(KernelConfig::new(manifests).unwrap());
        kernel
            .register_embedded_factory(execution_manifest.id, execution_factory)
            .unwrap();
        kernel
            .register_embedded_factory(context_manifest.id, context_factory)
            .unwrap();
        kernel
            .register_embedded_factory(sdk_manifest.id, sdk_factory)
            .unwrap();
        kernel.activate_resolved_harness(&resolved).unwrap();
        kernel.activate_all().unwrap();

        let register = kernel
            .invoke(
                &sdk_skills_service(),
                &serde_json::to_vec(&SdkSkillCommand::Register {
                    id: "review".into(),
                    content: b"review carefully".to_vec(),
                })
                .unwrap(),
                &authority,
                None,
            )
            .unwrap();
        assert!(matches!(
            serde_json::from_slice::<SdkSkillResponse>(&register).unwrap(),
            SdkSkillResponse::Skill { skill: Some(skill) }
                if skill.id == "review" && skill.content == b"review carefully"
        ));

        let list = kernel
            .invoke(
                &sdk_skills_service(),
                &serde_json::to_vec(&SdkSkillCommand::List).unwrap(),
                &authority,
                None,
            )
            .unwrap();
        assert!(matches!(
            serde_json::from_slice::<SdkSkillResponse>(&list).unwrap(),
            SdkSkillResponse::Skills { skills } if skills.len() == 1 && skills[0].id == "review"
        ));
    }
}
