{ inputs, self, ... }:
let
  systems = [
    "x86_64-linux"
    "aarch64-linux"
  ];

  mkRustPackage =
    pkgs: package:
    pkgs.rustPlatform.buildRustPackage {
      pname = package;
      version = "0";
      src = pkgs.lib.cleanSource ../rust;
      cargoLock.lockFile = ../rust/Cargo.lock;
      cargoBuildFlags = [
        "--package"
        package
      ];
      doCheck = false;
      installPhase = ''
        runHook preInstall
        mkdir -p "$out/share/phenix-rust-package"
        printf '%s\n' ${pkgs.lib.escapeShellArg package} > "$out/share/phenix-rust-package/name"
        runHook postInstall
      '';
    };

  mkEmbeddedPluginPackage =
    pkgs: package: pkgs.writeTextDir "share/phenix-rust-package/name" "${package}\n";

  mkBinaryPackage =
    pkgs: package: binary:
    pkgs.rustPlatform.buildRustPackage {
      pname = package;
      version = "0";
      src = pkgs.lib.cleanSource ../rust;
      cargoLock.lockFile = ../rust/Cargo.lock;
      cargoBuildFlags = [
        "--package"
        package
        "--bin"
        binary
      ];
      doCheck = false;
      installPhase = ''
        runHook preInstall
        mkdir -p "$out/bin"
        executable="$(find target -path '*/release/${binary}' -type f -print -quit)"
        test -n "$executable"
        cp "$executable" "$out/bin/${binary}"
        runHook postInstall
      '';
    };

  pluginIds = {
    artifacts = "phenix.artifacts";
    basic-context = "phenix.basic-context";
    basic-model = "phenix.basic-model";
    basic-skills = "phenix.basic-skills";
    basic-tools = "phenix.basic-tools";
    cli = "phenix.cli";
    context = "phenix.context";
    debug = "phenix.debug";
    execution = "phenix.execution";
    frontend = "phenix.frontend-services";
    hooks = "phenix.hooks";
    jobs = "phenix.jobs";
    language = "phenix.language";
    models = "phenix.models";
    options = "phenix.options";
    planning = "phenix.planning";
    repository-workers = "phenix.repository-workers";
    sdk = "phenix.sdk";
    session-tree = "phenix.session-tree";
    sessions = "phenix.sessions";
    workspace = "phenix.workspace";
  };

  basicPluginNames = [
    "basic-context"
    "basic-model"
    "basic-skills"
    "basic-tools"
  ];

  pluginCrates = builtins.mapAttrs (
    name: _:
    if builtins.elem name basicPluginNames then "phenix-plugin-basic-agent" else "phenix-plugin-${name}"
  ) pluginIds;

  pluginSets = builtins.listToAttrs (
    map (
      system:
      let
        pkgs = inputs.nixpkgs.legacyPackages.${system};
      in
      {
        name = system;
        value = builtins.mapAttrs (
          name: pluginId:
          self.lib.mkPhenixPlugin {
            inherit pkgs name;
            package = mkEmbeddedPluginPackage pkgs pluginCrates.${name};
            manifest = {
              id = pluginId;
              version = 1;
              execution = "embedded";
            };
          }
        ) pluginIds;
      }
    ) systems
  );

  clientSets = builtins.listToAttrs (
    map (
      system:
      let
        pkgs = inputs.nixpkgs.legacyPackages.${system};
      in
      {
        name = system;
        value.acp = self.lib.mkPhenixClient {
          inherit pkgs;
          name = "acp";
          package = mkRustPackage pkgs "phenix-acp";
        };
      }
    ) systems
  );
in
{
  flake = {
    phenixPlugins = pluginSets;
    phenixClients = clientSets;
  };

  perSystem =
    { pkgs, system, ... }:
    let
      pluginPackageChecks = pkgs.lib.mapAttrs' (name: package: {
        name = "phenix-plugin-${name}-package";
        value = package;
      }) pluginSets.${system};
    in
    {
      packages = {
        phenix-core = mkRustPackage pkgs "phenix-core";
        phenix-client = mkRustPackage pkgs "phenix-client";
        phenix-conductor = mkBinaryPackage pkgs "phenix-conductor" "phenix-conductor";
      };

      checks = pluginPackageChecks;
    };
}
