{ self, ... }:
let
  mkPhenixPlugin =
    {
      pkgs,
      name,
      manifest,
      package ? null,
      executable ? null,
      resources ? null,
    }:
    let
      execution = manifest.execution or null;
      isEmbedded = execution == "embedded";
      isExternal = execution == "external";
      isResourceOnly = execution == "resource-only";
      metadataDirectory = if isEmbedded then "share/phenix-plugins/${name}" else "share/phenix-plugin";
    in
    assert isEmbedded || isExternal || isResourceOnly;
    assert (!isEmbedded) || (package != null && executable == null);
    assert (!isExternal) || executable != null;
    assert (!isResourceOnly) || executable == null;
    pkgs.runCommand "phenix-plugin-${name}"
      {
        nativeBuildInputs = [ pkgs.jq ];
        passAsFile = [ "manifestJson" ];
        manifestJson = builtins.toJSON manifest;
        passthru = {
          phenixPluginId = manifest.id;
          phenixPluginExecution = execution;
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
        ${pkgs.lib.optionalString isExternal ''
          test -x "${executable}"
          mkdir -p "$out/bin"
          ln -s "${executable}" "$out/bin/${name}"
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
      selectedIds =
        if enabledPlugins != null then
          enabledPlugins
        else if embeddedPlugins != [ ] then
          selectedEmbeddedIds
        else
          null;
    in
    if plugins == [ ] && resources == [ ] && selectedIds == null && layerPolicies == [ ] then
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
                      --set PHENIX_RUNTIME_CONFIG "$out/share/phenix/runtime.json" \
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
          execution = "resource-only";
          dependencies = [ ];
          services = [ ];
          resource_namespaces = [ ];
          maximum_authority = [ ];
        };
        resources = fixtureResources;
      };
      externalExecutable = pkgs.writeShellScript "external-fixture" ''
        set -euo pipefail
        while IFS= read -r frame; do
          type="$(printf '%s' "$frame" | ${pkgs.jq}/bin/jq -r .type)"
          case "$type" in
            handshake)
              generation="$(printf '%s' "$frame" | ${pkgs.jq}/bin/jq -r .generation)"
              ${pkgs.jq}/bin/jq -cn --argjson generation "$generation" \
                '{type:"handshake_ok",protocol:3,plugin:"fixture.session-replacement",generation:$generation,services:[{service:"phenix.sessions@1",role:"terminal"}]}'
              ;;
            invoke)
              request_id="$(printf '%s' "$frame" | ${pkgs.jq}/bin/jq -r .request_id)"
              generation="$(printf '%s' "$frame" | ${pkgs.jq}/bin/jq -r .generation)"
              ${pkgs.jq}/bin/jq -cn --argjson request_id "$request_id" --argjson generation "$generation" \
                '{type:"result",request_id:$request_id,generation:$generation,output:[123,34,114,101,112,108,97,99,101,109,101,110,116,34,58,116,114,117,101,125]}'
              ;;
            stop) exit 0 ;;
            *) exit 2 ;;
          esac
        done
      '';
      externalPlugin = mkPhenixPlugin {
        inherit pkgs;
        name = "external-fixture";
        manifest = {
          id = "fixture.session-replacement";
          version = 1;
          execution = "external";
          dependencies = [ ];
          services = [
            {
              role = "terminal";
              service = "phenix.sessions@1";
              priority = 200;
              required_authority = [ ];
            }
          ];
          resource_namespaces = [ ];
          maximum_authority = [ ];
        };
        executable = externalExecutable;
      };
      externalLayerExecutable = pkgs.writeShellScript "external-layer-fixture" ''
        set -euo pipefail
        while IFS= read -r frame; do
          type="$(printf '%s' "$frame" | ${pkgs.jq}/bin/jq -r .type)"
          case "$type" in
            handshake)
              generation="$(printf '%s' "$frame" | ${pkgs.jq}/bin/jq -r .generation)"
              ${pkgs.jq}/bin/jq -cn --argjson generation "$generation" \
                '{type:"handshake_ok",protocol:3,plugin:"fixture.session-layer",generation:$generation,services:[{service:"phenix.sessions@1",role:"layer"}]}'
              ;;
            invoke)
              request_id="$(printf '%s' "$frame" | ${pkgs.jq}/bin/jq -r .request_id)"
              generation="$(printf '%s' "$frame" | ${pkgs.jq}/bin/jq -r .generation)"
              continuation="$(printf '%s' "$frame" | ${pkgs.jq}/bin/jq -r .continuation)"
              input="$(printf '%s' "$frame" | ${pkgs.jq}/bin/jq -c .input)"
              authority="$(printf '%s' "$frame" | ${pkgs.jq}/bin/jq -c .authority)"
              ${pkgs.jq}/bin/jq -cn --argjson request_id "$request_id" --argjson generation "$generation" \
                --argjson continuation "$continuation" --argjson input "$input" --argjson authority "$authority" \
                '{type:"continue",request_id:$request_id,generation:$generation,continuation:$continuation,input:$input,authority:$authority}'
              read -r continued
              test "$(printf '%s' "$continued" | ${pkgs.jq}/bin/jq -r .type)" = "continuation_result"
              ${pkgs.jq}/bin/jq -cn --argjson request_id "$request_id" --argjson generation "$generation" \
                '{type:"result",request_id:$request_id,generation:$generation,output:[123,34,101,120,116,101,114,110,97,108,95,108,97,121,101,114,34,58,116,114,117,101,125]}'
              ;;
            stop) exit 0 ;;
            *) exit 2 ;;
          esac
        done
      '';
      externalLayerPlugin = mkPhenixPlugin {
        inherit pkgs;
        name = "external-layer-fixture";
        manifest = {
          id = "fixture.session-layer";
          version = 1;
          execution = "external";
          dependencies = [ ];
          services = [
            {
              role = "layer";
              service = "phenix.sessions@1";
              priority = 300;
              required_authority = [ ];
            }
          ];
          resource_namespaces = [ ];
          maximum_authority = [
            "kernel.persistence.read"
            "kernel.persistence.write"
          ];
        };
        executable = externalLayerExecutable;
      };
      defaultPluginNames = [
        "artifacts"
        "cli"
        "context"
        "debug"
        "execution"
        "frontend"
        "hooks"
        "jobs"
        "language"
        "models"
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
      externalComposition = mkPhenix {
        inherit pkgs;
        plugins = defaultPlugins ++ [ externalPlugin ];
        resources = [ harnessResources ];
      };
      externalLayerComposition = mkPhenix {
        inherit pkgs;
        plugins = defaultPlugins ++ [ externalLayerPlugin ];
        resources = [ harnessResources ];
        layerPolicies = [
          {
            service = "phenix.sessions@1";
            plugin = "fixture.session-layer";
            priority = 300;
            required = true;
            enabled = true;
          }
        ];
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
            jq -e '(.plugins | length == 15) and ([.plugins[] | select(startswith("phenix.basic-"))] | length == 0) and (.services | index("phenix.sessions@1") != null)' "$TMPDIR/default-services.json" >/dev/null

            export PHENIX_STATE_DB="$TMPDIR/external.sqlite"
            "${externalComposition}/bin/phenix" --list-services > "$TMPDIR/external-services.json"
            jq -e '(.plugins | index("fixture.session-replacement")) != null' "$TMPDIR/external-services.json" >/dev/null
            printf '%s\n' '{"id":1,"service":"phenix.sessions@1","input":{"operation":"get","id":"missing"}}' \
              | "${externalComposition}/bin/phenix" > "$TMPDIR/replacement.json"
            jq -e '.status == "ok" and .output.replacement == true' "$TMPDIR/replacement.json" >/dev/null

            export PHENIX_STATE_DB="$TMPDIR/external-layer.sqlite"
            "${externalLayerComposition}/bin/phenix" --list-services > "$TMPDIR/external-layer-services.json"
            jq -e '(.plugins | index("fixture.session-layer")) != null' "$TMPDIR/external-layer-services.json" >/dev/null
            printf '%s\n' '{"id":1,"service":"phenix.sessions@1","input":{"operation":"get","id":"missing"}}' \
              | "${externalLayerComposition}/bin/phenix" > "$TMPDIR/external-layer.json"
            jq -e '.status == "ok" and .output.external_layer == true' "$TMPDIR/external-layer.json" >/dev/null

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
