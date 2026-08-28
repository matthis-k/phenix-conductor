use phenix_harness::{default_suite_authority, PhenixHarness};
use phenix_plugin_catalog::{
    execution_configuration_service, model_routing_service, options_service, AgentDefinition,
    ExecutionConfigurationCommand, ExecutionConfigurationResponse, ModelCommand, ModelResponse,
    ModelTarget, OptionCommand, OptionKey, OptionResponse, OptionScope, OptionSubjectId,
    OptionValue, OrchestrationDefinition, RoutingProfile,
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
    id: String,
    default_target: RuntimeModelTarget,
    #[serde(default)]
    callable_targets: BTreeMap<String, RuntimeModelTarget>,
}

#[derive(Debug, Deserialize, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingsConfiguration {
    #[serde(default)]
    global: BTreeMap<String, SettingValue>,
    #[serde(default)]
    sessions: BTreeMap<String, BTreeMap<String, SettingValue>>,
    #[serde(default)]
    agents: BTreeMap<String, BTreeMap<String, SettingValue>>,
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

struct RuntimeModelTarget {
    backend: String,
    provider: String,
    model: String,
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

pub(super) fn apply_config_directory(
    harness: &mut PhenixHarness,
    directory: &Path,
) -> Result<(), Box<dyn Error>> {
    if !directory.is_dir() {
        return Err(format!("config directory does not exist: {}", directory.display()).into());
    }

    let runtime = directory.join("runtime.json");
    if runtime.is_file() {
        apply_runtime_config(harness, &runtime)?;
    }

    let settings = directory.join("settings.json");
    if settings.is_file() {
        apply_settings(harness, &settings)?;
    }
    Ok(())
}

fn apply_settings(harness: &mut PhenixHarness, path: &Path) -> Result<(), Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let settings: SettingsConfiguration = serde_json::from_slice(&bytes)?;
    apply_settings_configuration(harness, settings)
}

fn apply_settings_configuration(
    harness: &mut PhenixHarness,
    settings: SettingsConfiguration,
) -> Result<(), Box<dyn Error>> {
    for (key, value) in settings.global {
        set_option(harness, key, OptionScope::Global, value)?;
    }
    for (session, values) in settings.sessions {
        let session = OptionSubjectId::parse(session)?;
        for (key, value) in values {
            set_option(
                harness,
                key,
                OptionScope::Session {
                    session: session.clone(),
                },
                value,
            )?;
        }
    }
    for (agent, values) in settings.agents {
        let agent = OptionSubjectId::parse(agent)?;
        for (key, value) in values {
            set_option(
                harness,
                key,
                OptionScope::Agent {
                    agent: agent.clone(),
                },
                value,
            )?;
        }
    }
    Ok(())
}

fn set_option(
    harness: &mut PhenixHarness,
    key: String,
    scope: OptionScope,
    value: SettingValue,
) -> Result<(), Box<dyn Error>> {
    let key = OptionKey::parse(key)?;
    let output = harness.invoke(
        &options_service(),
        &serde_json::to_vec(&OptionCommand::Set {
            key,
            scope,
            value: value.into(),
        })?,
        &default_suite_authority(),
        None,
    )?;
    match serde_json::from_slice::<OptionResponse>(&output)? {
        OptionResponse::Updated { .. } => Ok(()),
        _ => Err("options service rejected settings override".into()),
    }
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
                    id: "agent.scout".into()
                }
            ),
            ExecutionConfigurationResponse::Agent { agent: Some(_) }
        ));
        assert!(matches!(
            invoke_configuration(
                &mut harness,
                ExecutionConfigurationCommand::GetOrchestration {
                    id: "orchestration.review".into()
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
                    id: "router.test".into(),
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
