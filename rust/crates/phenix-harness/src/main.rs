mod runtime_config;

use phenix_conductor::serve_jsonl;
use phenix_core::{
    Authority, CapabilityId, ExternalPluginProcess, ExternalSandbox, ExternalTransportConfig,
    LayerPolicy, LocalPersistence, PluginExecution, PluginId, PluginManifest, ResourceNamespace,
    ServiceContribution, ServiceId, ServiceRole,
};
use phenix_harness::{default_suite_authority, HarnessBuilder};
use phenix_plugin_catalog::OptionStartupPrecedence;
use serde_json::{json, Map, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    error::Error,
    fs, io,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::Arc,
    time::Duration,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("phenix-harness: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    if env::args().any(|argument| argument == "--help" || argument == "-h") {
        println!("phenix-harness [--list-services]\n\nWithout arguments, reads JSONL service requests from stdin and writes JSONL responses.");
        return Ok(());
    }

    let state = state_path()?;
    if let Some(parent) = state.parent() {
        fs::create_dir_all(parent)?;
    }
    let persistence = LocalPersistence::open(&state)?;
    let mut builder = match configured_first_party_plugins()? {
        Some(enabled) => HarnessBuilder::with_selected_suite(&enabled)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?,
        None => HarnessBuilder::with_default_suite()?,
    };
    for package in configured_plugin_packages()? {
        add_packaged_plugin(&mut builder, &package)?;
    }
    apply_configured_layer_policy(&mut builder)?;
    let mut harness = builder.build_with_persistence(persistence)?;
    harness.activate()?;
    if let Some(path) = env::var_os("PHENIX_DEFAULT_CONFIG_DIR") {
        runtime_config::apply_default_config_directory(&mut harness, Path::new(&path))?;
    }
    let config_directory = env::var_os("PHENIX_CONFIG_DIR").map(PathBuf::from);
    let nix_settings = env::var_os("PHENIX_NIX_SETTINGS").map(PathBuf::from);
    if config_directory.is_some() || nix_settings.is_some() {
        let precedence = match env::var("PHENIX_SETTINGS_PRECEDENCE") {
            Ok(value) if value == "file" => OptionStartupPrecedence::File,
            Ok(value) if value == "nix" => OptionStartupPrecedence::Nix,
            Err(env::VarError::NotPresent) => OptionStartupPrecedence::Nix,
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

    if env::args().any(|argument| argument == "--list-services") {
        let plugins = harness
            .kernel()
            .config()
            .manifests()
            .map(|manifest| manifest.id.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut services = harness
            .kernel()
            .config()
            .manifests()
            .flat_map(|manifest| manifest.services.iter())
            .map(|contribution| contribution.service.as_str().to_owned())
            .collect::<Vec<_>>();
        services.sort();
        services.dedup();
        println!(
            "{}",
            serde_json::to_string(&json!({ "plugins": plugins, "services": services }))?
        );
        return Ok(());
    }

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout = io::BufWriter::new(stdout.lock());
    serve_jsonl(
        harness.kernel_mut(),
        &default_suite_authority(),
        stdin.lock(),
        &mut stdout,
    )?;
    Ok(())
}

#[derive(Clone)]
struct ProcessSandbox;

impl ExternalSandbox for ProcessSandbox {
    fn spawn(&self, executable: &str) -> io::Result<Child> {
        Command::new(executable)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
    }
}

fn configured_first_party_plugins() -> Result<Option<BTreeSet<String>>, Box<dyn Error>> {
    let Some(value) = env::var_os("PHENIX_ENABLED_PLUGINS") else {
        return Ok(None);
    };
    let value = value
        .into_string()
        .map_err(|_| "PHENIX_ENABLED_PLUGINS must be valid UTF-8")?;
    let enabled = value
        .split(',')
        .filter(|entry| !entry.is_empty())
        .map(str::to_owned)
        .collect();
    Ok(Some(enabled))
}

#[derive(serde::Deserialize)]
struct ConfiguredLayerPolicy {
    service: String,
    plugin: String,
    priority: i32,
    #[serde(default)]
    required: bool,
    #[serde(default = "default_layer_enabled")]
    enabled: bool,
}

fn default_layer_enabled() -> bool {
    true
}

fn apply_configured_layer_policy(builder: &mut HarnessBuilder) -> Result<(), Box<dyn Error>> {
    let Some(value) = env::var_os("PHENIX_LAYER_POLICY") else {
        return Ok(());
    };
    let value = value
        .into_string()
        .map_err(|_| "PHENIX_LAYER_POLICY must be valid UTF-8")?;
    let configured: Vec<ConfiguredLayerPolicy> = serde_json::from_str(&value)?;
    let mut policies = BTreeMap::<ServiceId, Vec<LayerPolicy>>::new();
    for layer in configured {
        let service = ServiceId::parse(layer.service)?;
        policies.entry(service).or_default().push(LayerPolicy {
            plugin: PluginId::parse(layer.plugin)?,
            priority: layer.priority,
            required: layer.required,
            enabled: layer.enabled,
        });
    }
    for (service, layers) in policies {
        builder.set_layer_policy(service, layers);
    }
    Ok(())
}

fn configured_plugin_packages() -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let Some(value) = env::var_os("PHENIX_PLUGIN_PACKAGES") else {
        return Ok(Vec::new());
    };
    let value = value
        .into_string()
        .map_err(|_| "PHENIX_PLUGIN_PACKAGES must be valid UTF-8")?;
    if value.is_empty() {
        return Ok(Vec::new());
    }
    Ok(value.split(':').map(PathBuf::from).collect())
}

fn add_packaged_plugin(builder: &mut HarnessBuilder, package: &Path) -> Result<(), Box<dyn Error>> {
    let manifest_path = package.join("share/phenix-plugin/manifest.json");
    let value: Value = serde_json::from_slice(&fs::read(&manifest_path)?)?;
    let object = value
        .as_object()
        .ok_or("plugin manifest must be a JSON object")?;
    let id = PluginId::parse(required_string(object, "id")?.to_owned())?;
    let version = object
        .get("version")
        .and_then(Value::as_u64)
        .unwrap_or(1)
        .try_into()
        .map_err(|_| "plugin manifest version does not fit u32")?;
    let execution_name = required_string(object, "execution")?;
    let execution = match execution_name {
        "resource-only" => PluginExecution::ResourceOnly,
        "external" => PluginExecution::External {
            executable: packaged_executable(package)?.display().to_string(),
        },
        "embedded" => {
            return Err("packaged embedded plugins must be linked through Harness policy".into());
        }
        other => return Err(format!("unsupported packaged plugin execution: {other}").into()),
    };
    let manifest = PluginManifest {
        id,
        version,
        execution,
        dependencies: parse_strings(object.get("dependencies"))?
            .into_iter()
            .map(PluginId::parse)
            .collect::<Result<_, _>>()?,
        services: parse_services(object.get("services"))?,
        resource_namespaces: parse_strings(object.get("resource_namespaces"))?
            .into_iter()
            .map(ResourceNamespace::parse)
            .collect::<Result<_, _>>()?,
        maximum_authority: parse_authority(object.get("maximum_authority"))?,
    };
    if matches!(manifest.execution, PluginExecution::External { .. }) {
        let transport =
            ExternalTransportConfig::new(Arc::new(ProcessSandbox), Duration::from_secs(5));
        builder.add_external(manifest, move |manifest| {
            let PluginExecution::External { executable } = &manifest.execution else {
                return Err("external factory received non-external manifest".into());
            };
            Ok(Box::new(ExternalPluginProcess::new(
                manifest.clone(),
                executable.clone(),
                transport.clone(),
            )))
        })?;
    } else {
        builder.add_manifest(manifest);
    }
    Ok(())
}

fn required_string<'a>(object: &'a Map<String, Value>, key: &str) -> Result<&'a str, String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("plugin manifest field {key} must be a string"))
}

fn parse_strings(value: Option<&Value>) -> Result<Vec<String>, String> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    value
        .as_array()
        .ok_or_else(|| "plugin manifest list field must be an array".to_owned())?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| "plugin manifest list entries must be strings".to_owned())
        })
        .collect()
}

fn parse_authority(value: Option<&Value>) -> Result<Authority, String> {
    Ok(Authority::new(
        parse_strings(value)?
            .into_iter()
            .map(CapabilityId::parse)
            .collect::<Result<Vec<_>, _>>()?,
    ))
}

fn parse_services(value: Option<&Value>) -> Result<Vec<ServiceContribution>, String> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    value
        .as_array()
        .ok_or_else(|| "plugin manifest services must be an array".to_owned())?
        .iter()
        .map(|value| {
            let object = value
                .as_object()
                .ok_or_else(|| "plugin service contribution must be an object".to_owned())?;
            let role = match required_string(object, "role")? {
                "terminal" => ServiceRole::Terminal,
                "layer" => ServiceRole::Layer,
                other => return Err(format!("unsupported plugin service role: {other}")),
            };
            let service = ServiceId::parse(required_string(object, "service")?.to_owned())?;
            let priority = object
                .get("priority")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                .try_into()
                .map_err(|_| "plugin service priority does not fit i32".to_owned())?;
            Ok(ServiceContribution {
                role,
                service,
                priority,
                required_authority: parse_authority(object.get("required_authority"))?,
            })
        })
        .collect()
}

fn packaged_executable(package: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let directory = package.join("bin");
    let mut entries = fs::read_dir(&directory)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() || path.is_symlink())
        .collect::<Vec<_>>();
    entries.sort();
    match entries.as_slice() {
        [executable] => Ok(executable.clone()),
        [] => Err(format!(
            "external plugin package has no executable: {}",
            directory.display()
        )
        .into()),
        _ => Err(format!(
            "external plugin package must contain exactly one executable: {}",
            directory.display()
        )
        .into()),
    }
}

fn state_path() -> Result<PathBuf, Box<dyn Error>> {
    if let Some(path) = env::var_os("PHENIX_STATE_DB") {
        return Ok(PathBuf::from(path));
    }
    if let Some(state_home) = env::var_os("XDG_STATE_HOME") {
        return Ok(PathBuf::from(state_home).join("phenix/harness.sqlite"));
    }
    if let Some(home) = env::var_os("HOME") {
        return Ok(PathBuf::from(home).join(".local/state/phenix/harness.sqlite"));
    }
    Err("cannot determine durable state path; set PHENIX_STATE_DB or XDG_STATE_HOME".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packaged_service_roles_require_explicit_terminal_or_layer() {
        let services = json!([
            { "role": "terminal", "service": "demo.terminal@1" },
            { "role": "layer", "service": "demo.layer@1" }
        ]);
        let parsed = parse_services(Some(&services)).unwrap();
        assert_eq!(parsed[0].role, ServiceRole::Terminal);
        assert_eq!(parsed[1].role, ServiceRole::Layer);

        let missing = json!([{ "service": "demo.missing@1" }]);
        let error = parse_services(Some(&missing)).unwrap_err();
        assert_eq!(error, "plugin manifest field role must be a string");
    }

    #[test]
    fn packaged_service_roles_reject_unknown_values() {
        let services = json!([{ "role": "fallback", "service": "demo@1" }]);
        let error = parse_services(Some(&services)).unwrap_err();
        assert!(error.contains("unsupported plugin service role"));
    }

    #[test]
    fn configured_layer_policy_groups_layers_by_service() {
        let configured = serde_json::to_string(&vec![json!({
            "service": "demo@1",
            "plugin": "layer",
            "priority": 7,
            "required": true
        })])
        .unwrap();
        let parsed: Vec<ConfiguredLayerPolicy> = serde_json::from_str(&configured).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].service, "demo@1");
        assert_eq!(parsed[0].plugin, "layer");
        assert_eq!(parsed[0].priority, 7);
        assert!(parsed[0].required);
        assert!(parsed[0].enabled);
    }
}
