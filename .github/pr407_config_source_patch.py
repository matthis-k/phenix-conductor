from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"missing anchor: {label}")
    return text.replace(old, new, 1)


# Options: parse settings into typed boundary values and expose bulk runtime apply.
path = Path("rust/crates/phenix-plugin-options/src/lib.rs")
text = path.read_text()

anchor = '''impl OptionValue {
    fn has_same_type(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Bool(_), Self::Bool(_))
                | (Self::Integer(_), Self::Integer(_))
                | (Self::String(_), Self::String(_))
        )
    }
}
'''
insert = anchor + '''
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum OptionSettingValue {
    Bool(bool),
    Integer(i64),
    String(String),
}

impl From<OptionSettingValue> for OptionValue {
    fn from(value: OptionSettingValue) -> Self {
        match value {
            OptionSettingValue::Bool(value) => Self::Bool(value),
            OptionSettingValue::Integer(value) => Self::Integer(value),
            OptionSettingValue::String(value) => Self::String(value),
        }
    }
}
'''
text = replace_once(text, anchor, insert, "setting value")

anchor = '''#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OptionAssignment {
    pub key: OptionKey,
    pub scope: OptionScope,
    pub value: OptionValue,
}
'''
insert = anchor + '''
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OptionSettings {
    #[serde(default)]
    pub global: BTreeMap<OptionKey, OptionSettingValue>,
    #[serde(default)]
    pub sessions: BTreeMap<OptionSubjectId, BTreeMap<OptionKey, OptionSettingValue>>,
    #[serde(default)]
    pub agents: BTreeMap<OptionSubjectId, BTreeMap<OptionKey, OptionSettingValue>>,
}

impl OptionSettings {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.global.is_empty() && self.sessions.is_empty() && self.agents.is_empty()
    }

    fn into_assignments(self) -> Vec<OptionAssignment> {
        let mut assignments = Vec::new();
        assignments.extend(self.global.into_iter().map(|(key, value)| OptionAssignment {
            key,
            scope: OptionScope::Global,
            value: value.into(),
        }));
        for (session, values) in self.sessions {
            assignments.extend(values.into_iter().map(|(key, value)| OptionAssignment {
                key,
                scope: OptionScope::Session {
                    session: session.clone(),
                },
                value: value.into(),
            }));
        }
        for (agent, values) in self.agents {
            assignments.extend(values.into_iter().map(|(key, value)| OptionAssignment {
                key,
                scope: OptionScope::Agent {
                    agent: agent.clone(),
                },
                value: value.into(),
            }));
        }
        assignments
    }
}
'''
text = replace_once(text, anchor, insert, "typed settings")

text = replace_once(
    text,
    '''    Configure {
        file_values: Vec<OptionAssignment>,
        nix_values: Vec<OptionAssignment>,
        precedence: OptionStartupPrecedence,
    },
''',
    '''    Configure {
        file: OptionSettings,
        nix: OptionSettings,
        precedence: OptionStartupPrecedence,
    },
    ApplySettings {
        settings: OptionSettings,
    },
''',
    "option commands",
)

anchor = '''    fn set(
        &mut self,
        key: &OptionKey,
        scope: OptionScope,
        value: OptionValue,
    ) -> Result<(), String> {
'''
method = '''    fn apply_settings(&mut self, values: Vec<OptionAssignment>) -> Result<bool, String> {
        let layer = self.configuration_layer(values)?;
        let mut changed = false;
        for (scope, values) in layer {
            let current = self.values.entry(scope).or_default();
            for (key, value) in values {
                if current.get(&key) == Some(&value) {
                    continue;
                }
                current.insert(key, value);
                changed = true;
            }
        }
        Ok(changed)
    }

'''
text = replace_once(text, anchor, method + anchor, "apply settings state")

text = replace_once(
    text,
    '''            OptionCommand::Configure {
                file_values,
                nix_values,
                precedence,
            } => {
                let count = file_values.len() + nix_values.len();
                changed = state.configure(file_values, nix_values, precedence)?;
                OptionResponse::Configured { count }
            }
''',
    '''            OptionCommand::Configure {
                file,
                nix,
                precedence,
            } => {
                let file_values = file.into_assignments();
                let nix_values = nix.into_assignments();
                let count = file_values.len() + nix_values.len();
                changed = state.configure(file_values, nix_values, precedence)?;
                OptionResponse::Configured { count }
            }
            OptionCommand::ApplySettings { settings } => {
                let values = settings.into_assignments();
                let count = values.len();
                changed = state.apply_settings(values)?;
                OptionResponse::Configured { count }
            }
''',
    "option dispatch",
)

# Add a boundary + non-destructive runtime merge regression.
anchor = '''    #[test]
    fn option_scope_and_value_type_are_enforced_before_state_changes() {
'''
test = '''    #[test]
    fn settings_parse_at_the_boundary_and_apply_without_clearing_runtime_values() {
        let settings: OptionSettings = serde_json::from_value(serde_json::json!({
            "global": {"session.auto_create": false},
            "agents": {"worker": {"agent.max_parallel_tasks": 4}}
        }))
        .unwrap();
        assert!(serde_json::from_value::<OptionSettings>(serde_json::json!({
            "global": {"invalid key": true}
        }))
        .is_err());

        let mut state = OptionState::default().with_defaults().unwrap();
        state
            .set(
                &key("model.default"),
                OptionScope::Global,
                OptionValue::String("runtime-model".into()),
            )
            .unwrap();
        state.apply_settings(settings.into_assignments()).unwrap();

        assert_eq!(
            state
                .resolve(&key("session.auto_create"), &OptionContext::default())
                .unwrap()
                .value,
            OptionValue::Bool(false)
        );
        assert_eq!(
            state
                .resolve(&key("model.default"), &OptionContext::default())
                .unwrap()
                .value,
            OptionValue::String("runtime-model".into())
        );
    }

'''
text = replace_once(text, anchor, test + anchor, "settings apply test")
path.write_text(text)

# Catalog exports the typed settings document.
path = Path("rust/crates/phenix-plugin-catalog/src/lib.rs")
text = path.read_text()
text = replace_once(
    text,
    '''    OptionDefinition, OptionKey, OptionResponse, OptionScope, OptionScopeKind,
    OptionStartupPrecedence, OptionSubjectId, OptionValue, OptionValueLayer, OptionValueSource,
''',
    '''    OptionDefinition, OptionKey, OptionResponse, OptionScope, OptionScopeKind,
    OptionSettingValue, OptionSettings, OptionStartupPrecedence, OptionSubjectId, OptionValue,
    OptionValueLayer, OptionValueSource,
''',
    "catalog settings exports",
)
path.write_text(text)

# Harness startup consumes the same typed settings document as SDK callers.
path = Path("rust/crates/phenix-harness/src/runtime_config.rs")
text = path.read_text()
text = replace_once(
    text,
    '''    ModelTarget, OptionAssignment, OptionCommand, OptionKey, OptionResponse, OptionScope,
    OptionStartupPrecedence, OptionSubjectId, OptionValue, OrchestrationDefinition, RoutingProfile,
''',
    '''    ModelTarget, OptionCommand, OptionResponse, OptionSettings, OptionStartupPrecedence,
    OrchestrationDefinition, RoutingProfile,
''',
    "runtime settings imports",
)
text = replace_once(
    text,
    '''use std::{collections::BTreeMap, error::Error, fs, io, path::Path};
''',
    '''use std::{collections::BTreeMap, error::Error, fs, path::Path};
''',
    "runtime std imports",
)
start = text.index("#[derive(Debug, Default, Deserialize)]\n#[serde(deny_unknown_fields)]\nstruct SettingsConfiguration")
end = text.index("\n#[derive(Debug, Deserialize)]\nstruct RuntimeModelTarget", start)
text = text[:start] + text[end + 1:]

text = replace_once(
    text,
    '''    let file_settings = read_optional_settings(file_path.as_deref())?;
    let nix_settings = read_optional_settings(nix_settings)?;
    let file_values = settings_assignments(file_settings)?;
    let nix_values = settings_assignments(nix_settings)?;
''',
    '''    let file = read_optional_settings(file_path.as_deref())?;
    let nix = read_optional_settings(nix_settings)?;
''',
    "startup settings parse",
)
text = replace_once(
    text,
    '''        if file_values.is_empty() && nix_values.is_empty() {
''',
    '''        if file.is_empty() && nix.is_empty() {
''',
    "empty startup settings",
)
text = replace_once(
    text,
    '''        &serde_json::to_vec(&OptionCommand::Configure {
            file_values,
            nix_values,
            precedence,
        })?,
''',
    '''        &serde_json::to_vec(&OptionCommand::Configure {
            file,
            nix,
            precedence,
        })?,
''',
    "startup configure command",
)
start = text.index("fn read_optional_settings(")
end = text.index("\npub(super) fn apply_runtime_config(", start)
replacement = '''fn read_optional_settings(path: Option<&Path>) -> Result<OptionSettings, Box<dyn Error>> {
    let Some(path) = path else {
        return Ok(OptionSettings::default());
    };
    if !path.exists() {
        return Ok(OptionSettings::default());
    }
    if !path.is_file() {
        return Err(format!("settings path is not a file: {}", path.display()).into());
    }
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}
'''
text = text[:start] + replacement + text[end:]
path.write_text(text)

# SDK config returns the actual source path instead of file contents.
path = Path("rust/crates/phenix-plugin-sdk/src/lib.rs")
text = path.read_text()
text = replace_once(
    text,
    '''use std::{
    collections::BTreeSet,
    env, fs,
    path::{Component, Path, PathBuf},
};
''',
    '''use std::{
    collections::BTreeSet,
    env,
    path::{Component, Path, PathBuf},
};
''',
    "sdk std imports",
)
text = replace_once(
    text,
    '''pub enum SdkConfigCommand {
    Read { path: String },
}
''',
    '''pub enum SdkConfigCommand {
    GetPath { path: String },
}
''',
    "config command",
)
text = replace_once(
    text,
    '''pub enum SdkConfigResponse {
    File { content: Vec<u8> },
}
''',
    '''pub enum SdkConfigResponse {
    Path { path: String },
}
''',
    "config response",
)
old = '''fn config_command(root: &Path, command: SdkConfigCommand) -> Result<SdkConfigResponse, String> {
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
'''
new = '''fn config_command(root: &Path, command: SdkConfigCommand) -> Result<SdkConfigResponse, String> {
    match command {
        SdkConfigCommand::GetPath { path } => {
            let path = config_path(root, &path)?;
            let path = path
                .into_os_string()
                .into_string()
                .map_err(|_| "config path is not valid UTF-8".to_owned())?;
            Ok(SdkConfigResponse::Path { path })
        }
    }
}
'''
text = replace_once(text, old, new, "config path resolution")

# Ensure the config interface is included in the SDK module regression and add path semantics coverage.
text = replace_once(
    text,
    '''            SdkSkillsInterface::interface_id(),
            ContextInterface::interface_id(),
''',
    '''            SdkSkillsInterface::interface_id(),
            SdkConfigInterface::interface_id(),
            ContextInterface::interface_id(),
''',
    "config interface regression",
)
anchor = '''    #[test]
    fn session_open_uses_scoped_options() {
'''
test = '''    #[test]
    fn config_get_path_preserves_source_identity_without_reading_it() {
        let root = Path::new("config-root");
        let response = config_command(
            root,
            SdkConfigCommand::GetPath {
                path: "settings.json".into(),
            },
        )
        .unwrap();
        assert_eq!(
            response,
            SdkConfigResponse::Path {
                path: root.join("settings.json").to_string_lossy().into_owned(),
            }
        );
        assert!(config_command(
            root,
            SdkConfigCommand::GetPath {
                path: "../settings.json".into(),
            },
        )
        .is_err());
    }

'''
text = replace_once(text, anchor, test + anchor, "config path test")
path.write_text(text)

# Product test consumes the returned path, proving the file remains the file-side source.
path = Path("modules/plugin-packaging.nix")
text = path.read_text()
old = '''            printf '%s\\n' '{"id":2,"service":"phenix.sdk.config@1","input":{"operation":"read","path":"settings.json"}}' \\
              | "${settingsComposition}/bin/phenix" > "$TMPDIR/settings-config.json"
            jq -e '.status == "ok" and .output.result == "file" and ((.output.content | implode | fromjson).global["session.auto_create"] == true)' \\
              "$TMPDIR/settings-config.json" >/dev/null
'''
new = '''            printf '%s\\n' '{"id":2,"service":"phenix.sdk.config@1","input":{"operation":"get_path","path":"settings.json"}}' \\
              | "${settingsComposition}/bin/phenix" > "$TMPDIR/settings-config.json"
            settings_path="$(jq -r '.output.path' "$TMPDIR/settings-config.json")"
            jq -e '.status == "ok" and .output.result == "path" and (.output.path | endswith("/settings.json"))' \\
              "$TMPDIR/settings-config.json" >/dev/null
            jq -e '.global["session.auto_create"] == true' "$settings_path" >/dev/null
'''
text = replace_once(text, old, new, "product config path test")
path.write_text(text)

# Specs document the intended high-level SDK flow and non-destructive semantics.
path = Path("spec/plugin-sdk.md")
text = path.read_text()
old = '''## Configuration files

The default `phenix` SDK exposes `phenix.sdk.config@1`. `Read` accepts a plain relative path and reads it under `PHENIX_CONFIG_DIR`. Absolute paths and `.` or `..` components are rejected.

The Nix wrapper owns `PHENIX_CONFIG_DIR`. It points at the selected user configuration directory, or the packaged default directory when none is supplied. SDK code receives relative names such as `settings.json`; it does not discover host configuration directories. Nix-generated settings are a separate startup source and are not written into that directory.
'''
new = '''## Configuration files

The default `phenix` SDK exposes `phenix.sdk.config@1`. `GetPath` accepts a plain relative path and returns its location under `PHENIX_CONFIG_DIR`. Absolute paths and `.` or `..` components are rejected. The service does not read, parse, create, truncate, or rewrite the file.

The path is the canonical transport representation because it preserves source identity and survives the typed cross-process boundary. An in-process binding may additionally open the file read-only, but it must retain the path. It must not replace a recoverable path with a bare handle.

Language bindings should expose the flow directly:

```text
settings = phenix.get_config_path("settings.json").parse()
phenix.apply_settings(settings)
```

`parse()` produces the typed settings document owned by `phenix.options`. `apply_settings()` maps to the options interface and applies those values to the runtime layer. It never writes back to the source file.

The Nix wrapper owns `PHENIX_CONFIG_DIR`. It points at the selected user configuration directory, or the packaged default directory when none is supplied. Nix-generated settings are a separate startup source and are not written into that directory.
'''
text = replace_once(text, old, new, "sdk config spec")
path.write_text(text)

path = Path("spec/plugin-options.md")
text = path.read_text()
text = text.replace(
    '''define\nget_definition\nset\nunset\nresolve\nlist\n''',
    '''define\nget_definition\nconfigure\napply_settings\nset\nunset\nresolve\nlist\n''',
    1,
)
anchor = '''Option state is durable and owned by `phenix.options`. Core only enforces the ordinary persistence namespace and authority rules.
'''
insert = anchor + '''
`OptionSettings` is the typed settings document shared by startup loading and SDK callers. Deserialization parses option keys and scope subject identities before the document enters runtime state.

`apply_settings` bulk-applies a parsed document to the runtime layer. It validates the complete document before changing state, preserves unrelated runtime values, and never modifies the source file it was parsed from. Runtime values therefore remain above both startup sources.
'''
text = replace_once(text, anchor, insert, "options apply spec")
path.write_text(text)
