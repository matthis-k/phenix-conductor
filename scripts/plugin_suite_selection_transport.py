from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    if text.count(old) != 1:
        raise SystemExit(f"{path}: expected one replacement target, found {text.count(old)}")
    target.write_text(text.replace(old, new, 1))


replace_once(
    "rust/crates/phenix-harness/src/lib.rs",
    "use std::{collections::BTreeMap, sync::Arc};",
    "use std::{collections::{BTreeMap, BTreeSet}, sync::Arc};",
)

marker = """    pub fn add_manifest(&mut self, manifest: PluginManifest) {
        self.manifests.push(manifest);
    }
"""
selection = """    pub fn default_suite_plugin_ids() -> BTreeSet<String> {
        let authority = default_suite_authority();
        [
            repository_worker_manifest(),
            session_manifest(),
            artifact_manifest(),
            cli_manifest(authority.clone()),
            context_manifest(),
            execution_manifest(authority.clone()),
            language_manifest(),
            planning_manifest(),
            workspace_manifest(),
            model_routing_manifest(authority.clone()),
            job_manifest(),
            frontend_manifest(authority.clone()),
            hook_manifest(authority.clone()),
            debug_manifest(authority),
        ]
        .into_iter()
        .map(|manifest| manifest.id.as_str().to_owned())
        .collect()
    }

    pub fn with_selected_suite(enabled: &BTreeSet<String>) -> Result<Self, String> {
        let available = Self::default_suite_plugin_ids();
        let unknown = enabled.difference(&available).cloned().collect::<Vec<_>>();
        if !unknown.is_empty() {
            return Err(format!(
                "unknown first-party plugin id(s): {}",
                unknown.join(", ")
            ));
        }

        let mut builder = Self::new();
        let authority = default_suite_authority();
        builder.add_selected(enabled, repository_worker_manifest(), repository_worker_factory)?;
        builder.add_selected(enabled, session_manifest(), session_factory)?;
        builder.add_selected(enabled, artifact_manifest(), artifact_factory)?;
        builder.add_selected(enabled, cli_manifest(authority.clone()), cli_factory)?;
        builder.add_selected(enabled, context_manifest(), context_factory)?;
        builder.add_selected(
            enabled,
            execution_manifest(authority.clone()),
            execution_factory,
        )?;
        builder.add_selected(enabled, language_manifest(), language_factory)?;
        builder.add_selected(enabled, planning_manifest(), planning_factory)?;
        builder.add_selected(enabled, workspace_manifest(), workspace_factory)?;
        builder.add_selected(
            enabled,
            model_routing_manifest(authority.clone()),
            model_routing_factory,
        )?;
        builder.add_selected(enabled, job_manifest(), job_factory)?;
        builder.add_selected(
            enabled,
            frontend_manifest(authority.clone()),
            frontend_factory,
        )?;
        builder.add_selected(enabled, hook_manifest(authority.clone()), hook_factory)?;
        builder.add_selected(enabled, debug_manifest(authority), debug_factory)?;
        Ok(builder)
    }

    fn add_selected<F>(
        &mut self,
        enabled: &BTreeSet<String>,
        manifest: PluginManifest,
        factory: F,
    ) -> Result<(), String>
    where
        F: Fn() -> Box<dyn PluginInstance> + Send + Sync + 'static,
    {
        if enabled.contains(manifest.id.as_str()) {
            self.add_embedded(manifest, factory)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    pub fn add_manifest(&mut self, manifest: PluginManifest) {
        self.manifests.push(manifest);
    }
"""
replace_once("rust/crates/phenix-harness/src/lib.rs", marker, selection)

main_import = """use std::{
    env,
    error::Error,
    fs,
    io::{self, BufRead, Write},
"""
main_import_new = """use std::{
    collections::BTreeSet,
    env,
    error::Error,
    fs,
    io::{self, BufRead, Write},
"""
replace_once("rust/crates/phenix-harness/src/main.rs", main_import, main_import_new)

replace_once(
    "rust/crates/phenix-harness/src/main.rs",
    "    let mut builder = HarnessBuilder::with_default_suite()?;\n",
    """    let mut builder = match configured_first_party_plugins()? {
        Some(enabled) => HarnessBuilder::with_selected_suite(&enabled)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?,
        None => HarnessBuilder::with_default_suite()?,
    };
""",
)

package_marker = """fn configured_plugin_packages() -> Result<Vec<PathBuf>, Box<dyn Error>> {
"""
package_insert = """fn configured_first_party_plugins() -> Result<Option<BTreeSet<String>>, Box<dyn Error>> {
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

fn configured_plugin_packages() -> Result<Vec<PathBuf>, Box<dyn Error>> {
"""
replace_once("rust/crates/phenix-harness/src/main.rs", package_marker, package_insert)

nix_signature = """      kernelOnly ? false,
      plugins ? [ ],
      ...
    }:
"""
nix_signature_new = """      kernelOnly ? false,
      plugins ? [ ],
      enabledPlugins ? null,
      ...
    }:
"""
replace_once("modules/plugin-packaging.nix", nix_signature, nix_signature_new)

nix_body = """    if plugins == [ ] then
      base
    else
      pkgs.symlinkJoin {
"""
nix_body_new = """    if plugins == [ ] && enabledPlugins == null then
      base
    else
      pkgs.symlinkJoin {
"""
replace_once("modules/plugin-packaging.nix", nix_body, nix_body_new)

post_build_old = """            let
              pluginPackages = pkgs.lib.concatStringsSep ":" (map toString plugins);
            in
            ''
              for program in phenix phenix-harness; do
                if [ -e "$out/bin/$program" ]; then
                  wrapProgram "$out/bin/$program" \\
                    --set PHENIX_PLUGIN_PACKAGES ${pkgs.lib.escapeShellArg pluginPackages}
                fi
              done
            '';
"""
post_build_new = """            let
              pluginPackages = pkgs.lib.concatStringsSep ":" (map toString plugins);
              enabledPluginIds =
                if enabledPlugins == null then null else pkgs.lib.concatStringsSep "," enabledPlugins;
            in
            ''
              for program in phenix phenix-harness; do
                if [ -e "$out/bin/$program" ]; then
                  ${pkgs.lib.optionalString (plugins != [ ]) ''
                    wrapProgram "$out/bin/$program" \\
                      --set PHENIX_PLUGIN_PACKAGES ${pkgs.lib.escapeShellArg pluginPackages}
                  ''}
                  ${pkgs.lib.optionalString (enabledPlugins != null) ''
                    wrapProgram "$out/bin/$program" \\
                      --set PHENIX_ENABLED_PLUGINS ${pkgs.lib.escapeShellArg enabledPluginIds}
                  ''}
                fi
              done
            '';
"""
replace_once("modules/plugin-packaging.nix", post_build_old, post_build_new)

composition_marker = """      kernelComposition = mkPhenix {
        inherit pkgs;
        kernelOnly = true;
        plugins = [ resourcePlugin ];
      };
"""
composition_new = """      kernelComposition = mkPhenix {
        inherit pkgs;
        kernelOnly = true;
        plugins = [ resourcePlugin ];
      };
      sessionOnlyComposition = mkPhenix {
        inherit pkgs;
        enabledPlugins = [ "phenix.sessions" ];
      };
      contextOnlyComposition = mkPhenix {
        inherit pkgs;
        enabledPlugins = [ "phenix.context" ];
      };
      invalidEmbeddedComposition = mkPhenix {
        inherit pkgs;
        enabledPlugins = [ "fixture.missing" ];
      };
"""
replace_once("modules/plugin-packaging.nix", composition_marker, composition_new)

check_marker = """            test -x "${resourceComposition}/bin/phenix"
            test -x "${resourceComposition}/bin/phenix-harness"
"""
check_new = """            export PHENIX_STATE_DB="$TMPDIR/session-only.sqlite"
            "${sessionOnlyComposition}/bin/phenix" --list-services > "$TMPDIR/session-only.json"
            jq -e '
              (.plugins | length == 1)
              and (.plugins[0] == "phenix.sessions")
              and (.services | index("phenix.sessions@1") != null)
              and (.services | index("phenix.context@1") == null)
            ' "$TMPDIR/session-only.json" >/dev/null
            export PHENIX_STATE_DB="$TMPDIR/context-only.sqlite"
            "${contextOnlyComposition}/bin/phenix" --list-services > "$TMPDIR/context-only.json"
            jq -e '
              (.plugins | length == 1)
              and (.plugins[0] == "phenix.context")
              and (.services | index("phenix.context@1") != null)
              and (.services | index("phenix.sessions@1") == null)
            ' "$TMPDIR/context-only.json" >/dev/null
            export PHENIX_STATE_DB="$TMPDIR/invalid-embedded.sqlite"
            if "${invalidEmbeddedComposition}/bin/phenix" --list-services > "$TMPDIR/invalid-embedded.json" 2>&1; then
              echo "unknown embedded plugin selection unexpectedly succeeded" >&2
              exit 1
            fi
            test -x "${resourceComposition}/bin/phenix"
            test -x "${resourceComposition}/bin/phenix-harness"
"""
replace_once("modules/plugin-packaging.nix", check_marker, check_new)
