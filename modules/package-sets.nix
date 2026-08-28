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
    planning = "phenix.planning";
    repository-workers = "phenix.repository-workers";
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
            package = mkRustPackage pkgs pluginCrates.${name};
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

      pluginOwnershipCheck =
        pkgs.runCommand "phenix-plugin-package-ownership"
          {
            nativeBuildInputs = [ pkgs.ripgrep ];
            src = pkgs.lib.cleanSource ../rust;
          }
          ''
            cd "$src"

            if rg -n --glob 'crates/*/Cargo.toml' \
              'phenix-kernel[[:space:]]*=[[:space:]]*\{[[:space:]]*package[[:space:]]*=[[:space:]]*"phenix-core"' crates; then
              echo 'stale phenix-kernel Cargo alias remains' >&2
              exit 1
            fi

            if rg -n --glob 'crates/*/Cargo.toml' \
              'phenix-plugin-suite[[:space:]]*=[[:space:]]*\{[[:space:]]*package[[:space:]]*=[[:space:]]*"phenix-plugin-catalog"' crates; then
              echo 'stale phenix-plugin-suite Cargo alias remains' >&2
              exit 1
            fi

            if rg -n --glob 'crates/*/Cargo.toml' \
              'phenix-core[[:space:]]*=[[:space:]]*\{[[:space:]]*package[[:space:]]*=[[:space:]]*"phenix-domain"' crates; then
              echo 'stale phenix-domain-as-core Cargo alias remains' >&2
              exit 1
            fi

            touch "$out"
          '';
    in
    {
      packages = {
        phenix-core = mkRustPackage pkgs "phenix-core";
        phenix-client = mkRustPackage pkgs "phenix-client";
        phenix-conductor = mkBinaryPackage pkgs "phenix-conductor" "phenix-conductor";
      };

      checks = pluginPackageChecks // {
        phenix-plugin-package-ownership = pluginOwnershipCheck;
      };
    };
}
