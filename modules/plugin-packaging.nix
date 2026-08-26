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
    if plugins == [ ] && resources == [ ] && selectedIds == null then
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
                '{type:"handshake_ok",protocol:1,plugin:"fixture.session-replacement",generation:$generation,services:["phenix.sessions@1"]}'
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
      firstPartyPlugins = builtins.attrValues self.phenixPlugins.${pkgs.system};
      harnessResources = self.packages.${pkgs.system}.phenix-harness-resources;
      defaultComposition = mkPhenix {
        inherit pkgs;
        plugins = firstPartyPlugins;
        resources = [ harnessResources ];
      };
      externalComposition = mkPhenix {
        inherit pkgs;
        plugins = firstPartyPlugins ++ [ externalPlugin ];
        resources = [ harnessResources ];
      };
      resourceComposition = mkPhenix {
        inherit pkgs;
        plugins = firstPartyPlugins ++ [ resourcePlugin ];
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
            jq -e '(.plugins | length == 14) and (.services | index("phenix.sessions@1") != null)' "$TMPDIR/default-services.json" >/dev/null

            export PHENIX_STATE_DB="$TMPDIR/external.sqlite"
            "${externalComposition}/bin/phenix" --list-services > "$TMPDIR/external-services.json"
            jq -e '(.plugins | index("fixture.session-replacement")) != null' "$TMPDIR/external-services.json" >/dev/null
            printf '%s\n' '{"id":1,"service":"phenix.sessions@1","input":{"operation":"get","id":"missing"}}' \
              | "${externalComposition}/bin/phenix" > "$TMPDIR/replacement.json"
            jq -e '.status == "ok" and .output.replacement == true' "$TMPDIR/replacement.json" >/dev/null

            export PHENIX_STATE_DB="$TMPDIR/session-only.sqlite"
            "${sessionOnlyComposition}/bin/phenix" --list-services > "$TMPDIR/session-only.json"
            jq -e '(.plugins == ["phenix.sessions"]) and (.services | index("phenix.sessions@1") != null) and (.services | index("phenix.context@1") == null)' "$TMPDIR/session-only.json" >/dev/null

            export PHENIX_STATE_DB="$TMPDIR/context-only.sqlite"
            "${contextOnlyComposition}/bin/phenix" --list-services > "$TMPDIR/context-only.json"
            jq -e '(.plugins == ["phenix.context"]) and (.services | index("phenix.context@1") != null) and (.services | index("phenix.sessions@1") == null)' "$TMPDIR/context-only.json" >/dev/null

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
