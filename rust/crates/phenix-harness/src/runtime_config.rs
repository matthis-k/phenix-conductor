use phenix_core::{CallableId, ModelId, PluginId, RoutingProfileId};
use phenix_harness::{default_suite_authority, PhenixHarness};
use phenix_plugin_catalog::{
    execution_configuration_service, model_routing_service, options_service, AgentDefinition,
    ExecutionConfigurationCommand, ExecutionConfigurationResponse, ModelCommand, ModelResponse,
    ModelTarget, OptionAssignment, OptionCommand, OptionKey, OptionResponse, OptionScope,
    OptionStartupPrecedence, OptionSubjectId, OptionValue, OrchestrationDefinition, RoutingProfile,
};
use serde::Deserialize;
use serde_json::Value;
use std::{collections::BTreeMap, error::Error, fs, path::Path};

#[derive(Debug, Deserialize)]
struct RuntimeConfiguration {
    agents: Vec<AgentDefinition>,
    orchestrations: Vec<OrchestrationDefinition>,
    routing_profiles: Vec<RuntimeRoutingProfile>,
}

#[derive(Debug, Deserialize)]
struct RuntimeRoutingProfile {
    id: RoutingProfileId,
    default_target: RuntimeModelTarget,
    #[serde(default)]
    callable_targets: BTreeMap<CallableId, RuntimeModelTarget>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingsConfiguration {
    #[serde(default)]
    global: BTreeMap<OptionKey, SettingValue>,
    #[serde(default)]
    sessions: BTreeMap<OptionSubjectId, BTreeMap<OptionKey, SettingValue>>,
    #[serde(default)]
    agents: BTreeMap<OptionSubjectId, BTreeMap<OptionKey, SettingValue>>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum SettingValue {
    Bool(bool),
    Integer(i64),
    String(String),
}

impl From<SettingValue> for OptionValue {
    fn from(value: SettingValue) -> Self {
        match value {
            SettingValue::Bool(value) => Self::Bool(value),
            SettingValue::Integer(value) => Self::Integer(value),
            SettingValue::String(value) => Self::String(value),
        }
    }
}

#[derive(Debug, Deserialize)]
struct RuntimeModelTarget {
    backend: String,
    provider: PluginId,
    model: ModelId,
    #[serde(default)]
    inference: Value,
}

impl RuntimeModelTarget {
    fn into_model_target(self) -> ModelTarget {
        let mut options = BTreeMap::new();
        options.insert("backend".into(), Value::String(self.backend));
        options.insert("inference".into(), self.inference);
        ModelTarget {
            provider_plugin: self.provider,
            model: self.model,
            options,
        }
    }
}

impl RuntimeRoutingProfile {
    fn into_routing_profile(self) -> RoutingProfile {
        RoutingProfile {
            id: self.id,
            default_target: self.default_target.into_model_target(),
            callable_targets: self
                .callable_targets
                .into_iter()
                .map(|(callable, target)| (callable, target.into_model_target()))
                .collect(),
        }
    }
}

pub(super) fn apply_default_config_directory(
    harness: &mut PhenixHarness,
    directory: &Path,
) -> Result<(), Box<dyn Error>> {
    if !directory.is_dir() {
        return Err(format!(
            "default config directory does not exist: {}",
            directory.display()
        )
        .into());
    }
    let runtime = directory.join("runtime.json");
    if runtime.is_file() {
        apply_runtime_config(harness, &runtime)?;
    }
    Ok(())
}

pub(super) fn apply_startup_settings(
    harness: &mut PhenixHarness,
    config_directory: Option<&Path>,
    nix_settings: Option<&Path>,
    precedence: OptionStartupPrecedence,
) -> Result<(), Box<dyn Error>> {
    let file_path = config_directory.map(|directory| directory.join("settings.json"));
    let file_settings = read_optional_settings(file_path.as_deref())?;
    let nix_settings = read_optional_settings(nix_settings)?;
    let file_values = settings_assignments(file_settings);
    let nix_values = settings_assignments(nix_settings);

    let service = options_service();
    let has_options = harness.kernel().config().manifests().any(|manifest| {
        manifest
            .services
            .iter()
            .any(|contribution| contribution.service == service)
    });
    if !has_options {
        if file_values.is_empty() && nix_values.is_empty() {
            return Ok(());
        }
        return Err("startup settings require the phenix.options plugin".into());
    }

    let output = harness.invoke(
        &service,
        &serde_json::to_vec(&OptionCommand::Configure {
            file_values,
            nix_values,
            precedence,
        })?,
        &default_suite_authority(),
        None,
    )?;
    match serde_json::from_slice::<OptionResponse>(&output)? {
        OptionResponse::Configured { .. } => Ok(()),
        _ => Err("options service rejected startup settings".into()),
    }
}

fn read_optional_settings(path: Option<&Path>) -> Result<SettingsConfiguration, Box<dyn Error>> {
    let Some(path) = path else {
        return Ok(SettingsConfiguration::default());
    };
    if !path.exists() {
        return Ok(SettingsConfiguration::default());
    }
    if !path.is_file() {
        return Err(format!("settings path is not a file: {}", path.display()).into());
    }
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn settings_assignments(settings: SettingsConfiguration) -> Vec<OptionAssignment> {
    let mut values = Vec::new();
    for (key, value) in settings.global {
        values.push(OptionAssignment {
            key,
            scope: OptionScope::Global,
            value: value.into(),
        });
    }
    for (session, settings) in settings.sessions {
        for (key, value) in settings {
            values.push(OptionAssignment {
                key,
                scope: OptionScope::Session(session.clone()),
                value: value.into(),
            });
        }
    }
    for (agent, settings) in settings.agents {
        for (key, value) in settings {
            values.push(OptionAssignment {
                key,
                scope: OptionScope::Agent(agent.clone()),
                value: value.into(),
            });
        }
    }
    values
}

pub(super) fn apply_runtime_config(
    harness: &mut PhenixHarness,
    path: &Path,
) -> Result<(), Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let configuration: RuntimeConfiguration = serde_json::from_slice(&bytes)?;
    apply_configuration(harness, configuration)
}

fn apply_configuration(
    harness: &mut PhenixHarness,
    configuration: RuntimeConfiguration,
) -> Result<(), Box<dyn Error>> {
    let authority = default_suite_authority();

    for agent in configuration.agents {
        let output = harness.invoke(
            &execution_configuration_service(),
            &serde_json::to_vec(&ExecutionConfigurationCommand::RegisterAgent { agent })?,
            &authority,
            None,
        )?;
        if !matches!(
            serde_json::from_slice::<ExecutionConfigurationResponse>(&output)?,
            ExecutionConfigurationResponse::Agent { agent: Some(_) }
        ) {
            return Err("execution configuration service rejected agent registration".into());
        }
    }

    for orchestration in configuration.orchestrations {
        let output = harness.invoke(
            &execution_configuration_service(),
            &serde_json::to_vec(&ExecutionConfigurationCommand::RegisterOrchestration {
                orchestration,
            })?,
            &authority,
            None,
        )?;
        if !matches!(
            serde_json::from_slice::<ExecutionConfigurationResponse>(&output)?,
            ExecutionConfigurationResponse::Orchestration {
                orchestration: Some(_)
            }
        ) {
            return Err(
                "execution configuration service rejected orchestration registration".into(),
            );
        }
    }

    for profile in configuration.routing_profiles {
        ensure_routing_profile(harness, profile.into_routing_profile())?;
    }

    Ok(())
}

fn ensure_routing_profile(
    harness: &mut PhenixHarness,
    profile: RoutingProfile,
) -> Result<(), Box<dyn Error>> {
    let authority = default_suite_authority();
    let output = harness.invoke(
        &model_routing_service(),
        &serde_json::to_vec(&ModelCommand::GetProfile {
            id: profile.id.clone(),
        })?,
        &authority,
        None,
    )?;
    let existing = match serde_json::from_slice::<ModelResponse>(&output)? {
        ModelResponse::Profile { profile } => profile,
        _ => return Err("model routing service returned the wrong profile response".into()),
    };

    match existing {
        Some(existing) if existing == profile => Ok(()),
        Some(_) => Err(format!("routing profile identity is immutable: {}", profile.id).into()),
        None => {
            let output = harness.invoke(
                &model_routing_service(),
                &serde_json::to_vec(&ModelCommand::RegisterProfile { profile })?,
                &authority,
                None,
            )?;
            if matches!(
                serde_json::from_slice::<ModelResponse>(&output)?,
                ModelResponse::Profile { profile: Some(_) }
            ) {
                Ok(())
            } else {
                Err("model routing service rejected profile registration".into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_plugin_catalog::{ExecutionConfigurationCommand, ModelCommand};
    use serde_json::json;

    fn sample_runtime() -> RuntimeConfiguration {
        serde_json::from_value(json!({
            "agents": [{
                "id": "agent.scout",
                "kind": "agent",
                "description": "Inspect repository evidence.",
                "input_schema": {"type": "string", "minLength": 1},
                "output_schema": {"type": "string"},
                "capabilities": [],
                "policy": {"requires_permission": false}
            }],
            "orchestrations": [{
                "descriptor": {
                    "id": "orchestration.review",
                    "kind": "orchestration",
                    "description": "Independent review",
                    "input_schema": {"type": "string", "minLength": 1},
                    "output_schema": {"type": "string"},
                    "capabilities": [],
                    "policy": {"requires_permission": false}
                },
                "policy": "sequential",
                "nodes": [{
                    "callable": "agent.scout",
                    "objective": "Inspect the current objective."
                }]
            }],
            "routing_profiles": [{
                "id": "router.test",
                "default_target": {
                    "backend": "phenix",
                    "provider": "provider.fixture",
                    "model": "model.test",
                    "inference": {"effort": "low"}
                },
                "callable_targets": {
                    "agent.scout": {
                        "backend": "phenix",
                        "provider": "provider.fixture",
                        "model": "model.scout",
                        "inference": {"effort": "medium"}
                    }
                }
            }]
        }))
        .unwrap()
    }

    fn invoke_configuration(
        harness: &mut PhenixHarness,
        command: ExecutionConfigurationCommand,
    ) -> ExecutionConfigurationResponse {
        let output = harness
            .invoke(
                &execution_configuration_service(),
                &serde_json::to_vec(&command).unwrap(),
                &default_suite_authority(),
                None,
            )
            .unwrap();
        serde_json::from_slice(&output).unwrap()
    }

    #[test]
    fn migrated_runtime_configuration_is_active_and_restart_safe() {
        let mut harness = PhenixHarness::default_suite().unwrap();
        harness.activate().unwrap();
        apply_configuration(&mut harness, sample_runtime()).unwrap();
        apply_configuration(&mut harness, sample_runtime()).unwrap();

        assert!(matches!(
            invoke_configuration(
                &mut harness,
                ExecutionConfigurationCommand::GetAgent {
                    id: CallableId::parse("agent.scout").unwrap()
                }
            ),
            ExecutionConfigurationResponse::Agent { agent: Some(_) }
        ));
        assert!(matches!(
            invoke_configuration(
                &mut harness,
                ExecutionConfigurationCommand::GetOrchestration {
                    id: CallableId::parse("orchestration.review").unwrap()
                }
            ),
            ExecutionConfigurationResponse::Orchestration {
                orchestration: Some(_)
            }
        ));

        let output = harness
            .invoke(
                &model_routing_service(),
                &serde_json::to_vec(&ModelCommand::GetProfile {
                    id: RoutingProfileId::parse("router.test").unwrap(),
                })
                .unwrap(),
                &default_suite_authority(),
                None,
            )
            .unwrap();
        assert!(matches!(
            serde_json::from_slice::<ModelResponse>(&output).unwrap(),
            ModelResponse::Profile { profile: Some(_) }
        ));
    }
}
