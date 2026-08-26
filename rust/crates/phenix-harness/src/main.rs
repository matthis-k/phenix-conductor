use phenix_harness::{default_suite_authority, PhenixHarness};
use phenix_kernel::{LocalPersistence, ServiceId};
use serde_json::{json, Value};
use std::{
    env,
    error::Error,
    fs,
    io::{self, BufRead, Write},
    path::PathBuf,
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
    let mut harness = PhenixHarness::default_suite_with_persistence(persistence)?;
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

fn handle_request(harness: &mut PhenixHarness, line: &str) -> Value {
    let request = match serde_json::from_str::<Value>(line) {
        Ok(Value::Object(request)) => request,
        Ok(_) => {
            return json!({ "id": Value::Null, "status": "error", "error": "request must be a JSON object" })
        }
        Err(error) => {
            return json!({ "id": Value::Null, "status": "error", "error": error.to_string() })
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
