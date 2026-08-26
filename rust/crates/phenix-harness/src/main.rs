use phenix_harness::{default_suite_authority, HarnessBuilder, PhenixHarness};
use phenix_kernel::{
    Authority, CapabilityId, ExternalPluginProcess, ExternalSandbox, ExternalTransportConfig,
    LocalPersistence, PluginExecution, PluginId, PluginManifest, ResourceNamespace,
    ServiceContribution, ServiceId,
};
use serde_json::{json, Map, Value};
use std::{
    env,
    error::Error,
    fs,
    io::{self, BufRead, Write},
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
    let mut builder = HarnessBuilder::with_default_suite()?;
    for package in configured_plugin_packages()? {
        add_packaged_plugin(&mut builder, &package)?;
    }
    let mut harness = builder.build_with_persistence(persistence)?;
    harness.activate()?;

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
    let mut stdout = io::BufWriter::new(io::stdout().lock());
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let response = handle_request(&mut harness, &line);
        serde_json::to_writer(&mut stdout, &response)?;
        stdout.write_all(b"\n")?;
        stdout.flush()?;
    }
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
            let service = ServiceId::parse(required_string(object, "service")?.to_owned())?;
            let priority = object
                .get("priority")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                .try_into()
                .map_err(|_| "plugin service priority does not fit i32".to_owned())?;
            Ok(ServiceContribution {
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

fn handle_request(harness: &mut PhenixHarness, line: &str) -> Value {
    let request = match serde_json::from_str::<Value>(line) {
        Ok(Value::Object(request)) => request,
        Ok(_) => {
            return json!({ "id": Value::Null, "status": "error", "error": "request must be a JSON object" });
        }
        Err(error) => {
            return json!({ "id": Value::Null, "status": "error", "error": error.to_string() });
        }
    };
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let Some(service) = request.get("service").and_then(Value::as_str) else {
        return json!({ "id": id, "status": "error", "error": "missing string field: service" });
    };
    let service = match ServiceId::parse(service) {
        Ok(service) => service,
        Err(error) => return json!({ "id": id, "status": "error", "error": error }),
    };
    let input = request.get("input").cloned().unwrap_or(Value::Null);
    let input = match serde_json::to_vec(&input) {
        Ok(input) => input,
        Err(error) => return json!({ "id": id, "status": "error", "error": error.to_string() }),
    };
    if request.contains_key("authority") || request.contains_key("binding") {
        return json!({
            "id": id,
            "status": "error",
            "error": "authority and provider binding are owned by Harness policy",
        });
    }

    match harness.invoke(&service, &input, &default_suite_authority(), None) {
        Ok(output) => match serde_json::from_slice::<Value>(&output) {
            Ok(output) => json!({ "id": id, "status": "ok", "output": output }),
            Err(_) => json!({ "id": id, "status": "ok", "output_bytes": output }),
        },
        Err(error) => json!({ "id": id, "status": "error", "error": error.to_string() }),
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
