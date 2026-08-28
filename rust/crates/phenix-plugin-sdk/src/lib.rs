#![forbid(unsafe_code)]

use phenix_core::{
    Authority, ComponentExport, ComponentId, ComponentImport, ComponentInterface,
    ComponentManifest, InterfaceId, PluginExecution, PluginHost, PluginId, PluginInstance,
    PluginManifest, SdkContribution, SdkNamespace, SdkResourceId, ServiceContribution, ServiceId,
    ServiceRole,
};
use phenix_plugin_basic_agent::{BasicSkillsInterface, BasicToolsInterface};
use phenix_plugin_context::ContextInterface;
use phenix_plugin_models::ModelRoutingInterface;
use phenix_plugin_options::{
    OptionCommand, OptionContext, OptionKey, OptionResponse, OptionSubjectId, OptionValue,
    OptionsInterface,
};
use phenix_plugin_sessions::{SessionCommand, SessionInterface, SessionRecord, SessionResponse};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const SDK_PLUGIN: &str = "phenix.sdk";
pub const SDK_COMPONENT: &str = "phenix.sdk";
pub const SDK_SESSION_SERVICE: &str = "phenix.sdk.sessions@1";

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

#[must_use]
pub fn sdk_session_service() -> ServiceId {
    ServiceId::parse(SDK_SESSION_SERVICE).expect("static SDK session service id is valid")
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
        services: vec![ServiceContribution {
            role: ServiceRole::Terminal,
            service: sdk_session_service(),
            priority: 100,
            required_authority: Authority::default(),
        }],
        resource_namespaces: Vec::new(),
        maximum_authority,
    }
}

#[must_use]
pub fn sdk_component_manifest(maximum_authority: Authority) -> ComponentManifest {
    ComponentManifest {
        id: sdk_component_id(),
        owner: PluginId::parse(SDK_PLUGIN).expect("static SDK plugin id is valid"),
        imports: vec![
            ComponentImport {
                interface: SessionInterface::interface_id(),
                required: true,
                authority: maximum_authority.clone(),
            },
            ComponentImport {
                interface: OptionsInterface::interface_id(),
                required: true,
                authority: maximum_authority.clone(),
            },
        ],
        exports: vec![ComponentExport {
            interface: SdkSessionInterface::interface_id(),
            priority: 100,
            required_authority: Authority::default(),
        }],
        maximum_authority,
    }
}

#[must_use]
pub fn sdk_factory() -> Box<dyn PluginInstance> {
    Box::new(SdkPlugin)
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
        BasicToolsInterface::interface_id(),
        BasicSkillsInterface::interface_id(),
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
    ]);
    contribution
}

struct SdkPlugin;

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
        if service != &sdk_session_service() {
            return Err(format!("unsupported SDK service: {service}"));
        }
        let command: SdkSessionCommand =
            serde_json::from_slice(input).map_err(|error| error.to_string())?;
        let response = match command {
            SdkSessionCommand::Open { id, agent } => open_session(host, id, agent)?,
        };
        serde_json::to_vec(&response).map_err(|error| error.to_string())
    }
}

fn open_session(
    host: &PluginHost<'_>,
    id: String,
    agent: Option<String>,
) -> Result<SdkSessionResponse, String> {
    if id.trim().is_empty() {
        return Err("session id must not be empty".into());
    }
    let context = option_context(&id, agent)?;
    let existing = match host
        .invoke_import::<SessionInterface>(
            &sdk_component_id(),
            &SessionCommand::Get { id: id.clone() },
        )
        .map_err(|error| error.to_string())?
    {
        SessionResponse::Session { session } => session,
        response => return Err(format!("unexpected session lookup response: {response:?}")),
    };

    if let Some(session) = existing {
        if resolve_bool(host, "session.reuse_existing", &context)? {
            return Ok(SdkSessionResponse::Opened {
                session,
                created: false,
            });
        }
        return Err(format!(
            "session already exists and reuse is disabled: {id}"
        ));
    }

    if !resolve_bool(host, "session.auto_create", &context)? {
        return Err(format!(
            "session does not exist and auto-create is disabled: {id}"
        ));
    }

    match host
        .invoke_import::<SessionInterface>(&sdk_component_id(), &SessionCommand::Create { id })
        .map_err(|error| error.to_string())?
    {
        SessionResponse::Created { session } => Ok(SdkSessionResponse::Opened {
            session,
            created: true,
        }),
        response => Err(format!("unexpected session create response: {response:?}")),
    }
}

fn option_context(id: &str, agent: Option<String>) -> Result<OptionContext, String> {
    Ok(OptionContext {
        session: Some(OptionSubjectId::parse(id).map_err(str::to_owned)?),
        agent: agent
            .map(OptionSubjectId::parse)
            .transpose()
            .map_err(str::to_owned)?,
    })
}

fn resolve_bool(host: &PluginHost<'_>, key: &str, context: &OptionContext) -> Result<bool, String> {
    let key = OptionKey::parse(key).expect("static SDK option key is valid");
    let response = host
        .invoke_import::<OptionsInterface>(
            &sdk_component_id(),
            &OptionCommand::Resolve {
                key: key.clone(),
                context: context.clone(),
            },
        )
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
    use phenix_plugin_options::{
        options_component_manifest, options_factory, options_manifest, options_service, OptionScope,
    };
    use phenix_plugin_sessions::{session_component_manifest, session_factory, session_manifest};

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
            BasicToolsInterface::interface_id(),
            BasicSkillsInterface::interface_id(),
            ContextInterface::interface_id(),
            OptionsInterface::interface_id(),
        ] {
            assert!(contribution.interfaces.contains(&interface));
        }
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
                    scope: OptionScope::Agent {
                        agent: OptionSubjectId::parse("planner").unwrap(),
                    },
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
}
