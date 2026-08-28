#![forbid(unsafe_code)]

use phenix_core::{
    Authority, CapabilityId, ComponentExport, ComponentId, ComponentInterface, ComponentManifest,
    DurableSchema, InterfaceId, PluginExecution, PluginHost, PluginId, PluginInstance,
    PluginManifest, ResourceNamespace, ServiceContribution, ServiceId, ServiceRole, TransactionOp,
};
use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

pub const OPTIONS_SERVICE: &str = "phenix.options@1";
pub const OPTIONS_PLUGIN: &str = "phenix.options";
pub const OPTIONS_COMPONENT: &str = "phenix.options";
const OPTIONS_NAMESPACE: &str = "phenix.options.state";
const STATE_KEY: &str = "options";
const PERSISTENCE_SCHEMA: &str = "kernel.persistence.schema";
const PERSISTENCE_READ: &str = "kernel.persistence.read";
const PERSISTENCE_WRITE: &str = "kernel.persistence.write";

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OptionKey(String);

impl OptionKey {
    pub fn parse(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        if value.is_empty() {
            return Err("option key must not be empty");
        }
        if !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        {
            return Err("option key contains unsupported characters");
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for OptionKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for OptionKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for OptionKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OptionSubjectId(String);

impl OptionSubjectId {
    pub fn parse(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err("option scope subject must not be empty");
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Serialize for OptionSubjectId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for OptionSubjectId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OptionScopeKind {
    Global,
    Session,
    Agent,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "scope", rename_all = "snake_case", deny_unknown_fields)]
pub enum OptionScope {
    Global,
    Session { session: OptionSubjectId },
    Agent { agent: OptionSubjectId },
}

impl OptionScope {
    fn kind(&self) -> OptionScopeKind {
        match self {
            Self::Global => OptionScopeKind::Global,
            Self::Session { .. } => OptionScopeKind::Session,
            Self::Agent { .. } => OptionScopeKind::Agent,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OptionContext {
    pub session: Option<OptionSubjectId>,
    pub agent: Option<OptionSubjectId>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum OptionValue {
    Bool(bool),
    Integer(i64),
    String(String),
}

impl OptionValue {
    fn has_same_type(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Bool(_), Self::Bool(_))
                | (Self::Integer(_), Self::Integer(_))
                | (Self::String(_), Self::String(_))
        )
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OptionDefinition {
    pub key: OptionKey,
    pub default: OptionValue,
    pub scopes: BTreeSet<OptionScopeKind>,
}

impl OptionDefinition {
    pub fn new(
        key: impl Into<String>,
        default: OptionValue,
        scopes: impl IntoIterator<Item = OptionScopeKind>,
    ) -> Self {
        Self {
            key: OptionKey::parse(key).expect("static option key is valid"),
            default,
            scopes: scopes.into_iter().collect(),
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.scopes.is_empty() {
            return Err(format!("option {} has no writable scope", self.key));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OptionValueSource {
    Default,
    Global,
    Session,
    Agent,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedOption {
    pub key: OptionKey,
    pub value: OptionValue,
    pub source: OptionValueSource,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum OptionCommand {
    Define { definition: OptionDefinition },
    GetDefinition { key: OptionKey },
    Set {
        key: OptionKey,
        scope: OptionScope,
        value: OptionValue,
    },
    Unset { key: OptionKey, scope: OptionScope },
    Resolve { key: OptionKey, context: OptionContext },
    List { context: OptionContext },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "result", rename_all = "snake_case", deny_unknown_fields)]
pub enum OptionResponse {
    Defined { definition: OptionDefinition },
    Definition { definition: Option<OptionDefinition> },
    Updated { key: OptionKey, scope: OptionScope },
    Value { option: ResolvedOption },
    Options { options: Vec<ResolvedOption> },
}

pub struct OptionsInterface;

impl ComponentInterface for OptionsInterface {
    type Request = OptionCommand;
    type Response = OptionResponse;

    fn interface_id() -> InterfaceId {
        InterfaceId::parse(OPTIONS_SERVICE).expect("static options interface id is valid")
    }
}

#[must_use]
pub fn options_service() -> ServiceId {
    ServiceId::parse(OPTIONS_SERVICE).expect("static options service id is valid")
}

#[must_use]
pub fn options_component_id() -> ComponentId {
    ComponentId::parse(OPTIONS_COMPONENT).expect("static options component id is valid")
}

#[must_use]
pub fn options_manifest() -> PluginManifest {
    PluginManifest {
        id: PluginId::parse(OPTIONS_PLUGIN).expect("static options plugin id is valid"),
        version: 1,
        execution: PluginExecution::Embedded,
        dependencies: Vec::new(),
        services: vec![ServiceContribution {
            role: ServiceRole::Terminal,
            service: options_service(),
            priority: 100,
            required_authority: Authority::default(),
        }],
        resource_namespaces: vec![options_namespace()],
        maximum_authority: persistence_authority(),
    }
}

#[must_use]
pub fn options_component_manifest() -> ComponentManifest {
    ComponentManifest {
        id: options_component_id(),
        owner: PluginId::parse(OPTIONS_PLUGIN).expect("static options plugin id is valid"),
        imports: Vec::new(),
        exports: vec![ComponentExport {
            interface: OptionsInterface::interface_id(),
            priority: 100,
            required_authority: Authority::default(),
        }],
        maximum_authority: persistence_authority(),
    }
}

#[must_use]
pub fn options_factory() -> Box<dyn PluginInstance> {
    Box::new(OptionsPlugin)
}

pub fn default_option_definitions() -> Vec<OptionDefinition> {
    use OptionScopeKind::{Agent, Global, Session};
    vec![
        OptionDefinition::new(
            "session.auto_create",
            OptionValue::Bool(true),
            [Global, Session, Agent],
        ),
        OptionDefinition::new(
            "session.reuse_existing",
            OptionValue::Bool(true),
            [Global, Session, Agent],
        ),
        OptionDefinition::new(
            "session.max_turns",
            OptionValue::Integer(0),
            [Global, Session],
        ),
        OptionDefinition::new(
            "model.default",
            OptionValue::String("default".into()),
            [Global, Session, Agent],
        ),
        OptionDefinition::new(
            "tools.confirmation",
            OptionValue::String("ask".into()),
            [Global, Session, Agent],
        ),
        OptionDefinition::new(
            "skills.auto_load",
            OptionValue::Bool(true),
            [Global, Session, Agent],
        ),
        OptionDefinition::new(
            "context.auto_load",
            OptionValue::Bool(true),
            [Global, Session, Agent],
        ),
        OptionDefinition::new(
            "agent.max_parallel_tasks",
            OptionValue::Integer(1),
            [Global, Agent],
        ),
    ]
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
struct OptionState {
    definitions: BTreeMap<OptionKey, OptionDefinition>,
    values: BTreeMap<OptionScope, BTreeMap<OptionKey, OptionValue>>,
}

impl OptionState {
    fn with_defaults(mut self) -> Result<Self, String> {
        for definition in default_option_definitions() {
            match self.definitions.get(&definition.key) {
                Some(existing) if existing == &definition => {}
                Some(_) => {
                    return Err(format!(
                        "persisted definition for {} conflicts with built-in definition",
                        definition.key
                    ));
                }
                None => {
                    self.definitions.insert(definition.key.clone(), definition);
                }
            }
        }
        Ok(self)
    }

    fn define(&mut self, definition: OptionDefinition) -> Result<bool, String> {
        definition.validate()?;
        match self.definitions.get(&definition.key) {
            Some(existing) if existing == &definition => Ok(false),
            Some(_) => Err(format!("option {} is already defined differently", definition.key)),
            None => {
                self.definitions.insert(definition.key.clone(), definition);
                Ok(true)
            }
        }
    }

    fn set(&mut self, key: &OptionKey, scope: OptionScope, value: OptionValue) -> Result<(), String> {
        let definition = self
            .definitions
            .get(key)
            .ok_or_else(|| format!("unknown option: {key}"))?;
        if !definition.scopes.contains(&scope.kind()) {
            return Err(format!("option {key} cannot be set at {:?} scope", scope.kind()));
        }
        if !definition.default.has_same_type(&value) {
            return Err(format!("option {key} value type does not match its definition"));
        }
        self.values.entry(scope).or_default().insert(key.clone(), value);
        Ok(())
    }

    fn unset(&mut self, key: &OptionKey, scope: &OptionScope) -> Result<(), String> {
        if !self.definitions.contains_key(key) {
            return Err(format!("unknown option: {key}"));
        }
        if let Some(values) = self.values.get_mut(scope) {
            values.remove(key);
            if values.is_empty() {
                self.values.remove(scope);
            }
        }
        Ok(())
    }

    fn resolve(&self, key: &OptionKey, context: &OptionContext) -> Result<ResolvedOption, String> {
        let definition = self
            .definitions
            .get(key)
            .ok_or_else(|| format!("unknown option: {key}"))?;

        if let Some(agent) = &context.agent {
            let scope = OptionScope::Agent {
                agent: agent.clone(),
            };
            if let Some(value) = self.value_at(key, &scope) {
                return Ok(ResolvedOption {
                    key: key.clone(),
                    value: value.clone(),
                    source: OptionValueSource::Agent,
                });
            }
        }
        if let Some(session) = &context.session {
            let scope = OptionScope::Session {
                session: session.clone(),
            };
            if let Some(value) = self.value_at(key, &scope) {
                return Ok(ResolvedOption {
                    key: key.clone(),
                    value: value.clone(),
                    source: OptionValueSource::Session,
                });
            }
        }
        if let Some(value) = self.value_at(key, &OptionScope::Global) {
            return Ok(ResolvedOption {
                key: key.clone(),
                value: value.clone(),
                source: OptionValueSource::Global,
            });
        }
        Ok(ResolvedOption {
            key: key.clone(),
            value: definition.default.clone(),
            source: OptionValueSource::Default,
        })
    }

    fn value_at(&self, key: &OptionKey, scope: &OptionScope) -> Option<&OptionValue> {
        self.values.get(scope)?.get(key)
    }
}

struct OptionsPlugin;

impl PluginInstance for OptionsPlugin {
    fn start(&mut self, host: &PluginHost<'_>) -> Result<(), String> {
        host.register_durable_schema(&DurableSchema::new(options_namespace(), 1))
            .map_err(|error| error.to_string())
    }

    fn invoke(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        if service != &options_service() {
            return Err(format!("unsupported options service: {service}"));
        }
        let command: OptionCommand =
            serde_json::from_slice(input).map_err(|error| error.to_string())?;
        let (mut state, raw) = load_state(host)?;
        let mut changed = false;
        let response = match command {
            OptionCommand::Define { definition } => {
                changed = state.define(definition.clone())?;
                OptionResponse::Defined { definition }
            }
            OptionCommand::GetDefinition { key } => OptionResponse::Definition {
                definition: state.definitions.get(&key).cloned(),
            },
            OptionCommand::Set { key, scope, value } => {
                state.set(&key, scope.clone(), value)?;
                changed = true;
                OptionResponse::Updated { key, scope }
            }
            OptionCommand::Unset { key, scope } => {
                state.unset(&key, &scope)?;
                changed = true;
                OptionResponse::Updated { key, scope }
            }
            OptionCommand::Resolve { key, context } => OptionResponse::Value {
                option: state.resolve(&key, &context)?,
            },
            OptionCommand::List { context } => OptionResponse::Options {
                options: state
                    .definitions
                    .keys()
                    .map(|key| state.resolve(key, &context))
                    .collect::<Result<Vec<_>, _>>()?,
            },
        };
        if changed {
            save_state(host, raw, &state)?;
        }
        serde_json::to_vec(&response).map_err(|error| error.to_string())
    }
}

fn load_state(host: &PluginHost<'_>) -> Result<(OptionState, Option<Vec<u8>>), String> {
    let raw = host
        .read_durable(&options_namespace(), STATE_KEY)
        .map_err(|error| error.to_string())?;
    let state = match raw.as_deref() {
        Some(bytes) => serde_json::from_slice(bytes).map_err(|error| error.to_string())?,
        None => OptionState::default(),
    };
    Ok((state.with_defaults()?, raw))
}

fn save_state(
    host: &PluginHost<'_>,
    old: Option<Vec<u8>>,
    state: &OptionState,
) -> Result<(), String> {
    host.transact_durable(
        &options_namespace(),
        &[
            TransactionOp::AssertValue {
                key: STATE_KEY.into(),
                expected: old,
            },
            TransactionOp::Put {
                key: STATE_KEY.into(),
                value: serde_json::to_vec(state).map_err(|error| error.to_string())?,
            },
        ],
    )
    .map_err(|error| error.to_string())
}

fn options_namespace() -> ResourceNamespace {
    ResourceNamespace::parse(OPTIONS_NAMESPACE).expect("static options namespace is valid")
}

fn persistence_authority() -> Authority {
    Authority::new([
        CapabilityId::parse(PERSISTENCE_SCHEMA).expect("static capability is valid"),
        CapabilityId::parse(PERSISTENCE_READ).expect("static capability is valid"),
        CapabilityId::parse(PERSISTENCE_WRITE).expect("static capability is valid"),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn subject(value: &str) -> OptionSubjectId {
        OptionSubjectId::parse(value).unwrap()
    }

    fn key(value: &str) -> OptionKey {
        OptionKey::parse(value).unwrap()
    }

    #[test]
    fn option_resolution_uses_agent_then_session_then_global_then_default() {
        let mut state = OptionState::default().with_defaults().unwrap();
        let key = key("model.default");
        assert_eq!(
            state
                .resolve(&key, &OptionContext::default())
                .unwrap()
                .value,
            OptionValue::String("default".into())
        );

        state
            .set(&key, OptionScope::Global, OptionValue::String("global".into()))
            .unwrap();
        state
            .set(
                &key,
                OptionScope::Session {
                    session: subject("s1"),
                },
                OptionValue::String("session".into()),
            )
            .unwrap();
        state
            .set(
                &key,
                OptionScope::Agent {
                    agent: subject("a1"),
                },
                OptionValue::String("agent".into()),
            )
            .unwrap();

        let context = OptionContext {
            session: Some(subject("s1")),
            agent: Some(subject("a1")),
        };
        let resolved = state.resolve(&key, &context).unwrap();
        assert_eq!(resolved.value, OptionValue::String("agent".into()));
        assert_eq!(resolved.source, OptionValueSource::Agent);
    }

    #[test]
    fn option_scope_and_value_type_are_enforced_before_state_changes() {
        let mut state = OptionState::default().with_defaults().unwrap();
        assert!(state
            .set(
                &key("session.max_turns"),
                OptionScope::Agent {
                    agent: subject("worker"),
                },
                OptionValue::Integer(10),
            )
            .is_err());
        assert!(state
            .set(
                &key("session.max_turns"),
                OptionScope::Global,
                OptionValue::String("ten".into()),
            )
            .is_err());
    }

    #[test]
    fn custom_definitions_are_idempotent_but_cannot_change_shape() {
        let mut state = OptionState::default().with_defaults().unwrap();
        let definition = OptionDefinition::new(
            "testing.capture_events",
            OptionValue::Bool(false),
            [OptionScopeKind::Global, OptionScopeKind::Session],
        );
        assert!(state.define(definition.clone()).unwrap());
        assert!(!state.define(definition).unwrap());
        assert!(state
            .define(OptionDefinition::new(
                "testing.capture_events",
                OptionValue::String("no".into()),
                [OptionScopeKind::Global],
            ))
            .is_err());
    }

    #[test]
    fn identifiers_reject_invalid_external_state() {
        assert!(serde_json::from_str::<OptionKey>("\"has space\"").is_err());
        assert!(serde_json::from_str::<OptionSubjectId>("\"   \"").is_err());
    }
}
