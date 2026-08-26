from pathlib import Path

lib = Path("rust/crates/phenix-harness/src/lib.rs")
text = lib.read_text()
text = text.replace(
    "PluginInstance, PluginManifest,\n};",
    "PluginInstance, PluginManifest, PersistenceBackend,\n};",
    1,
)
old = '''    pub fn build(self) -> Result<PhenixHarness, KernelError> {
        let config = KernelConfig::new(self.manifests)?;
        let mut kernel = Kernel::new(config);
        for (plugin, factory) in self.embedded_factories {
            kernel.register_embedded_factory(plugin, move || factory())?;
        }
        Ok(PhenixHarness { kernel })
    }
'''
new = '''    pub fn build(self) -> Result<PhenixHarness, KernelError> {
        let config = KernelConfig::new(self.manifests)?;
        let mut kernel = Kernel::new(config);
        for (plugin, factory) in self.embedded_factories {
            kernel.register_embedded_factory(plugin, move || factory())?;
        }
        Ok(PhenixHarness { kernel })
    }

    pub fn build_with_persistence(
        self,
        persistence: impl PersistenceBackend + 'static,
    ) -> Result<PhenixHarness, KernelError> {
        let config = KernelConfig::new(self.manifests)?;
        let mut kernel = Kernel::with_persistence(config, persistence);
        for (plugin, factory) in self.embedded_factories {
            kernel.register_embedded_factory(plugin, move || factory())?;
        }
        Ok(PhenixHarness { kernel })
    }
'''
if old in text:
    text = text.replace(old, new, 1)
elif "pub fn build_with_persistence(" not in text:
    raise SystemExit("HarnessBuilder::build anchor missing")

old = '''    pub fn default_suite() -> Result<Self, KernelError> {
        HarnessBuilder::with_default_suite()?.build()
    }
'''
new = '''    pub fn default_suite() -> Result<Self, KernelError> {
        HarnessBuilder::with_default_suite()?.build()
    }

    pub fn default_suite_with_persistence(
        persistence: impl PersistenceBackend + 'static,
    ) -> Result<Self, KernelError> {
        HarnessBuilder::with_default_suite()?.build_with_persistence(persistence)
    }
'''
if old in text:
    text = text.replace(old, new, 1)
elif "pub fn default_suite_with_persistence(" not in text:
    raise SystemExit("default_suite anchor missing")
lib.write_text(text)

main = Path("rust/crates/phenix-harness/src/main.rs")
main.write_text(r'''use phenix_harness::PhenixHarness;
use phenix_kernel::{Authority, CapabilityId, LocalPersistence, PluginId, ServiceId};
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
        println!("{}", serde_json::to_string(&json!({ "plugins": plugins, "services": services }))?);
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
        Ok(_) => return json!({ "id": Value::Null, "status": "error", "error": "request must be a JSON object" }),
        Err(error) => return json!({ "id": Value::Null, "status": "error", "error": error.to_string() }),
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
    let authority = match request_authority(request.get("authority")) {
        Ok(authority) => authority,
        Err(error) => return json!({ "id": id, "status": "error", "error": error }),
    };
    let binding = match request.get("binding") {
        None | Some(Value::Null) => None,
        Some(Value::String(value)) => match PluginId::parse(value.clone()) {
            Ok(binding) => Some(binding),
            Err(error) => return json!({ "id": id, "status": "error", "error": error }),
        },
        Some(_) => return json!({ "id": id, "status": "error", "error": "binding must be a string or null" }),
    };

    match harness.invoke(&service, &input, &authority, binding.as_ref()) {
        Ok(output) => match serde_json::from_slice::<Value>(&output) {
            Ok(output) => json!({ "id": id, "status": "ok", "output": output }),
            Err(_) => json!({ "id": id, "status": "ok", "output_bytes": output }),
        },
        Err(error) => json!({ "id": id, "status": "error", "error": error.to_string() }),
    }
}

fn request_authority(value: Option<&Value>) -> Result<Authority, String> {
    let Some(value) = value else {
        return Ok(Authority::default());
    };
    let values = value
        .as_array()
        .ok_or_else(|| "authority must be an array of capability strings".to_owned())?;
    let mut capabilities = Vec::with_capacity(values.len());
    for value in values {
        let capability = value
            .as_str()
            .ok_or_else(|| "authority entries must be strings".to_owned())?;
        capabilities.push(CapabilityId::parse(capability.to_owned()).map_err(str::to_owned)?);
    }
    Ok(Authority::new(capabilities))
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
''')

nix = Path("modules/phenix-acp.nix")
text = nix.read_text()
old = '''      # Until the first-party suite has moved behind kernel plugin contracts, the
      # supported Harness package remains the existing product binary. The package
      # name is stable now; the implementation behind it changes in this PR.
      phenixHarness = phenixConductor;
'''
new = '''      phenixHarness = pkgs.rustPlatform.buildRustPackage {
        pname = "phenix-harness";
        version = "0";
        src = rustSource;

        cargoLock.lockFile = ../rust/Cargo.lock;
        cargoBuildFlags = [
          "--package"
          "phenix-harness"
          "--bin"
          "phenix-harness"
        ];
        doCheck = false;

        installPhase = ''
          runHook preInstall
          mkdir -p "$out/bin"
          harness_binary="$(find target -path '*/release/phenix-harness' -type f -print -quit)"
          test -n "$harness_binary"
          cp "$harness_binary" "$out/bin/phenix-harness"
          ln -s phenix-harness "$out/bin/phenix"
          runHook postInstall
        '';
      };
'''
if old in text:
    text = text.replace(old, new, 1)
elif 'pname = "phenix-harness";' not in text:
    raise SystemExit("phenixHarness alias anchor missing")

start = text.index('      phenixProductSmoke =')
end = text.index('    in\n    {', start)
replacement = '''      phenixProductSmoke =
        pkgs.runCommand "phenix-product-smoke"
          {
            nativeBuildInputs = [
              phenixAcpSmoke
              phenixHarness
              pkgs.jq
            ];
          }
          ''
            phenix-acp-smoke

            export PHENIX_STATE_DB="$TMPDIR/harness.sqlite"
            phenix-harness --list-services > "$TMPDIR/services.json"
            jq -e '
              (.plugins | length == 14)
              and (.services | index("phenix.sessions@1") != null)
              and (.services | index("phenix.context@1") != null)
              and (.services | index("phenix.execution@1") != null)
              and (.services | index("phenix.repository-work-queue@1") != null)
            ' "$TMPDIR/services.json" >/dev/null

            printf '%s\\n' '{"id":1,"service":"phenix.sessions@1","input":{"operation":"create","id":"product-smoke","parent":null}}' \\
              | phenix-harness > "$TMPDIR/create.json"
            jq -e '
              .id == 1
              and .status == "ok"
              and .output.result == "created"
              and .output.session.id == "product-smoke"
            ' "$TMPDIR/create.json" >/dev/null

            printf '%s\\n' '{"id":2,"service":"phenix.sessions@1","input":{"operation":"get","id":"product-smoke"}}' \\
              | phenix-harness > "$TMPDIR/restore.json"
            jq -e '
              .id == 2
              and .status == "ok"
              and .output.result == "session"
              and .output.session.id == "product-smoke"
            ' "$TMPDIR/restore.json" >/dev/null

            touch "$out"
          '';
'''
text = text[:start] + replacement + text[end:]
text = text.replace('phenix-harness.program = "${phenixHarness}/bin/phenix-conductor";', 'phenix-harness.program = "${phenixHarness}/bin/phenix-harness";')
text = text.replace('phenix.program = "${phenixHarness}/bin/phenix-conductor";', 'phenix.program = "${phenixHarness}/bin/phenix";')
text = text.replace('default.program = "${phenixHarness}/bin/phenix-conductor";', 'default.program = "${phenixHarness}/bin/phenix";')
nix.write_text(text)
