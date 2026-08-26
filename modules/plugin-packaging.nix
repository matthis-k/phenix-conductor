{ self, ... }:
let
  mkPhenixPlugin =
    {
      pkgs,
      name,
      manifest,
      executable ? null,
      resources ? null,
    }:
    let
      execution = manifest.execution or null;
      isExternal = execution == "external";
      isResourceOnly = execution == "resource-only";
    in
    assert isExternal || isResourceOnly;
    assert (!isExternal) || executable != null;
    assert (!isResourceOnly) || executable == null;
    pkgs.runCommand "phenix-plugin-${name}"
      {
        nativeBuildInputs = [ pkgs.jq ];
        passAsFile = [ "manifestJson" ];
        manifestJson = builtins.toJSON manifest;
      }
      ''
        set -euo pipefail
        mkdir -p "$out/share/phenix-plugin"
        jq -e 'type == "object" and (.id | type == "string" and length > 0)' \
          "$manifestJsonPath" >/dev/null
        cp "$manifestJsonPath" "$out/share/phenix-plugin/manifest.json"

        ${
          if isExternal then
            ''
              test -x "${executable}"
              mkdir -p "$out/bin"
              ln -s "${executable}" "$out/bin/${name}"
            ''
          else
            ""
        }

        ${
          if resources != null then
            ''
              test -e "${resources}"
              ln -s "${resources}" "$out/share/phenix-plugin/resources"
            ''
          else
            ""
        }
      '';

  mkPhenix =
    {
      pkgs,
      kernelOnly ? false,
      plugins ? [ ],
      ...
    }:
    let
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
        nativeBuildInputs = [ pkgs.makeWrapper ];
        postBuild =
          if kernelOnly then
            ""
          else
            let
              pluginPackages = pkgs.lib.concatStringsSep ":" (map toString plugins);
            in
            ''
              for program in phenix phenix-harness; do
                if [ -e "$out/bin/$program" ]; then
                  wrapProgram "$out/bin/$program" \
                    --set PHENIX_PLUGIN_PACKAGES ${pkgs.lib.escapeShellArg pluginPackages}
                fi
              done
            '';
      };
in
{
  flake = {
    lib = {
      inherit mkPhenix mkPhenixPlugin;
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
              ${pkgs.jq}/bin/jq -cn \
                --argjson generation "$generation" \
                '{type:"handshake_ok",protocol:1,plugin:"fixture.session-replacement",generation:$generation,services:["phenix.sessions@1"]}'
              ;;
            invoke)
              request_id="$(printf '%s' "$frame" | ${pkgs.jq}/bin/jq -r .request_id)"
              generation="$(printf '%s' "$frame" | ${pkgs.jq}/bin/jq -r .generation)"
              ${pkgs.jq}/bin/jq -cn \
                --argjson request_id "$request_id" \
                --argjson generation "$generation" \
                '{type:"result",request_id:$request_id,generation:$generation,output:[123,34,114,101,112,108,97,99,101,109,101,110,116,34,58,116,114,117,101,125]}'
              ;;
            stop)
              exit 0
              ;;
            *)
              exit 2
              ;;
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
      defaultComposition = mkPhenix { inherit pkgs; };
      externalComposition = mkPhenix {
        inherit pkgs;
        plugins = [ externalPlugin ];
      };
      resourceComposition = mkPhenix {
        inherit pkgs;
        plugins = [ resourcePlugin ];
      };
      kernelComposition = mkPhenix {
        inherit pkgs;
        kernelOnly = true;
        plugins = [ resourcePlugin ];
      };
    in
    {
      checks.phenix-plugin-packaging =
        pkgs.runCommand "phenix-plugin-packaging-check"
          {
            nativeBuildInputs = [ pkgs.jq ];
          }
          ''
            set -euxo pipefail
            test -x "${defaultComposition}/bin/phenix"
            test -x "${defaultComposition}/bin/phenix-harness"
            test ! -e "${defaultComposition}/bin/phenix-conductor"
            test -x "${externalComposition}/bin/phenix"
            test -x "${externalComposition}/bin/phenix-harness"
            test -x "${externalComposition}/bin/external-fixture"
            export PHENIX_STATE_DB="$TMPDIR/composition.sqlite"
            "${defaultComposition}/bin/phenix" --list-services > "$TMPDIR/default-services.json"
            jq -e '(.plugins | index("fixture.session-replacement")) == null' "$TMPDIR/default-services.json" >/dev/null
            "${externalComposition}/bin/phenix" --list-services > "$TMPDIR/external-services.json"
            jq -e '(.plugins | index("fixture.session-replacement")) != null' "$TMPDIR/external-services.json" >/dev/null
            printf '%s\n' '{"id":1,"service":"phenix.sessions@1","input":{"operation":"get","id":"missing"}}' \
              | "${externalComposition}/bin/phenix" > "$TMPDIR/replacement.json"
            jq -e '.status == "ok" and .output.replacement == true' "$TMPDIR/replacement.json" >/dev/null
            test -x "${resourceComposition}/bin/phenix"
            test -x "${resourceComposition}/bin/phenix-harness"
            test -e "${resourceComposition}/share/phenix-plugin/resources/README.txt"
            "${resourceComposition}/bin/phenix" --list-services > "$TMPDIR/resource-services.json"
            jq -e '(.plugins | index("fixture.resources")) != null' "$TMPDIR/resource-services.json" >/dev/null
            test -x "${kernelComposition}/bin/phenix-kernel"
            test ! -e "${kernelComposition}/bin/phenix"
            test ! -e "${kernelComposition}/bin/phenix-harness"
            test ! -e "${kernelComposition}/bin/phenix-conductor"
            test -e "${resourcePlugin}/share/phenix-plugin/resources/README.txt"
            jq -e '.id == "fixture.resources" and .execution == "resource-only"' \
              "${resourcePlugin}/share/phenix-plugin/manifest.json" >/dev/null
            test -x "${externalPlugin}/bin/external-fixture"
            jq -e '.id == "fixture.session-replacement" and .execution == "external"' \
              "${externalPlugin}/share/phenix-plugin/manifest.json" >/dev/null
            touch "$out"
          '';
    };
}
