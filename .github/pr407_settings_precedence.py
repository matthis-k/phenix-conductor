from pathlib import Path


def require_replace(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"missing anchor: {label}")
    return text.replace(old, new, 1)


# Typed option layers and source-first resolution.
path = Path("rust/crates/phenix-plugin-options/src/lib.rs")
text = path.read_text()
if "OptionStartupPrecedence" not in text:
    anchor = '''#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OptionDefinition {
'''
    types = '''#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OptionStartupPrecedence {
    #[default]
    Nix,
    File,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OptionValueLayer {
    Runtime,
    Nix,
    File,
    Default,
}

'''
    text = require_replace(text, anchor, types + anchor, "option layer types")

    text = require_replace(
        text,
        '''pub struct ResolvedOption {
    pub key: OptionKey,
    pub value: OptionValue,
    pub source: OptionValueSource,
}
''',
        '''pub struct ResolvedOption {
    pub key: OptionKey,
    pub value: OptionValue,
    pub source: OptionValueSource,
    pub layer: OptionValueLayer,
}
''',
        "resolved option layer",
    )

    text = require_replace(
        text,
        '''    Configure {
        values: Vec<OptionAssignment>,
    },
''',
        '''    Configure {
        file_values: Vec<OptionAssignment>,
        nix_values: Vec<OptionAssignment>,
        precedence: OptionStartupPrecedence,
    },
''',
        "configure command",
    )

    text = require_replace(
        text,
        '''struct OptionState {
    definitions: BTreeMap<OptionKey, OptionDefinition>,
    #[serde(default)]
    configured_values: BTreeMap<String, BTreeMap<OptionKey, OptionValue>>,
    values: BTreeMap<String, BTreeMap<OptionKey, OptionValue>>,
}
''',
        '''struct OptionState {
    definitions: BTreeMap<OptionKey, OptionDefinition>,
    #[serde(default)]
    file_values: BTreeMap<String, BTreeMap<OptionKey, OptionValue>>,
    #[serde(default)]
    nix_values: BTreeMap<String, BTreeMap<OptionKey, OptionValue>>,
    #[serde(default)]
    startup_precedence: OptionStartupPrecedence,
    values: BTreeMap<String, BTreeMap<OptionKey, OptionValue>>,
}
''',
        "option state layers",
    )

    start = text.index("    fn configure(")
    end = text.index("\n    fn set(", start)
    configure = '''    fn configure(
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
            let scope = assignment.scope.storage_key();
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
'''
    text = text[:start] + configure + text[end:]

    start = text.index("    fn resolve(&self")
    end = text.index("\n}\n\nstruct OptionsPlugin", start)
    resolve = '''    fn resolve(&self, key: &OptionKey, context: &OptionContext) -> Result<ResolvedOption, String> {
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
            value: definition.default.clone(),
            source: OptionValueSource::Default,
            layer: OptionValueLayer::Default,
        })
    }
'''
    text = text[:start] + resolve + text[end:]

    helper_anchor = "\nstruct OptionsPlugin;\n"
    helper = '''
fn resolve_layer<'a>(
    values: &'a BTreeMap<String, BTreeMap<OptionKey, OptionValue>>,
    key: &OptionKey,
    context: &OptionContext,
) -> Option<(&'a OptionValue, OptionValueSource)> {
    if let Some(agent) = &context.agent {
        let scope = OptionScope::Agent {
            agent: agent.clone(),
        };
        if let Some(value) = value_at(values, key, &scope) {
            return Some((value, OptionValueSource::Agent));
        }
    }
    if let Some(session) = &context.session {
        let scope = OptionScope::Session {
            session: session.clone(),
        };
        if let Some(value) = value_at(values, key, &scope) {
            return Some((value, OptionValueSource::Session));
        }
    }
    value_at(values, key, &OptionScope::Global)
        .map(|value| (value, OptionValueSource::Global))
}

fn value_at<'a>(
    values: &'a BTreeMap<String, BTreeMap<OptionKey, OptionValue>>,
    key: &OptionKey,
    scope: &OptionScope,
) -> Option<&'a OptionValue> {
    values.get(&scope.storage_key())?.get(key)
}
'''
    text = require_replace(text, helper_anchor, helper + helper_anchor, "layer resolution helper")

    text = require_replace(
        text,
        '''            OptionCommand::Configure { values } => {
                let count = values.len();
                changed = state.configure(values)?;
                OptionResponse::Configured { count }
            }
''',
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
        "configure dispatch",
    )

    # Replace the configuration test with precedence coverage.
    test_start = text.index("    #[test]\n    fn configuration_snapshot_replaces_removed_values_without_overwriting_runtime_state()")
    test_end = text.index("\n    #[test]\n    fn option_scope_and_value_type_are_enforced_before_state_changes()", test_start)
    tests = '''    #[test]
    fn runtime_values_win_before_startup_source_and_scope_precedence() {
        let mut state = OptionState::default().with_defaults().unwrap();
        let key = key("model.default");
        state
            .configure(
                vec![OptionAssignment {
                    key: key.clone(),
                    scope: OptionScope::Agent {
                        agent: subject("worker"),
                    },
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
'''
    text = text[:test_start] + tests + text[test_end:]

    # Existing resolution test should expose the runtime layer too.
    old = '''        assert_eq!(resolved.value, OptionValue::String("agent".into()));
        assert_eq!(resolved.source, OptionValueSource::Agent);
'''
    new = old + "        assert_eq!(resolved.layer, OptionValueLayer::Runtime);\n"
    text = require_replace(text, old, new, "runtime layer assertion")

path.write_text(text)

# Re-export the new public types.
path = Path("rust/crates/phenix-plugin-catalog/src/lib.rs")
text = path.read_text()
if "OptionStartupPrecedence" not in text:
    text = require_replace(
        text,
        '''    options_manifest, options_service, OptionCommand, OptionContext, OptionDefinition, OptionKey,
    OptionResponse, OptionScope, OptionScopeKind, OptionSubjectId, OptionValue, OptionValueSource,
    OptionsInterface, ResolvedOption, OPTIONS_COMPONENT, OPTIONS_PLUGIN, OPTIONS_SERVICE,
''',
        '''    options_manifest, options_service, OptionAssignment, OptionCommand, OptionContext,
    OptionDefinition, OptionKey, OptionResponse, OptionScope, OptionScopeKind,
    OptionStartupPrecedence, OptionSubjectId, OptionValue, OptionValueLayer, OptionValueSource,
    OptionsInterface, ResolvedOption, OPTIONS_COMPONENT, OPTIONS_PLUGIN, OPTIONS_SERVICE,
''',
        "catalog option exports",
    )
path.write_text(text)

# Split built-in config loading from startup settings sources.
path = Path("rust/crates/phenix-harness/src/runtime_config.rs")
text = path.read_text()
text = text.replace(
    "    ModelTarget, OptionAssignment, OptionCommand, OptionKey, OptionResponse, OptionScope,\n    OptionSubjectId,\n",
    "    ModelTarget, OptionAssignment, OptionCommand, OptionKey, OptionResponse, OptionScope,\n    OptionStartupPrecedence, OptionSubjectId,\n",
    1,
)
start = text.index("pub(super) fn apply_config_directory(")
end = text.index("\npub(super) fn apply_runtime_config(", start)
replacement = '''pub(super) fn apply_default_config_directory(
    harness: &mut PhenixHarness,
    directory: &Path,
) -> Result<(), Box<dyn Error>> {
    if !directory.is_dir() {
        return Err(format!("default config directory does not exist: {}", directory.display()).into());
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
    let file_values = settings_assignments(file_settings)?;
    let nix_values = settings_assignments(nix_settings)?;

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

fn settings_assignments(
    settings: SettingsConfiguration,
) -> Result<Vec<OptionAssignment>, Box<dyn Error>> {
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
    Ok(values)
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
text = text[:start] + replacement + text[end:]
path.write_text(text)

# Runtime wiring: one default config root, one user config root, one Nix settings source.
path = Path("rust/crates/phenix-harness/src/main.rs")
text = path.read_text()
if "OptionStartupPrecedence" not in text.split("use serde_json", 1)[0]:
    text = text.replace(
        "use phenix_harness::{default_suite_authority, HarnessBuilder};\n",
        "use phenix_harness::{default_suite_authority, HarnessBuilder};\nuse phenix_plugin_catalog::OptionStartupPrecedence;\n",
        1,
    )
old = '''    if let Some(path) = env::var_os("PHENIX_CONFIG_DIR") {
        runtime_config::apply_config_directory(&mut harness, Path::new(&path))?;
    } else if let Some(path) = env::var_os("PHENIX_RUNTIME_CONFIG") {
        runtime_config::apply_runtime_config(&mut harness, Path::new(&path))?;
    }
'''
new = '''    if let Some(path) = env::var_os("PHENIX_DEFAULT_CONFIG_DIR") {
        runtime_config::apply_default_config_directory(&mut harness, Path::new(&path))?;
    }
    let config_directory = env::var_os("PHENIX_CONFIG_DIR").map(PathBuf::from);
    let nix_settings = env::var_os("PHENIX_NIX_SETTINGS").map(PathBuf::from);
    if config_directory.is_some() || nix_settings.is_some() {
        let precedence = match env::var("PHENIX_SETTINGS_PRECEDENCE").as_deref() {
            Ok("file") => OptionStartupPrecedence::File,
            Ok("nix") | Err(env::VarError::NotPresent) => OptionStartupPrecedence::Nix,
            Ok(value) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("invalid PHENIX_SETTINGS_PRECEDENCE: {value}"),
                )
                .into())
            }
            Err(error) => return Err(error.into()),
        };
        runtime_config::apply_startup_settings(
            &mut harness,
            config_directory.as_deref(),
            nix_settings.as_deref(),
            precedence,
        )?;
    }
'''
text = require_replace(text, old, new, "main config wiring")
path.write_text(text)

# Nix wrapper: user config directory, generated Nix settings, configurable startup precedence.
path = Path("modules/plugin-packaging.nix")
text = path.read_text()
text = require_replace(
    text,
    '''      layerPolicies ? [ ],
      settings ? { },
      ...
''',
    '''      layerPolicies ? [ ],
      settings ? { },
      configDirectory ? null,
      settingsPrecedence ? "nix",
      ...
''',
    "mkPhenix arguments",
)
text = require_replace(
    text,
    '''      settingsFile = pkgs.writeText "phenix-settings.json" (builtins.toJSON settings);
      selectedIds =
''',
    '''      nixSettingsFile = pkgs.writeText "phenix-nix-settings.json" (builtins.toJSON settings);
      validSettingsPrecedence = builtins.elem settingsPrecedence [ "nix" "file" ];
      selectedIds =
''',
    "nix settings file",
)
text = require_replace(
    text,
    '''    if
      plugins == [ ] && resources == [ ] && selectedIds == null && layerPolicies == [ ] && settings == { }
    then
''',
    '''    if !validSettingsPrecedence then
      throw "mkPhenix settingsPrecedence must be either 'nix' or 'file'"
    else if
      plugins == [ ]
      && resources == [ ]
      && selectedIds == null
      && layerPolicies == [ ]
      && settings == { }
      && configDirectory == null
      && settingsPrecedence == "nix"
    then
''',
    "wrapper fast path",
)

# Replace the current config-root setup block.
old = '''              ${pkgs.lib.optionalString (resources != [ ] || settings != { }) ''
                mkdir -p "$out/share/phenix"
                rm -f "$out/share/phenix/settings.json"
                cp ${settingsFile} "$out/share/phenix/settings.json"
              ''}

              for program in phenix phenix-harness; do
                if [ -e "$out/bin/$program" ]; then
                  ${pkgs.lib.optionalString (resources != [ ] || settings != { }) ''
                    wrapProgram "$out/bin/$program" \\
                      --set PHENIX_CONFIG_DIR "$out/share/phenix"
                  ''}
'''
new = '''              for program in phenix phenix-harness; do
                if [ -e "$out/bin/$program" ]; then
                  ${pkgs.lib.optionalString (resources != [ ]) ''
                    wrapProgram "$out/bin/$program" \\
                      --set PHENIX_DEFAULT_CONFIG_DIR "$out/share/phenix"
                  ''}
                  ${pkgs.lib.optionalString (configDirectory != null || resources != [ ]) ''
                    wrapProgram "$out/bin/$program" \\
                      --set PHENIX_CONFIG_DIR ${pkgs.lib.escapeShellArg (if configDirectory != null then toString configDirectory else "$out/share/phenix")}
                  ''}
                  ${pkgs.lib.optionalString (settings != { }) ''
                    wrapProgram "$out/bin/$program" \\
                      --set PHENIX_NIX_SETTINGS ${pkgs.lib.escapeShellArg (toString nixSettingsFile)}
                  ''}
                  ${pkgs.lib.optionalString (configDirectory != null || resources != [ ] || settings != { }) ''
                    wrapProgram "$out/bin/$program" \\
                      --set PHENIX_SETTINGS_PRECEDENCE ${pkgs.lib.escapeShellArg settingsPrecedence}
                  ''}
'''
text = require_replace(text, old, new, "wrapper config environment")

# Product fixtures for both startup precedence orders.
anchor = '''      settingsComposition = mkPhenix {
        inherit pkgs;
        plugins = defaultPlugins;
        resources = [ harnessResources ];
        settings = {
          global = {
            "session.auto_create" = false;
          };
          agents = {
            "agent.scout" = {
              "agent.max_parallel_tasks" = 4;
            };
          };
        };
      };
'''
replacement = '''      settingsConfigDirectory = pkgs.writeTextDir "settings.json" (
        builtins.toJSON {
          global = {
            "session.auto_create" = true;
          };
          agents = {
            "agent.scout" = {
              "agent.max_parallel_tasks" = 7;
            };
          };
        }
      );
      settingsComposition = mkPhenix {
        inherit pkgs;
        plugins = defaultPlugins;
        resources = [ harnessResources ];
        configDirectory = settingsConfigDirectory;
        settings = {
          global = {
            "session.auto_create" = false;
          };
          agents = {
            "agent.scout" = {
              "agent.max_parallel_tasks" = 4;
            };
          };
        };
      };
      filePrecedenceComposition = mkPhenix {
        inherit pkgs;
        plugins = defaultPlugins;
        resources = [ harnessResources ];
        configDirectory = settingsConfigDirectory;
        settingsPrecedence = "file";
        settings = {
          global = {
            "session.auto_create" = false;
          };
        };
      };
'''
text = require_replace(text, anchor, replacement, "settings fixtures")

old_tests = '''            jq -e '.global["session.auto_create"] == false and .agents["agent.scout"]["agent.max_parallel_tasks"] == 4' \\
              "${settingsComposition}/share/phenix/settings.json" >/dev/null
            export PHENIX_STATE_DB="$TMPDIR/settings.sqlite"
            printf '%s\\n' '{"id":1,"service":"phenix.options@1","input":{"operation":"resolve","key":"session.auto_create","context":{}}}' \\
              | "${settingsComposition}/bin/phenix" > "$TMPDIR/settings-option.json"
            jq -e '.status == "ok" and .output.result == "value" and .output.option.value.type == "bool" and .output.option.value.value == false and .output.option.source == "global"' \\
              "$TMPDIR/settings-option.json" >/dev/null
            printf '%s\\n' '{"id":2,"service":"phenix.sdk.config@1","input":{"operation":"read","path":"settings.json"}}' \\
              | "${settingsComposition}/bin/phenix" > "$TMPDIR/settings-config.json"
            jq -e '.status == "ok" and .output.result == "file" and ((.output.content | implode | fromjson).global["session.auto_create"] == false)' \\
              "$TMPDIR/settings-config.json" >/dev/null

'''
new_tests = '''            export PHENIX_STATE_DB="$TMPDIR/settings.sqlite"
            printf '%s\\n' '{"id":1,"service":"phenix.options@1","input":{"operation":"resolve","key":"session.auto_create","context":{}}}' \\
              | "${settingsComposition}/bin/phenix" > "$TMPDIR/settings-option.json"
            jq -e '.status == "ok" and .output.result == "value" and .output.option.value.type == "bool" and .output.option.value.value == false and .output.option.source == "global" and .output.option.layer == "nix"' \\
              "$TMPDIR/settings-option.json" >/dev/null
            printf '%s\\n' '{"id":2,"service":"phenix.sdk.config@1","input":{"operation":"read","path":"settings.json"}}' \\
              | "${settingsComposition}/bin/phenix" > "$TMPDIR/settings-config.json"
            jq -e '.status == "ok" and .output.result == "file" and ((.output.content | implode | fromjson).global["session.auto_create"] == true)' \\
              "$TMPDIR/settings-config.json" >/dev/null
            printf '%s\\n' '{"id":3,"service":"phenix.options@1","input":{"operation":"set","key":"session.auto_create","scope":"global","value":{"type":"bool","value":true}}}' \\
              | "${settingsComposition}/bin/phenix" > "$TMPDIR/settings-runtime-set.json"
            printf '%s\\n' '{"id":4,"service":"phenix.options@1","input":{"operation":"resolve","key":"session.auto_create","context":{"agent":"agent.scout"}}}' \\
              | "${settingsComposition}/bin/phenix" > "$TMPDIR/settings-runtime-resolve.json"
            jq -e '.status == "ok" and .output.option.value.value == true and .output.option.layer == "runtime" and .output.option.source == "global"' \\
              "$TMPDIR/settings-runtime-resolve.json" >/dev/null

            export PHENIX_STATE_DB="$TMPDIR/settings-file-first.sqlite"
            printf '%s\\n' '{"id":1,"service":"phenix.options@1","input":{"operation":"resolve","key":"session.auto_create","context":{}}}' \\
              | "${filePrecedenceComposition}/bin/phenix" > "$TMPDIR/settings-file-first.json"
            jq -e '.status == "ok" and .output.option.value.value == true and .output.option.layer == "file"' \\
              "$TMPDIR/settings-file-first.json" >/dev/null

'''
text = require_replace(text, old_tests, new_tests, "settings product tests")
path.write_text(text)

# Product smoke no longer expects generated settings.json in the packaged defaults.
path = Path("modules/phenix-acp.nix")
text = path.read_text()
text = text.replace('            test -f ${supportedPhenix}/share/phenix/settings.json\n', '', 1)
path.write_text(text)

# Specs: one source of truth for startup/runtime precedence.
path = Path("spec/plugin-options.md")
text = path.read_text()
marker = "## Settings file"
if marker in text:
    prefix = text[: text.index(marker)]
    suffix = '''## Settings file

The wrapper can provide two startup sources: `settings.json` from `PHENIX_CONFIG_DIR`, and typed Nix `settings` materialized separately by `mkPhenix`.

Resolution is source-first. Runtime `Set` values always win. The preferred startup source is next, then the other startup source, then built-in defaults. Within each source, scope precedence is `agent > session > global`.

`mkPhenix.settingsPrecedence` is `"nix"` by default. Set it to `"file"` to let `settings.json` override Nix settings. Startup sources are applied as replaceable snapshots, so removing an entry removes that source's prior value. Runtime values are durable and remain separate.
'''
    path.write_text(prefix + suffix)

path = Path("spec/plugin-sdk.md")
text = path.read_text()
old = "The Nix wrapper owns `PHENIX_CONFIG_DIR`. SDK code receives relative names such as `settings.json`; it does not discover host configuration directories."
new = "The Nix wrapper owns `PHENIX_CONFIG_DIR`. It points at the selected user configuration directory, or the packaged default directory when none is supplied. SDK code receives relative names such as `settings.json`; it does not discover host configuration directories. Nix-generated settings are a separate startup source and are not written into that directory."
if old in text:
    text = text.replace(old, new, 1)
path.write_text(text)
