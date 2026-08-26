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
          "$manifestJsonPath" > "$out/share/phenix-plugin/manifest.json"

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
          execution = "resource-only";
        };
        resources = fixtureResources;
      };
      externalExecutable = pkgs.writeShellScript "external-fixture" ''
        exit 0
      '';
      externalPlugin = mkPhenixPlugin {
        inherit pkgs;
        name = "external-fixture";
        manifest = {
          id = "fixture.external";
          execution = "external";
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
            set -euo pipefail
            test -x "${defaultComposition}/bin/phenix"
            test -x "${defaultComposition}/bin/phenix-harness"
            test ! -e "${defaultComposition}/bin/phenix-conductor"
            test -x "${externalComposition}/bin/phenix"
            test -x "${externalComposition}/bin/phenix-harness"
            test -x "${externalComposition}/bin/external-fixture"
            test -x "${resourceComposition}/bin/phenix"
            test -x "${resourceComposition}/bin/phenix-harness"
            test -e "${resourceComposition}/share/phenix-plugin/resources/README.txt"
            test -x "${kernelComposition}/bin/phenix-kernel"
            test ! -e "${kernelComposition}/bin/phenix"
            test ! -e "${kernelComposition}/bin/phenix-harness"
            test ! -e "${kernelComposition}/bin/phenix-conductor"
            test -e "${resourcePlugin}/share/phenix-plugin/resources/README.txt"
            jq -e '.id == "fixture.resources" and .execution == "resource-only"' \
              "${resourcePlugin}/share/phenix-plugin/manifest.json" >/dev/null
            test -x "${externalPlugin}/bin/external-fixture"
            jq -e '.id == "fixture.external" and .execution == "external"' \
              "${externalPlugin}/share/phenix-plugin/manifest.json" >/dev/null
            touch "$out"
          '';
    };
}
