{ inputs, ... }:
{
  perSystem =
    {
      pkgs,
      self',
      system,
      ...
    }:
    let
      maintenanceLib = inputs.phenix-flake-ci.lib;

      repositoryRoot = ''
        repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
        cd "$repo_root"
      '';
      rustRoot = ''
        ${repositoryRoot}
        cd rust
      '';

      sourceCi = {
        enable = true;
        stage = "source";
        name = "Source";
        timeoutMinutes = 60;
      };

      integrationTargets = [
        {
          id = "backend-acp-persistent-continuity";
          package = "phenix-backend-acp";
          test = "persistent_continuity";
          label = "phenix-backend-acp / persistent_continuity";
        }
        {
          id = "backend-acp-tool-bridge";
          package = "phenix-backend-acp";
          test = "tool_bridge";
          label = "phenix-backend-acp / tool_bridge";
        }
      ];

      testTargets = [
        {
          id = "phenix-adapter-acp-runtime-plugin";
          package = "phenix-adapter-acp";
          test = "runtime_plugin";
          label = "phenix-adapter-acp / runtime_plugin";
        }
        {
          id = "phenix-domain-context-serialization";
          package = "phenix-domain";
          test = "context_serialization";
          label = "phenix-domain / context_serialization";
        }
        {
          id = "sdk-plugin-authoring";
          package = "phenix-sdk";
          test = "plugin_authoring";
          label = "phenix-sdk / plugin_authoring";
        }
        {
          id = "sdk-plugin-attribute-only-gate";
          package = "phenix-sdk";
          test = "plugin_attribute_only_gate";
          label = "phenix-sdk / plugin_attribute_only_gate";
        }
        {
          id = "sdk-plugin-component-authoring";
          package = "phenix-sdk";
          test = "plugin_component_authoring";
          label = "phenix-sdk / plugin_component_authoring";
        }
        {
          id = "sdk-plugin-config-authoring";
          package = "phenix-sdk";
          test = "plugin_config_authoring";
          label = "phenix-sdk / plugin_config_authoring";
        }
        {
          id = "sdk-plugin-dependency-authoring";
          package = "phenix-sdk";
          test = "plugin_dependency_authoring";
          label = "phenix-sdk / plugin_dependency_authoring";
        }
        {
          id = "sdk-plugin-import-authoring";
          package = "phenix-sdk";
          test = "plugin_import_authoring";
          label = "phenix-sdk / plugin_import_authoring";
        }
        {
          id = "sdk-plugin-layer-authority";
          package = "phenix-sdk";
          test = "plugin_layer_authority";
          label = "phenix-sdk / plugin_layer_authority";
        }
        {
          id = "sdk-plugin-lifecycle-authoring";
          package = "phenix-sdk";
          test = "plugin_lifecycle_authoring";
          label = "phenix-sdk / plugin_lifecycle_authoring";
        }
        {
          id = "sdk-plugin-manifest-authoring";
          package = "phenix-sdk";
          test = "plugin_manifest_authoring";
          label = "phenix-sdk / plugin_manifest_authoring";
        }
        {
          id = "sdk-plugin-public-projection";
          package = "phenix-sdk";
          test = "plugin_public_projection";
          label = "phenix-sdk / plugin_public_projection";
        }
        {
          id = "sdk-incompatible-schema";
          package = "phenix-sdk";
          test = "incompatible_schema";
          label = "phenix-sdk / incompatible_schema";
        }
        {
          id = "sdk-plugin-resource-authoring";
          package = "phenix-sdk";
          test = "plugin_resource_authoring";
          label = "phenix-sdk / plugin_resource_authoring";
        }
        {
          id = "sdk-plugin-stateless-manifest-authoring";
          package = "phenix-sdk";
          test = "plugin_stateless_manifest_authoring";
          label = "phenix-sdk / plugin_stateless_manifest_authoring";
        }
        {
          id = "harness-component-graph";
          package = "phenix-harness";
          test = "component_graph";
          label = "phenix-harness / component_graph";
        }
        {
          id = "harness-supported-product";
          package = "phenix-harness";
          test = "supported_product_journeys";
          label = "harness / supported_product_journeys";
        }
      ];

      runtimeTargets = [
        {
          id = "harness-process-roundtrip";
          package = "phenix-harness";
          test = "process_roundtrip";
          label = "harness / process_roundtrip";
        }
      ];

      cargoTestTargets = testTargets ++ runtimeTargets ++ integrationTargets;

      mkCargoSuite = target: {
        name = target.label;
        runtimeInputs = pkgs: [
          pkgs.cargo
          pkgs.git
          pkgs.rustc
        ];
        exec = ''
          ${rustRoot}
          cargo test --quiet --locked -p ${target.package} --test ${target.test}
        '';
      };

      mkCargoSuites =
        targets:
        builtins.listToAttrs (
          builtins.map (target: {
            name = target.id;
            value = mkCargoSuite target;
          }) targets
        );

      mkProductSuite =
        {
          check,
          name,
        }:
        {
          inherit name;
          runtimeInputs = pkgs: [
            pkgs.git
            pkgs.nix
          ];
          exec = ''
            ${repositoryRoot}
            system="$(nix eval --impure --raw --expr builtins.currentSystem)"
            nix build --no-link --print-build-logs ".#checks.$system.${check}"
          '';
        };

      ciCommands = maintenanceLib.mkCi {
        ci = {
          name = "CI";
          timeoutMinutes = 120;
          needs = [ "source" ];
          env = {
            CARGO_HOME = "\${{ runner.temp }}/phenix-cargo-home";
            CARGO_TARGET_DIR = "\${{ runner.temp }}/phenix-cargo-target";
            CARGO_TERM_QUIET = "true";
          };
        };

        build.rust-workspace = {
          name = "Rust workspace";
          runtimeInputs = pkgs: [
            pkgs.cargo
            pkgs.git
            pkgs.rustc
          ];
          exec = ''
            ${rustRoot}
            cargo build --workspace --locked --quiet
          '';
        };

        test = {
          unit = {
            name = "Rust unit tests";
            runtimeInputs = pkgs: [
              pkgs.bash
              pkgs.bubblewrap
              pkgs.cargo
              pkgs.coreutils
              pkgs.git
              pkgs.iproute2
              pkgs.ripgrep
              pkgs.rsync
              pkgs.rustc
              pkgs.slirp4netns
              pkgs.socat
              pkgs.util-linux
            ];
            exec = ''
              ${rustRoot}

              if [ "''${GITHUB_ACTIONS:-}" = "true" ] \
                && [ -r /proc/sys/kernel/apparmor_restrict_unprivileged_userns ] \
                && [ "$(cat /proc/sys/kernel/apparmor_restrict_unprivileged_userns)" = "1" ]; then
                /usr/bin/sudo -n /usr/sbin/sysctl \
                  -w kernel.apparmor_restrict_unprivileged_userns=0 >/dev/null
              fi

              timeout --signal=KILL 300 \
                cargo test --quiet --workspace --lib --bins --locked -- --nocapture --test-threads=1
            '';
          };

          docs = {
            name = "Rust doc tests";
            runtimeInputs = pkgs: [
              pkgs.cargo
              pkgs.git
              pkgs.rustc
            ];
            exec = ''
              ${rustRoot}
              cargo test --quiet --workspace --doc --locked
            '';
          };
        }
        // mkCargoSuites testTargets;

        runtime = mkCargoSuites runtimeTargets;
        integration = mkCargoSuites integrationTargets;

        product = {
          phenix = mkProductSuite {
            check = "phenix-product-smoke";
            name = "Phenix product smoke";
          };
          plugin-packaging = mkProductSuite {
            check = "phenix-plugin-packaging";
            name = "Plugin packaging";
          };
          stitch-runtime = mkProductSuite {
            check = "stitch-runtime-smoke";
            name = "Stitch runtime smoke";
          };
          stitch-mcp = mkProductSuite {
            check = "stitch-mcp-package";
            name = "Stitch MCP package";
          };
        };
      };

      expectedCargoTargetLines = builtins.concatStringsSep "\n" (
        builtins.map (target: "printf '%s\\t%s\\n' '${target.package}' '${target.test}'") cargoTestTargets
      );

      maintenance = maintenanceLib.mkMaintenance {
        name = "maintenance";
        description = "Phenix maintenance";
        ci.github = {
          enable = true;
          outputName = "phenix-maintenance";
        };
        gitHooks = {
          enable = true;
          preCommit = [ "fix" ];
        };

        commands = ciCommands // {
          all = {
            description = "Run static validation and the complete semantic CI pipeline";
            dependencies = [
              [ "check" ]
              [ "pipeline" ]
            ];
            exec = ''
              "$0" check
              "$0" pipeline
            '';
          };

          check = {
            description = "Run static/source validation";
            order = [
              "source"
              "rust"
            ];
            commands = {
              source = {
                description = "Formatting, source analysis, test classification, and workflow consistency";
                order = [
                  "nix-format"
                  "rust-format"
                  "statix"
                  "actionlint"
                  "plugin-architecture"
                  "structural-boundaries"
                  "test-targets"
                  "workflow-sync"
                ];
                commands = {
                  nix-format = {
                    description = "Nix formatting";
                    ci = sourceCi // {
                      stepName = "Nix formatting";
                    };
                    runtimeInputs = pkgs: [
                      pkgs.findutils
                      pkgs.git
                      pkgs.nixfmt
                    ];
                    exec = ''
                      ${repositoryRoot}
                      find . -type f -name '*.nix' \
                        -not -path './.git/*' \
                        -print0 |
                        xargs -0 -r nixfmt --check
                    '';
                  };

                  rust-format = {
                    description = "Rust formatting";
                    ci = sourceCi // {
                      stepName = "Rust formatting";
                    };
                    runtimeInputs = pkgs: [
                      pkgs.cargo
                      pkgs.git
                      pkgs.rustfmt
                    ];
                    exec = ''
                      ${rustRoot}
                      cargo fmt --all --check
                    '';
                  };

                  statix = {
                    description = "Nix static analysis";
                    ci = sourceCi // {
                      stepName = "Statix";
                    };
                    runtimeInputs = pkgs: [
                      pkgs.git
                      pkgs.statix
                    ];
                    exec = ''
                      ${repositoryRoot}
                      statix check --ignore '.git/**'
                    '';
                  };

                  actionlint = {
                    description = "GitHub Actions syntax";
                    ci = sourceCi // {
                      stepName = "Actionlint";
                    };
                    runtimeInputs = pkgs: [
                      pkgs.actionlint
                      pkgs.findutils
                      pkgs.git
                    ];
                    exec = ''
                      ${repositoryRoot}
                      find .github/workflows -type f \
                        \( -name '*.yml' -o -name '*.yaml' \) -print0 |
                        xargs -0 -r actionlint
                    '';
                  };

                  plugin-architecture = {
                    description = "Plugin package roles and runtime dependency boundaries";
                    ci = sourceCi // {
                      stepName = "Plugin architecture";
                    };
                    runtimeInputs = pkgs: [
                      pkgs.bash
                      pkgs.cargo
                      pkgs.git
                      pkgs.jq
                    ];
                    exec = ''
                      ${repositoryRoot}
                      bash scripts/check-plugin-architecture.sh
                      bash scripts/check-spec-lifecycle-fixtures.sh
                      bash scripts/check-spec-lifecycle.sh
                    '';
                  };

                  structural-boundaries = {
                    description = "Canonical structural boundary enforcement";
                    ci = sourceCi // {
                      stepName = "Structural boundaries";
                    };
                    runtimeInputs = pkgs: [
                      pkgs.bash
                      pkgs.git
                    ];
                    exec = ''
                      ${repositoryRoot}
                      bash scripts/check-structural-boundaries.sh
                    '';
                  };

                  test-targets = {
                    description = "Every Cargo integration target has an explicit semantic CI phase";
                    ci = sourceCi // {
                      stepName = "Test target classification";
                    };
                    runtimeInputs = pkgs: [
                      pkgs.cargo
                      pkgs.coreutils
                      pkgs.diffutils
                      pkgs.git
                      pkgs.jq
                    ];
                    exec = ''
                      ${rustRoot}
                      expected="$(mktemp)"
                      actual="$(mktemp)"
                      trap 'rm -f "$expected" "$actual"' EXIT

                      {
                        ${expectedCargoTargetLines}
                      } | sort > "$expected"

                      cargo metadata --format-version 1 --no-deps |
                        jq -r '
                          .packages[]
                          | . as $package
                          | .targets[]
                          | select(.kind == ["test"])
                          | [$package.name, .name]
                          | @tsv
                        ' |
                        sort > "$actual"

                      diff -u "$expected" "$actual"
                    '';
                  };

                  workflow-sync = {
                    description = "Committed GitHub workflow matches the Nix CI declaration";
                    ci = sourceCi // {
                      stepName = "Generated workflow";
                    };
                    runtimeInputs = pkgs: [
                      pkgs.diffutils
                      pkgs.git
                      pkgs.nix
                    ];
                    exec = ''
                      ${repositoryRoot}
                      system="$(nix eval --impure --raw --expr builtins.currentSystem)"
                      generated="$(mktemp)"
                      trap 'rm -f "$generated"' EXIT
                      nix eval --raw \
                        ".#packages.$system.phenix-maintenance.phenixMaintenance.ci.github.workflow" \
                        > "$generated"
                      diff -u .github/workflows/ci.yml "$generated"
                    '';
                  };
                };
              };

              rust = {
                description = "Rust static analysis with Clippy";
                ci = sourceCi // {
                  stepName = "Clippy";
                };
                runtimeInputs = pkgs: [
                  pkgs.cargo
                  pkgs.clippy
                  pkgs.git
                  pkgs.rustc
                ];
                exec = ''
                  ${rustRoot}
                  cargo clippy --quiet --workspace --all-targets --locked -- -D warnings
                '';
              };
            };
          };

          fix = {
            description = "Apply deterministic Nix and Rust normalization";
            runtimeInputs = pkgs: [
              pkgs.cargo
              pkgs.findutils
              pkgs.git
              pkgs.nixfmt
              pkgs.rustfmt
              pkgs.statix
            ];
            exec = ''
              ${repositoryRoot}

              statix fix

              find . -type f -name '*.nix' \
                -not -path './.git/*' \
                -print0 |
                xargs -0 -r nixfmt

              (
                cd rust
                cargo fmt --all
              )
            '';
          };
        };
      };

      maintenancePackage = maintenanceLib.mkMaintenancePackage {
        inherit pkgs maintenance;
      };
      maintenanceOutputs = maintenanceLib.mkMaintenanceOutputs {
        inherit maintenance;
        systems = [ system ];
        pkgsFor = _: pkgs;
        outputName = "phenix-maintenance";
      };
    in
    {
      packages = maintenanceOutputs.packages.${system};
      apps = maintenanceOutputs.apps.${system};

      devShells.default = pkgs.mkShell {
        name = "phenix-dev";
        packages = [
          pkgs.actionlint
          pkgs.bubblewrap
          pkgs.cargo
          pkgs.clippy
          pkgs.git
          pkgs.jq
          pkgs.lua-language-server
          pkgs.nixd
          pkgs.nixfmt
          pkgs.rsync
          pkgs.rust-analyzer
          pkgs.rustc
          pkgs.rustfmt
          pkgs.slirp4netns
          pkgs.statix
          pkgs.taplo
          maintenancePackage.package
          self'.packages.stitch
          self'.packages.stitch-mcp
        ];
        shellHook = ''
          ${maintenancePackage.shellHook}

          echo "phenix dev shell"
          echo "  all:          maintenance all"
          echo "  static:       maintenance check"
          echo "  build:        maintenance build"
          echo "  tests:        maintenance test"
          echo "  runtime:      maintenance runtime"
          echo "  integration:  maintenance integration"
          echo "  product:      maintenance product"
          echo "  fixes:        maintenance fix"
        '';
      };
    };
}
