from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one replacement, found {count}")
    file.write_text(text.replace(old, new, 1))


replace_once(
    "rust/crates/phenix-harness/src/lib.rs",
    "use std::{collections::BTreeMap, sync::Arc};",
    "use std::{\n    collections::{BTreeMap, BTreeSet},\n    sync::Arc,\n};",
)

old_default_suite = '''    pub fn with_default_suite() -> Result<Self, KernelError> {
        let mut builder = Self::new();
        let authority = default_suite_authority();
        builder.add_embedded(repository_worker_manifest(), repository_worker_factory)?;
        builder.add_embedded(session_manifest(), session_factory)?;
        builder.add_embedded(artifact_manifest(), artifact_factory)?;
        builder.add_embedded(cli_manifest(authority.clone()), cli_factory)?;
        builder.add_embedded(context_manifest(), context_factory)?;
        builder.add_embedded(execution_manifest(authority.clone()), execution_factory)?;
        builder.add_embedded(language_manifest(), language_factory)?;
        builder.add_embedded(planning_manifest(), planning_factory)?;
        builder.add_embedded(workspace_manifest(), workspace_factory)?;
        builder.add_embedded(
            model_routing_manifest(authority.clone()),
            model_routing_factory,
        )?;
        builder.add_embedded(job_manifest(), job_factory)?;
        builder.add_embedded(frontend_manifest(authority.clone()), frontend_factory)?;
        builder.add_embedded(hook_manifest(authority.clone()), hook_factory)?;
        builder.add_embedded(debug_manifest(authority), debug_factory)?;
        Ok(builder)
    }
'''

new_default_suite = '''    pub fn with_default_suite() -> Result<Self, KernelError> {
        Self::with_default_suite_selection(&BTreeSet::new())
    }

    pub fn with_default_suite_selection(
        disabled: &BTreeSet<PluginId>,
    ) -> Result<Self, KernelError> {
        let mut builder = Self::new();
        let authority = default_suite_authority();

        macro_rules! add_if_enabled {
            ($manifest:expr, $factory:expr) => {{
                let manifest = $manifest;
                if !disabled.contains(&manifest.id) {
                    builder.add_embedded(manifest, $factory)?;
                }
            }};
        }

        add_if_enabled!(repository_worker_manifest(), repository_worker_factory);
        add_if_enabled!(session_manifest(), session_factory);
        add_if_enabled!(artifact_manifest(), artifact_factory);
        add_if_enabled!(cli_manifest(authority.clone()), cli_factory);
        add_if_enabled!(context_manifest(), context_factory);
        add_if_enabled!(execution_manifest(authority.clone()), execution_factory);
        add_if_enabled!(language_manifest(), language_factory);
        add_if_enabled!(planning_manifest(), planning_factory);
        add_if_enabled!(workspace_manifest(), workspace_factory);
        add_if_enabled!(
            model_routing_manifest(authority.clone()),
            model_routing_factory
        );
        add_if_enabled!(job_manifest(), job_factory);
        add_if_enabled!(frontend_manifest(authority.clone()), frontend_factory);
        add_if_enabled!(hook_manifest(authority.clone()), hook_factory);
        add_if_enabled!(debug_manifest(authority), debug_factory);
        Ok(builder)
    }
'''
replace_once("rust/crates/phenix-harness/src/lib.rs", old_default_suite, new_default_suite)

replace_once(
    "rust/crates/phenix-harness/src/main.rs",
    "use phenix_harness::{default_suite_authority, PhenixHarness};\nuse phenix_kernel::{LocalPersistence, ServiceId};",
    "use phenix_harness::{default_suite_authority, HarnessBuilder, PhenixHarness};\nuse phenix_kernel::{LocalPersistence, PluginId, ServiceId};",
)
replace_once(
    "rust/crates/phenix-harness/src/main.rs",
    "    env,\n    error::Error,",
    "    collections::BTreeSet,\n    env,\n    error::Error,",
)
replace_once(
    "rust/crates/phenix-harness/src/main.rs",
    "    let persistence = LocalPersistence::open(&state)?;\n    let mut harness = PhenixHarness::default_suite_with_persistence(persistence)?;",
    "    let persistence = LocalPersistence::open(&state)?;\n    let disabled = disabled_plugins()?;\n    let mut harness = HarnessBuilder::with_default_suite_selection(&disabled)?\n        .build_with_persistence(persistence)?;",
)

main_tail = '''fn state_path() -> Result<PathBuf, Box<dyn Error>> {
'''
main_insert = '''fn disabled_plugins() -> Result<BTreeSet<PluginId>, Box<dyn Error>> {
    let Some(config_path) = env::var_os("PHENIX_RUNTIME_CONFIG") else {
        return Ok(BTreeSet::new());
    };
    let config = serde_json::from_slice::<Value>(&fs::read(config_path)?)?;
    let disabled = config
        .get("disabledPlugins")
        .and_then(Value::as_array)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "runtime config missing disabledPlugins array"))?;
    disabled
        .iter()
        .map(|value| {
            let id = value.as_str().ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "disabled plugin id must be a string")
            })?;
            PluginId::parse(id).map_err(|message| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid disabled plugin id {id}: {message}"),
                )
                .into()
            })
        })
        .collect()
}

fn state_path() -> Result<PathBuf, Box<dyn Error>> {
'''
replace_once("rust/crates/phenix-harness/src/main.rs", main_tail, main_insert)

replace_once(
    "modules/plugin-packaging.nix",
    "      kernelOnly ? false,\n      plugins ? [ ],\n      ...",
    "      kernelOnly ? false,\n      plugins ? [ ],\n      disabledPlugins ? [ ],\n      ...",
)

old_mkphenix_body = '''    let
      base =
        if kernelOnly then
          self.packages.${pkgs.system}.phenix-kernel
        else
          self.packages.${pkgs.system}.phenix-harness;
    in
    if plugins == [ ] then
      base
    else
      pkgs.symlinkJoin {
        name = if kernelOnly then "phenix-kernel-composed" else "phenix-composed";
        paths = [ base ] ++ plugins;
      };
'''
new_mkphenix_body = '''    let
      base =
        if kernelOnly then
          self.packages.${pkgs.system}.phenix-kernel
        else
          self.packages.${pkgs.system}.phenix-harness;
      runtimeConfig = pkgs.writeText "phenix-runtime-config.json" (
        builtins.toJSON {
          inherit disabledPlugins;
          pluginPackages = map toString plugins;
        }
      );
    in
    if kernelOnly then
      if plugins == [ ] then
        base
      else
        pkgs.symlinkJoin {
          name = "phenix-kernel-composed";
          paths = [ base ] ++ plugins;
        }
    else
      pkgs.symlinkJoin {
        name = "phenix-composed";
        paths = [ base ] ++ plugins;
        nativeBuildInputs = [ pkgs.makeWrapper ];
        postBuild = ''
          wrapProgram "$out/bin/phenix" --set PHENIX_RUNTIME_CONFIG "${runtimeConfig}"
          wrapProgram "$out/bin/phenix-harness" --set PHENIX_RUNTIME_CONFIG "${runtimeConfig}"
        '';
      };
'''
replace_once("modules/plugin-packaging.nix", old_mkphenix_body, new_mkphenix_body)

replace_once(
    "modules/plugin-packaging.nix",
    '''      externalComposition = mkPhenix {
        inherit pkgs;
        plugins = [ externalPlugin ];
      };
''',
    '''      omittedSessionComposition = mkPhenix {
        inherit pkgs;
        disabledPlugins = [ "phenix.sessions" ];
      };
      externalComposition = mkPhenix {
        inherit pkgs;
        plugins = [ externalPlugin ];
      };
''',
)

replace_once(
    "modules/plugin-packaging.nix",
    '''            test -x "${defaultComposition}/bin/phenix"
            test -x "${defaultComposition}/bin/phenix-harness"
            test ! -e "${defaultComposition}/bin/phenix-conductor"
''',
    '''            test -x "${defaultComposition}/bin/phenix"
            test -x "${defaultComposition}/bin/phenix-harness"
            test ! -e "${defaultComposition}/bin/phenix-conductor"
            default_services="$(${defaultComposition}/bin/phenix --list-services)"
            echo "$default_services" | jq -e '.plugins | index("phenix.sessions") != null' >/dev/null
            echo "$default_services" | jq -e '.services | index("phenix.sessions@1") != null' >/dev/null
            omitted_services="$(${omittedSessionComposition}/bin/phenix --list-services)"
            echo "$omitted_services" | jq -e '.plugins | index("phenix.sessions") == null' >/dev/null
            echo "$omitted_services" | jq -e '.services | index("phenix.sessions@1") == null' >/dev/null
''',
)
