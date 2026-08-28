from pathlib import Path

options = Path("rust/crates/phenix-plugin-options/src/lib.rs")
text = options.read_text()

if "pub struct OptionAssignment" not in text:
    anchor = '''impl OptionDefinition {
    pub fn new(
'''
    assignment = '''#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OptionAssignment {
    pub key: OptionKey,
    pub scope: OptionScope,
    pub value: OptionValue,
}

'''
    text = text.replace(anchor, assignment + anchor, 1)

    text = text.replace(
        '''    Set {
        key: OptionKey,
        scope: OptionScope,
        value: OptionValue,
    },
''',
        '''    Configure {
        values: Vec<OptionAssignment>,
    },
    Set {
        key: OptionKey,
        scope: OptionScope,
        value: OptionValue,
    },
''',
        1,
    )
    text = text.replace(
        '''    Updated {
        key: OptionKey,
        scope: OptionScope,
    },
''',
        '''    Configured {
        count: usize,
    },
    Updated {
        key: OptionKey,
        scope: OptionScope,
    },
''',
        1,
    )

    text = text.replace(
        '''struct OptionState {
    definitions: BTreeMap<OptionKey, OptionDefinition>,
    values: BTreeMap<String, BTreeMap<OptionKey, OptionValue>>,
}
''',
        '''struct OptionState {
    definitions: BTreeMap<OptionKey, OptionDefinition>,
    #[serde(default)]
    configured_values: BTreeMap<String, BTreeMap<OptionKey, OptionValue>>,
    values: BTreeMap<String, BTreeMap<OptionKey, OptionValue>>,
}
''',
        1,
    )

    old_set = '''    fn set(
        &mut self,
        key: &OptionKey,
        scope: OptionScope,
        value: OptionValue,
    ) -> Result<(), String> {
        let definition = self
            .definitions
            .get(key)
            .ok_or_else(|| format!("unknown option: {key}"))?;
        if !definition.scopes.contains(&scope.kind()) {
            return Err(format!(
                "option {key} cannot be set at {:?} scope",
                scope.kind()
            ));
        }
        if !definition.default.has_same_type(&value) {
            return Err(format!(
                "option {key} value type does not match its definition"
            ));
        }
        self.values
            .entry(scope.storage_key())
            .or_default()
            .insert(key.clone(), value);
        Ok(())
    }
'''
    new_set = '''    fn configure(&mut self, values: Vec<OptionAssignment>) -> Result<bool, String> {
        let mut configured_values = BTreeMap::<String, BTreeMap<OptionKey, OptionValue>>::new();
        for assignment in values {
            self.validate_value(&assignment.key, &assignment.scope, &assignment.value)?;
            let scope = assignment.scope.storage_key();
            if configured_values
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
        if self.configured_values == configured_values {
            return Ok(false);
        }
        self.configured_values = configured_values;
        Ok(true)
    }

    fn set(
        &mut self,
        key: &OptionKey,
        scope: OptionScope,
        value: OptionValue,
    ) -> Result<(), String> {
        self.validate_value(key, &scope, &value)?;
        self.values
            .entry(scope.storage_key())
            .or_default()
            .insert(key.clone(), value);
        Ok(())
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
        if !definition.scopes.contains(&scope.kind()) {
            return Err(format!(
                "option {key} cannot be set at {:?} scope",
                scope.kind()
            ));
        }
        if !definition.default.has_same_type(value) {
            return Err(format!(
                "option {key} value type does not match its definition"
            ));
        }
        Ok(())
    }
'''
    if old_set not in text:
        raise SystemExit("options set anchor missing")
    text = text.replace(old_set, new_set, 1)

    text = text.replace(
        '''    fn value_at(&self, key: &OptionKey, scope: &OptionScope) -> Option<&OptionValue> {
        self.values.get(&scope.storage_key())?.get(key)
    }
''',
        '''    fn value_at(&self, key: &OptionKey, scope: &OptionScope) -> Option<&OptionValue> {
        let scope = scope.storage_key();
        self.values
            .get(&scope)
            .and_then(|values| values.get(key))
            .or_else(|| {
                self.configured_values
                    .get(&scope)
                    .and_then(|values| values.get(key))
            })
    }
''',
        1,
    )

    text = text.replace(
        '''            OptionCommand::GetDefinition { key } => OptionResponse::Definition {
                definition: state.definitions.get(&key).cloned(),
            },
            OptionCommand::Set { key, scope, value } => {
''',
        '''            OptionCommand::GetDefinition { key } => OptionResponse::Definition {
                definition: state.definitions.get(&key).cloned(),
            },
            OptionCommand::Configure { values } => {
                let count = values.len();
                changed = state.configure(values)?;
                OptionResponse::Configured { count }
            }
            OptionCommand::Set { key, scope, value } => {
''',
        1,
    )

    test_anchor = '''    #[test]
    fn option_scope_and_value_type_are_enforced_before_state_changes() {
'''
    new_test = '''    #[test]
    fn configuration_snapshot_replaces_removed_values_without_overwriting_runtime_state() {
        let mut state = OptionState::default().with_defaults().unwrap();
        let key = key("model.default");
        state
            .configure(vec![OptionAssignment {
                key: key.clone(),
                scope: OptionScope::Global,
                value: OptionValue::String("configured".into()),
            }])
            .unwrap();
        assert_eq!(
            state.resolve(&key, &OptionContext::default()).unwrap().value,
            OptionValue::String("configured".into())
        );

        state
            .set(
                &key,
                OptionScope::Global,
                OptionValue::String("runtime".into()),
            )
            .unwrap();
        assert_eq!(
            state.resolve(&key, &OptionContext::default()).unwrap().value,
            OptionValue::String("runtime".into())
        );

        state.configure(Vec::new()).unwrap();
        state.unset(&key, &OptionScope::Global).unwrap();
        assert_eq!(
            state.resolve(&key, &OptionContext::default()).unwrap().value,
            OptionValue::String("default".into())
        );
    }

'''
    text = text.replace(test_anchor, new_test + test_anchor, 1)

options.write_text(text)

runtime = Path("rust/crates/phenix-harness/src/runtime_config.rs")
text = runtime.read_text()
if "OptionAssignment" not in text.split("};", 1)[0]:
    text = text.replace(
        "    ModelTarget, OptionCommand, OptionKey, OptionResponse, OptionScope, OptionSubjectId,\n",
        "    ModelTarget, OptionAssignment, OptionCommand, OptionKey, OptionResponse, OptionScope,\n    OptionSubjectId,\n",
        1,
    )

old_directory = '''    let settings = directory.join("settings.json");
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
'''
new_directory = '''    let settings = directory.join("settings.json");
    let settings = if settings.is_file() {
        serde_json::from_slice(&fs::read(settings)?)?
    } else {
        SettingsConfiguration::default()
    };
    apply_settings_configuration(harness, settings)
}
'''
if old_directory not in text:
    raise SystemExit("runtime settings directory anchor missing")
text = text.replace(old_directory, new_directory, 1)

start = text.index("fn apply_settings_configuration(")
end = text.index("\npub(super) fn apply_runtime_config(", start)
new_apply = r'''fn apply_settings_configuration(
    harness: &mut PhenixHarness,
    settings: SettingsConfiguration,
) -> Result<(), Box<dyn Error>> {
    let mut values = Vec::new();
    for (key, value) in settings.global {
        values.push(option_assignment(key, OptionScope::Global, value)?);
    }
    for (session, settings) in settings.sessions {
        let session = OptionSubjectId::parse(session)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
        for (key, value) in settings {
            values.push(option_assignment(
                key,
                OptionScope::Session {
                    session: session.clone(),
                },
                value,
            )?);
        }
    }
    for (agent, settings) in settings.agents {
        let agent = OptionSubjectId::parse(agent)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
        for (key, value) in settings {
            values.push(option_assignment(
                key,
                OptionScope::Agent {
                    agent: agent.clone(),
                },
                value,
            )?);
        }
    }

    let output = harness.invoke(
        &options_service(),
        &serde_json::to_vec(&OptionCommand::Configure { values })?,
        &default_suite_authority(),
        None,
    )?;
    match serde_json::from_slice::<OptionResponse>(&output)? {
        OptionResponse::Configured { .. } => Ok(()),
        _ => Err("options service rejected settings configuration".into()),
    }
}

fn option_assignment(
    key: String,
    scope: OptionScope,
    value: SettingValue,
) -> Result<OptionAssignment, Box<dyn Error>> {
    Ok(OptionAssignment {
        key: OptionKey::parse(key)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?,
        scope,
        value: value.into(),
    })
}
'''
text = text[:start] + new_apply + text[end:]
runtime.write_text(text)

spec = Path("spec/plugin-options.md")
text = spec.read_text()
needle = "Startup loads built-in option definitions first, then applies `settings.json`. Resolution remains `agent > session > global > default`; the file only supplies scoped overrides."
replacement = needle + " The file is applied as one replaceable configuration snapshot, so removing an entry removes the prior file-derived value. Runtime `Set` values are a separate mutable layer and take precedence over configured values at the same scope."
if needle in text and "replaceable configuration snapshot" not in text:
    spec.write_text(text.replace(needle, replacement, 1))
