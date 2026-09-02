{ self, ... }:
let
  mkPhenixPlugin =
    {
      pkgs,
      name,
      manifest,
      package ? null,
      resources ? null,
    }:
    let
      execution = manifest.execution or null;
      executionKind = if builtins.isAttrs execution then execution.kind or null else null;
      isEmbedded = executionKind == "embedded";
      isRuntime = executionKind == "runtime";
      isResourceOnly = executionKind == "resource_only";
      metadataDirectory = if isEmbedded then "share/phenix-plugins/${name}" else "share/phenix-plugin";
    in
    assert isEmbedded || isRuntime || isResourceOnly;
    assert (!isEmbedded) || package != null;
    assert (!isRuntime) || package == null;
    assert (!isResourceOnly) || package == null;
    pkgs.runCommand "phenix-plugin-${name}"
      {
        nativeBuildInputs = [ pkgs.jq ];
        passAsFile = [ "manifestJson" ];
        manifestJson = builtins.toJSON manifest;
        passthru = {
          phenixPluginId = manifest.id;
          phenixPluginExecution = executionKind;
        };
      }
      ''
        set -euo pipefail
        mkdir -p "$out/${metadataDirectory}"
        jq -e 'type == "object" and (.id | type == "string" and length > 0)' \
          "$manifestJsonPath" >/dev/null
        cp "$manifestJsonPath" "$out/${metadataDirectory}/manifest.json"

        ${pkgs.lib.optionalString isEmbedded ''
          ln -s "${package}" "$out/${metadataDirectory}/embedded-package"
        ''}
        ${pkgs.lib.optionalString (resources != null) ''
          test -e "${resources}"
          ln -s "${resources}" "$out/share/phenix-plugin/resources"
        ''}
      '';

  mkPhenixClient =
    {
      pkgs,
      name,
      package,
    }:
    pkgs.runCommand "phenix-client-${name}" { } ''
      mkdir -p "$out/share/phenix-client"
      printf '%s\n' ${pkgs.lib.escapeShellArg name} > "$out/share/phenix-client/name"
      ln -s ${package} "$out/share/phenix-client/rust-package"
    '';

  mkPhenix =
    {
      pkgs,
      conductorOnly ? false,
      plugins ? [ ],
      resources ? [ ],
      enabledPlugins ? null,
      layerPolicies ? [ ],
      settings ? { },
      configDirectory ? null,
      settingsPrecedence ? "nix",
      ...
    }:
    let
      base =
        if conductorOnly then
          self.packages.${pkgs.system}.phenix-conductor
        else
          self.packages.${pkgs.system}.phenix-harness-runtime;
      isEmbedded = plugin: (plugin.phenixPluginExecution or null) == "embedded";
      embeddedPlugins = builtins.filter isEmbedded plugins;
      packagedPlugins = builtins.filter (plugin: !isEmbedded plugin) plugins;
      selectedEmbeddedIds = map (plugin: plugin.phenixPluginId) embeddedPlugins;
      nixSettingsFile = pkgs.writeText "phenix-nix-settings.json" (builtins.toJSON settings);
      validSettingsPrecedence = builtins.elem settingsPrecedence [
        "nix"
        "file"
      ];
      selectedIds =
        if enabledPlugins != null then
          enabledPlugins
        else if embeddedPlugins != [ ] then
          selectedEmbeddedIds
        else
          null;
    in
    if !validSettingsPrecedence then
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
      base
    else
      pkgs.symlinkJoin {
        name = if conductorOnly then "phenix-conductor-composed" else "phenix-composed";
        paths = [ base ] ++ plugins ++ resources;
        nativeBuildInputs = [ pkgs.makeWrapper ];
        postBuild =
          if conductorOnly then
            ""
          else
            let
              pluginPackages = pkgs.lib.concatStringsSep ":" (map toString packagedPlugins);
              enabledPluginIds = if selectedIds == null then null else pkgs.lib.concatStringsSep "," selectedIds;
              layerPolicyJson = builtins.toJSON layerPolicies;
            in
            ''
              for program in phenix phenix-harness; do
                if [ -e "$out/bin/$program" ]; then
                  ${pkgs.lib.optionalString (resources != [ ]) ''
                    wrapProgram "$out/bin/$program" \
                      --set PHENIX_DEFAULT_CONFIG_DIR "$out/share/phenix"
                  ''}
                  ${pkgs.lib.optionalString (configDirectory != null) ''
                    wrapProgram "$out/bin/$program" \
                      --set PHENIX_CONFIG_DIR ${pkgs.lib.escapeShellArg (toString configDirectory)}
                  ''}
                  ${pkgs.lib.optionalString (configDirectory == null && resources != [ ]) ''
                    wrapProgram "$out/bin/$program" \
                      --set PHENIX_CONFIG_DIR "$out/share/phenix"
                  ''}
                  ${pkgs.lib.optionalString (settings != { }) ''
                    wrapProgram "$out/bin/$program" \
                      --set PHENIX_NIX_SETTINGS ${pkgs.lib.escapeShellArg (toString nixSettingsFile)}
                  ''}
                  ${pkgs.lib.optionalString (configDirectory != null || resources != [ ] || settings != { }) ''
                    wrapProgram "$out/bin/$program" \
                      --set PHENIX_SETTINGS_PRECEDENCE ${pkgs.lib.escapeShellArg settingsPrecedence}
                  ''}
                  ${pkgs.lib.optionalString (packagedPlugins != [ ]) ''
                    wrapProgram "$out/bin/$program" \
                      --set PHENIX_PLUGIN_PACKAGES ${pkgs.lib.escapeShellArg pluginPackages}
                  ''}
                  ${pkgs.lib.optionalString (selectedIds != null) ''
                    wrapProgram "$out/bin/$program" \
                      --set PHENIX_ENABLED_PLUGINS ${pkgs.lib.escapeShellArg enabledPluginIds}
                  ''}
                  ${pkgs.lib.optionalString (layerPolicies != [ ]) ''
                    wrapProgram "$out/bin/$program" \
                      --set PHENIX_LAYER_POLICY ${pkgs.lib.escapeShellArg layerPolicyJson}
                  ''}
                  ${pkgs.lib.optionalString (resources != [ ]) ''
                    wrapProgram "$out/bin/$program" \
                      --set PHENIX_SKILL_PATH "$out/share/phenix/skills"
                  ''}
                fi
              done
            '';
      };
in
{
  flake = {
    lib = {
      inherit mkPhenix mkPhenixClient mkPhenixPlugin;
    };
    wrappers.phenix.wrap = mkPhenix;
  };

  perSystem =
    { pkgs, ... }:
    let
      fixtureResources = pkgs.writeTextDir "README.txt" "resource plugin fixture";
      resourcePlugin = mkPhenixPlugin {
        inherit pkgs;
        name = "resource-fixture";
        manifest = {
          id = "fixture.resources";
          version = 1;
          execution.kind = "resource_only";
          dependencies = [ ];
          services = [ ];
          resource_namespaces = [ ];
          maximum_authority = [ ];
        };
        resources = fixtureResources;
      };
      defaultPluginNames = [
        "artifacts"
        "api"
        "command-toolbelt"
        "context"
        "debug"
        "execution"
        "frontend"
        "hooks"
        "jobs"
        "language"
        "models"
        "options"
        "planning"
        "repository-workers"
        "session-tree"
        "sessions"
        "workspace"
      ];
      defaultPlugins = map (name: self.phenixPlugins.${pkgs.system}.${name}) defaultPluginNames;
      harnessResources = self.packages.${pkgs.system}.phenix-harness-resources;
      defaultComposition = mkPhenix {
        inherit pkgs;
        plugins = defaultPlugins;
        resources = [ harnessResources ];
      };
      settingsConfigDirectory = pkgs.writeTextDir "settings.json" (
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
      resourceComposition = mkPhenix {
        inherit pkgs;
        plugins = defaultPlugins ++ [ resourcePlugin ];
        resources = [ harnessResources ];
      };
      conductorComposition = mkPhenix {
        inherit pkgs;
        conductorOnly = true;
      };
      sessionOnlyComposition = mkPhenix {
        inherit pkgs;
        plugins = [ self.phenixPlugins.${pkgs.system}.sessions ];
      };
      contextOnlyComposition = mkPhenix {
        inherit pkgs;
        plugins = [ self.phenixPlugins.${pkgs.system}.context ];
      };
    in
    {
      packages = {
        phenix-harness = defaultComposition;
        phenix = defaultComposition;
        default = defaultComposition;
      };
      apps = {
        phenix-harness.program = "${defaultComposition}/bin/phenix-harness";
        phenix.program = "${defaultComposition}/bin/phenix";
        default.program = "${defaultComposition}/bin/phenix";
        phenix-conductor.program = "${self.packages.${pkgs.system}.phenix-conductor}/bin/phenix-conductor";
      };
      checks.phenix-plugin-packaging =
        pkgs.runCommand "phenix-plugin-packaging-check" { nativeBuildInputs = [ pkgs.jq ]; }
          ''
            set -euxo pipefail
            test -x "${defaultComposition}/bin/phenix"
            test -x "${defaultComposition}/bin/phenix-harness"
            test -f "${defaultComposition}/share/phenix/runtime.json"
            test -f "${defaultComposition}/share/phenix/skills/write/SKILL.md"
            test -f "${defaultComposition}/share/phenix/skills/pstack-LICENSE"
            export PHENIX_STATE_DB="$TMPDIR/composition.sqlite"
            "${defaultComposition}/bin/phenix" --list-services > "$TMPDIR/default-services.json"
            jq -e '(.plugins | length == 17) and ([.plugins[] | select(startswith("phenix.basic-"))] | length == 0) and (.services | index("phenix.sessions@1") != null)' "$TMPDIR/default-services.json" >/dev/null

            assert_option() {
              local file="$1"
              local expected_value="$2"
              local expected_layer="$3"
              jq -e --argjson expected_value "$expected_value" --arg expected_layer "$expected_layer" '
                .status == "ok"
                and .output.type == "variant"
                and .output.value.tag == "Value"
                and (.output.value.value.value.option.value |
                  .value.value.tag == "Bool"
                  and .value.value.value.type == "bool"
                  and .value.value.value.value == $expected_value
                  and .source.value.tag == "Global"
                  and .layer.value.tag == $expected_layer
                )
              ' "$file" >/dev/null
            }

            export PHENIX_STATE_DB="$TMPDIR/settings.sqlite"
            printf '%s\n' '{"id":1,"service":"phenix.options@1","input":{"type":"variant","value":{"tag":"Resolve","value":{"type":"table","value":{"key":{"type":"string","value":"session.auto_create"},"context":{"type":"table","value":{"session":{"type":"option","value":null},"agent":{"type":"option","value":null}}}}}}}}' \
              | "${settingsComposition}/bin/phenix" > "$TMPDIR/settings-option.json"
            assert_option "$TMPDIR/settings-option.json" false Nix
            printf '%s\n' '{"id":2,"service":"phenix.api.config@1","input":{"type":"variant","value":{"tag":"Read","value":{"type":"table","value":{"path":{"type":"string","value":"settings.json"}}}}}}' \
              | "${settingsComposition}/bin/phenix" > "$TMPDIR/settings-config.json"
            jq -e '.status == "ok" and .output.type == "variant" and .output.value.tag == "File" and ((.output.value.value.value.content.value | implode | fromjson).global["session.auto_create"] == true)' \
              "$TMPDIR/settings-config.json" >/dev/null
            printf '%s\n' '{"id":3,"service":"phenix.options@1","input":{"type":"variant","value":{"tag":"Set","value":{"type":"table","value":{"key":{"type":"string","value":"session.auto_create"},"scope":{"type":"variant","value":{"tag":"Global","value":{"type":"unit"}}},"value":{"type":"variant","value":{"tag":"Bool","value":{"type":"bool","value":true}}}}}}}}' \
              | "${settingsComposition}/bin/phenix" > "$TMPDIR/settings-runtime-set.json"
            printf '%s\n' '{"id":4,"service":"phenix.options@1","input":{"type":"variant","value":{"tag":"Resolve","value":{"type":"table","value":{"key":{"type":"string","value":"session.auto_create"},"context":{"type":"table","value":{"session":{"type":"option","value":null},"agent":{"type":"option","value":{"type":"string","value":"agent.scout"}}}}}}}}}' \
              | "${settingsComposition}/bin/phenix" > "$TMPDIR/settings-runtime-resolve.json"
            assert_option "$TMPDIR/settings-runtime-resolve.json" true Runtime

            export PHENIX_STATE_DB="$TMPDIR/settings-file-first.sqlite"
            printf '%s\n' '{"id":1,"service":"phenix.options@1","input":{"type":"variant","value":{"tag":"Resolve","value":{"type":"table","value":{"key":{"type":"string","value":"session.auto_create"},"context":{"type":"table","value":{"session":{"type":"option","value":null},"agent":{"type":"option","value":null}}}}}}}}' \
              | "${filePrecedenceComposition}/bin/phenix" > "$TMPDIR/settings-file-first.json"
            assert_option "$TMPDIR/settings-file-first.json" true File

            export PHENIX_STATE_DB="$TMPDIR/session-only.sqlite"
            "${sessionOnlyComposition}/bin/phenix" --list-services > "$TMPDIR/session-only.json"
            jq -e '(.plugins == ["phenix.sessions"]) and (.services | index("phenix.sessions@1") != null) and (.services | index("phenix.context@1") == null)' "$TMPDIR/session-only.json" >/dev/null

            export PHENIX_STATE_DB="$TMPDIR/context-only.sqlite"
            "${contextOnlyComposition}/bin/phenix" --list-services > "$TMPDIR/context-only.json"
            jq -e '((.plugins | sort) == ["phenix.context", "phenix.execution"]) and (.services | index("phenix.context@1") != null) and (.services | index("phenix.sessions@1") == null)' "$TMPDIR/context-only.json" >/dev/null

            export PHENIX_STATE_DB="$TMPDIR/resource.sqlite"
            test -e "${resourceComposition}/share/phenix-plugin/resources/README.txt"
            "${resourceComposition}/bin/phenix" --list-services > "$TMPDIR/resource-services.json"
            jq -e '(.plugins | index("fixture.resources")) != null' "$TMPDIR/resource-services.json" >/dev/null

            test -x "${conductorComposition}/bin/phenix-conductor"
            test ! -e "${conductorComposition}/bin/phenix"
            test ! -e "${conductorComposition}/bin/phenix-harness"
            "${conductorComposition}/bin/phenix-conductor" --list-services > "$TMPDIR/conductor-services.json"
            jq -e '(.plugins == []) and (.services == [])' "$TMPDIR/conductor-services.json" >/dev/null
            touch "$out"
          '';
    };
}
