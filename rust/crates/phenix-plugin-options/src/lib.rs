#![forbid(unsafe_code)]

use phenix_core::{
    Authority, CapabilityId, ComponentExport, ComponentId, ComponentInterface, ComponentManifest,
    DurableSchema, PluginContext, PluginExecution, PluginHost, PluginId, PluginInstance,
    PluginManifest, ResourceNamespace, ServiceContribution, ServiceId, ServiceRole, TransactionOp,
};
use phenix_sdk::{
    options_service, OptionAssignment, OptionCommand, OptionContext, OptionDefinition, OptionKey,
    OptionResponse, OptionScope, OptionScopeKind, OptionStartupPrecedence, OptionSubjectId,
    OptionValue, OptionValueLayer, OptionValueSource, OptionsInterface, ResolvedOption,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const OPTIONS_PLUGIN: &str = "phenix.options";
pub const OPTIONS_COMPONENT: &str = "phenix.options";
const OPTIONS_NAMESPACE: &str = "phenix.options.state";
const STATE_KEY: &str = "options";
const PERSISTENCE_SCHEMA: &str = "kernel.persistence.schema";
const PERSISTENCE_READ: &str = "kernel.persistence.read";
const PERSISTENCE_WRITE: &str = "kernel.persistence.write";

type OptionsContext<'host, 'runtime> = PluginContext<'host, 'runtime, ()>;

fn plugin_context<'host, 'runtime>(
    host: &'host PluginHost<'runtime>,
) -> OptionsContext<'host, 'runtime> {
    PluginContext::new(host, (), (), ())
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
            schema: OptionsInterface::schema(),
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
        builtin_definition(
            "session.auto_create",
            OptionValue::Bool(true),
            [Global, Session, Agent],
        ),
        builtin_definition(
            "session.reuse_existing",
            OptionValue::Bool(true),
            [Global, Session, Agent],
        ),
        builtin_definition(
            "session.max_turns",
            OptionValue::Integer(0),
            [Global, Session],
        ),
        builtin_definition(
            "model.default",
            OptionValue::String("default".into()),
            [Global, Session, Agent],
        ),
        builtin_definition(
            "tools.confirmation",
            OptionValue::String("ask".into()),
            [Global, Session, Agent],
        ),
        builtin_definition(
            "skills.auto_load",
            OptionValue::Bool(true),
            [Global, Session, Agent],
        ),
        builtin_definition(
            "context.auto_load",
            OptionValue::Bool(true),
            [Global, Session, Agent],
        ),
        builtin_definition(
            "agent.max_parallel_tasks",
            OptionValue::Integer(1),
            [Global, Agent],
        ),
    ]
}

fn builtin_definition(
    key: &str,
    default: OptionValue,
    scopes: impl IntoIterator<Item = OptionScopeKind>,
) -> OptionDefinition {
    OptionDefinition::new(
        OptionKey::parse(key).expect("static option key is valid"),
        default,
        scopes,
    )
    .expect("static option definition is valid")
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
struct OptionState {
    definitions: BTreeMap<OptionKey, OptionDefinition>,
    #[serde(default)]
    file_values: BTreeMap<String, BTreeMap<OptionKey, OptionValue>>,
    #[serde(default)]
    nix_values: BTreeMap<String, BTreeMap<OptionKey, OptionValue>>,
    #[serde(default)]
    startup_precedence: OptionStartupPrecedence,
    values: BTreeMap<String, BTreeMap<OptionKey, OptionValue>>,
}

impl OptionState {
    fn with_defaults(mut self) -> Result<Self, String> {
        for definition in default_option_definitions() {
            let key = definition.key().clone();
            match self.definitions.get(&key) {
                Some(existing) if existing == &definition => {}
                Some(_) => {
                    return Err(format!(
                        "persisted definition for {key} conflicts with built-in definition"
                    ));
                }
                None => {
                    self.definitions.insert(key, definition);
                }
            }
        }
        Ok(self)
    }

    fn define(&mut self, definition: OptionDefinition) -> Result<bool, String> {
        let key = definition.key().clone();
        match self.definitions.get(&key) {
            Some(existing) if existing == &definition => Ok(false),
            Some(_) => Err(format!("option {key} is already defined differently")),
            None => {
                self.definitions.insert(key, definition);
                Ok(true)
            }
        }
    }

    fn configure(
        &mut self,
        file_values: Vec<OptionAssignment>,
        nix_values: Vec<OptionAssignment>,
        precedence: OptionStartupPrecedence,
    ) -> Result<bool, String> {
        let file_values = self.configuration_layer(file_values)?;
        let nix_values = self.configuration_layer(nix_values)?;
        if self.file_values == file_values
            && self.nix_values == nix_values
            && self.startup_precedence == precedence
        {
            return Ok(false);
        }
        self.file_values = file_values;
        self.nix_values = nix_values;
        self.startup_precedence = precedence;
        Ok(true)
    }

    fn configuration_layer(
        &self,
        values: Vec<OptionAssignment>,
    ) -> Result<BTreeMap<String, BTreeMap<OptionKey, OptionValue>>, String> {
        let mut layer = BTreeMap::<String, BTreeMap<OptionKey, OptionValue>>::new();
        for assignment in values {
            self.validate_value(&assignment.key, &assignment.scope, &assignment.value)?;
            let scope = scope_storage_key(&assignment.scope);
            if layer
                .entry(scope.clone())
                .or_default()
                .insert(assignment.key.clone(), assignment.value)
                .is_some()
            {
                return Err(format!(
                    "option {} is configured more than once at {scope}",
                    assignment.key
                ));
            }
        }
        Ok(layer)
    }

    fn set(
        &mut self,
        key: &OptionKey,
        scope: OptionScope,
        value: OptionValue,
    ) -> Result<bool, String> {
        self.validate_value(key, &scope, &value)?;
        let values = self.values.entry(scope_storage_key(&scope)).or_default();
        if values.get(key) == Some(&value) {
            return Ok(false);
        }
        values.insert(key.clone(), value);
        Ok(true)
    }

    fn validate_value(
        &self,
        key: &OptionKey,
        scope: &OptionScope,
        value: &OptionValue,
    ) -> Result<(), String> {
        let definition = self
            .definitions
            .get(key)
            .ok_or_else(|| format!("unknown option: {key}"))?;
        let kind = scope_kind(scope);
        if !definition.scopes().any(|allowed| allowed == kind) {
            return Err(format!("option {key} cannot be set at {kind:?} scope"));
        }
        if !same_value_type(definition.default_value(), value) {
            return Err(format!(
                "option {key} value type does not match its definition"
            ));
        }
        Ok(())
    }

    fn unset(&mut self, key: &OptionKey, scope: &OptionScope) -> Result<bool, String> {
        if !self.definitions.contains_key(key) {
            return Err(format!("unknown option: {key}"));
        }
        let storage_key = scope_storage_key(scope);
        let Some(values) = self.values.get_mut(&storage_key) else {
            return Ok(false);
        };
        let removed = values.remove(key).is_some();
        if values.is_empty() {
            self.values.remove(&storage_key);
        }
        Ok(removed)
    }

    fn resolve(&self, key: &OptionKey, context: &OptionContext) -> Result<ResolvedOption, String> {
        let definition = self
            .definitions
            .get(key)
            .ok_or_else(|| format!("unknown option: {key}"))?;

        if let Some((value, source)) = resolve_layer(&self.values, key, context) {
            return Ok(ResolvedOption {
                key: key.clone(),
                value: value.clone(),
                source,
                layer: OptionValueLayer::Runtime,
            });
        }

        let startup_layers = match self.startup_precedence {
            OptionStartupPrecedence::Nix => [
                (OptionValueLayer::Nix, &self.nix_values),
                (OptionValueLayer::File, &self.file_values),
            ],
            OptionStartupPrecedence::File => [
                (OptionValueLayer::File, &self.file_values),
                (OptionValueLayer::Nix, &self.nix_values),
            ],
        };
        for (layer, values) in startup_layers {
            if let Some((value, source)) = resolve_layer(values, key, context) {
                return Ok(ResolvedOption {
                    key: key.clone(),
                    value: value.clone(),
                    source,
                    layer,
                });
            }
        }

        Ok(ResolvedOption {
            key: key.clone(),
            value: definition.default_value().clone(),
            source: OptionValueSource::Default,
            layer: OptionValueLayer::Default,
        })
    }
}

fn scope_kind(scope: &OptionScope) -> OptionScopeKind {
    match scope {
        OptionScope::Global => OptionScopeKind::Global,
        OptionScope::Session(_) => OptionScopeKind::Session,
        OptionScope::Agent(_) => OptionScopeKind::Agent,
    }
}

fn scope_storage_key(scope: &OptionScope) -> String {
    match scope {
        OptionScope::Global => "global".into(),
        OptionScope::Session(session) => format!("session:{}", encode_subject(session.as_str())),
        OptionScope::Agent(agent) => format!("agent:{}", encode_subject(agent.as_str())),
    }
}

fn same_value_type(left: &OptionValue, right: &OptionValue) -> bool {
    matches!(
        (left, right),
        (OptionValue::Bool(_), OptionValue::Bool(_))
            | (OptionValue::Integer(_), OptionValue::Integer(_))
            | (OptionValue::String(_), OptionValue::String(_))
    )
}

fn resolve_layer<'a>(
    values: &'a BTreeMap<String, BTreeMap<OptionKey, OptionValue>>,
    key: &OptionKey,
    context: &OptionContext,
) -> Option<(&'a OptionValue, OptionValueSource)> {
    if let Some(agent) = &context.agent {
        let scope = OptionScope::Agent(agent.clone());
        if let Some(value) = value_at(values, key, &scope) {
            return Some((value, OptionValueSource::Agent));
        }
    }
    if let Some(session) = &context.session {
        let scope = OptionScope::Session(session.clone());
        if let Some(value) = value_at(values, key, &scope) {
            return Some((value, OptionValueSource::Session));
        }
    }
    value_at(values, key, &OptionScope::Global).map(|value| (value, OptionValueSource::Global))
}

fn value_at<'a>(
    values: &'a BTreeMap<String, BTreeMap<OptionKey, OptionValue>>,
    key: &OptionKey,
    scope: &OptionScope,
) -> Option<&'a OptionValue> {
    values.get(&scope_storage_key(scope))?.get(key)
}

struct OptionsPlugin;

impl PluginInstance for OptionsPlugin {
    fn start(&mut self, host: &PluginHost<'_>) -> Result<(), String> {
        plugin_context(host)
            .kernel
            .register_durable_schema(&DurableSchema::new(options_namespace(), 1))
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
        let context = plugin_context(host);
        let interface = OptionsInterface::interface_id();
        let command = context
            .kernel
            .decode_projected::<OptionCommand>(&interface, input)
            .map_err(|error| error.to_string())?;
        let response = handle(&context, command)?;
        context
            .kernel
            .encode_value(&response)
            .map_err(|error| error.to_string())
    }
}

fn handle(
    context: &OptionsContext<'_, '_>,
    command: OptionCommand,
) -> Result<OptionResponse, String> {
    let (mut state, raw) = load_state(context)?;
    let response = match command {
        OptionCommand::Define { definition } => {
            let changed = state.define(definition.clone())?;
            if changed {
                save_state(context, raw, &state)?;
            }
            return Ok(OptionResponse::Defined { definition });
        }
        OptionCommand::GetDefinition { key } => OptionResponse::Definition {
            definition: state.definitions.get(&key).cloned(),
        },
        OptionCommand::Configure {
            file_values,
            nix_values,
            precedence,
        } => {
            let count = file_values.len() + nix_values.len();
            let changed = state.configure(file_values, nix_values, precedence)?;
            if changed {
                save_state(context, raw, &state)?;
            }
            return Ok(OptionResponse::Configured { count });
        }
        OptionCommand::Set { key, scope, value } => {
            let changed = state.set(&key, scope.clone(), value)?;
            if changed {
                save_state(context, raw, &state)?;
            }
            return Ok(OptionResponse::Updated { key, scope });
        }
        OptionCommand::Unset { key, scope } => {
            let changed = state.unset(&key, &scope)?;
            if changed {
                save_state(context, raw, &state)?;
            }
            return Ok(OptionResponse::Updated { key, scope });
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
    Ok(response)
}

fn load_state(context: &OptionsContext<'_, '_>) -> Result<(OptionState, Option<Vec<u8>>), String> {
    let raw = context
        .kernel
        .read_durable(&options_namespace(), STATE_KEY)
        .map_err(|error| error.to_string())?;
    let state = match raw.as_deref() {
        Some(bytes) => serde_json::from_slice(bytes).map_err(|error| error.to_string())?,
        None => OptionState::default(),
    };
    Ok((state.with_defaults()?, raw))
}

fn save_state(
    context: &OptionsContext<'_, '_>,
    old: Option<Vec<u8>>,
    state: &OptionState,
) -> Result<(), String> {
    context
        .kernel
        .transact_durable(
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

fn encode_subject(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(value.len() * 2);
    for byte in value.bytes() {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
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
            .set(
                &key,
                OptionScope::Global,
                OptionValue::String("global".into()),
            )
            .unwrap();
        state
            .set(
                &key,
                OptionScope::Session(subject("s1")),
                OptionValue::String("session".into()),
            )
            .unwrap();
        state
            .set(
                &key,
                OptionScope::Agent(subject("a1")),
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
        assert_eq!(resolved.layer, OptionValueLayer::Runtime);
        serde_json::to_vec(&state).unwrap();
    }

    #[test]
    fn runtime_values_win_before_startup_source_and_scope_precedence() {
        let mut state = OptionState::default().with_defaults().unwrap();
        let key = key("model.default");
        state
            .configure(
                vec![OptionAssignment {
                    key: key.clone(),
                    scope: OptionScope::Agent(subject("worker")),
                    value: OptionValue::String("file".into()),
                }],
                vec![OptionAssignment {
                    key: key.clone(),
                    scope: OptionScope::Global,
                    value: OptionValue::String("nix".into()),
                }],
                OptionStartupPrecedence::Nix,
            )
            .unwrap();
        let context = OptionContext {
            session: None,
            agent: Some(subject("worker")),
        };
        let resolved = state.resolve(&key, &context).unwrap();
        assert_eq!(resolved.value, OptionValue::String("nix".into()));
        assert_eq!(resolved.source, OptionValueSource::Global);
        assert_eq!(resolved.layer, OptionValueLayer::Nix);

        state
            .set(
                &key,
                OptionScope::Global,
                OptionValue::String("runtime".into()),
            )
            .unwrap();
        let resolved = state.resolve(&key, &context).unwrap();
        assert_eq!(resolved.value, OptionValue::String("runtime".into()));
        assert_eq!(resolved.layer, OptionValueLayer::Runtime);
    }

    #[test]
    fn file_precedence_and_configuration_removal_are_declarative() {
        let mut state = OptionState::default().with_defaults().unwrap();
        let key = key("model.default");
        state
            .configure(
                vec![OptionAssignment {
                    key: key.clone(),
                    scope: OptionScope::Global,
                    value: OptionValue::String("file".into()),
                }],
                vec![OptionAssignment {
                    key: key.clone(),
                    scope: OptionScope::Global,
                    value: OptionValue::String("nix".into()),
                }],
                OptionStartupPrecedence::File,
            )
            .unwrap();
        let resolved = state.resolve(&key, &OptionContext::default()).unwrap();
        assert_eq!(resolved.value, OptionValue::String("file".into()));
        assert_eq!(resolved.layer, OptionValueLayer::File);

        state
            .configure(Vec::new(), Vec::new(), OptionStartupPrecedence::Nix)
            .unwrap();
        let resolved = state.resolve(&key, &OptionContext::default()).unwrap();
        assert_eq!(resolved.value, OptionValue::String("default".into()));
        assert_eq!(resolved.layer, OptionValueLayer::Default);
    }

    #[test]
    fn option_scope_and_value_type_are_enforced_before_state_changes() {
        let mut state = OptionState::default().with_defaults().unwrap();
        assert!(state
            .set(
                &key("session.max_turns"),
                OptionScope::Agent(subject("worker")),
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
            key("testing.capture_events"),
            OptionValue::Bool(false),
            [OptionScopeKind::Global, OptionScopeKind::Session],
        )
        .unwrap();
        assert!(state.define(definition.clone()).unwrap());
        assert!(!state.define(definition).unwrap());
        assert!(state
            .define(
                OptionDefinition::new(
                    key("testing.capture_events"),
                    OptionValue::String("no".into()),
                    [OptionScopeKind::Global],
                )
                .unwrap()
            )
            .is_err());
    }

    #[test]
    fn definitions_cannot_deserialize_without_a_writable_scope() {
        assert!(serde_json::from_str::<OptionDefinition>(
            r#"{"key":"testing.capture_events","default":{"type":"bool","value":false},"scopes":[]}"#,
        )
        .is_err());
    }

    #[test]
    fn repeated_updates_do_not_change_state() {
        let mut state = OptionState::default().with_defaults().unwrap();
        let key = key("model.default");
        assert!(state
            .set(
                &key,
                OptionScope::Global,
                OptionValue::String("configured".into()),
            )
            .unwrap());
        assert!(!state
            .set(
                &key,
                OptionScope::Global,
                OptionValue::String("configured".into()),
            )
            .unwrap());
        assert!(state.unset(&key, &OptionScope::Global).unwrap());
        assert!(!state.unset(&key, &OptionScope::Global).unwrap());
    }

    #[test]
    fn identifiers_reject_invalid_external_state() {
        assert!(serde_json::from_str::<OptionKey>("\"has space\"").is_err());
        assert!(serde_json::from_str::<OptionSubjectId>("\"   \"").is_err());
    }

    #[test]
    fn scopes_have_unambiguous_wire_forms() {
        for (scope, expected) in [
            (OptionScope::Global, serde_json::json!("global")),
            (
                OptionScope::Session(subject("session-1")),
                serde_json::json!({ "session": "session-1" }),
            ),
            (
                OptionScope::Agent(subject("worker")),
                serde_json::json!({ "agent": "worker" }),
            ),
        ] {
            assert_eq!(serde_json::to_value(&scope).unwrap(), expected);
            assert_eq!(
                serde_json::from_value::<OptionScope>(expected).unwrap(),
                scope
            );
        }
    }
}
