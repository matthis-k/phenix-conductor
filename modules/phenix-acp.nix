{ self, ... }: {
  perSystem =
    { pkgs, system, ... }:
    let
      rustSource = pkgs.lib.cleanSource ../rust;

      phenixHarnessRuntime = pkgs.rustPlatform.buildRustPackage {
        pname = "phenix-harness-runtime";
        version = "0";
        src = rustSource;

        cargoLock.lockFile = ../rust/Cargo.lock;
        cargoBuildFlags = [
          "--package"
          "phenix-harness"
          "--bin"
          "phenix-harness"
        ];
        doCheck = false;

        installPhase = ''
          runHook preInstall
          mkdir -p "$out/bin"
          harness_binary="$(find target -path '*/release/phenix-harness' -type f -print -quit)"
          test -n "$harness_binary"
          cp "$harness_binary" "$out/bin/phenix-harness"
          ln -s phenix-harness "$out/bin/phenix"
          runHook postInstall
        '';
      };

      runtimeConfig = pkgs.writeText "phenix-runtime.json" (
        builtins.toJSON (import ../config/phenix/runtime.nix)
      );

      phenixHarnessResources = pkgs.runCommand "phenix-harness-resources" { } ''
        mkdir -p "$out/share/phenix/skills"
        cp ${runtimeConfig} "$out/share/phenix/runtime.json"
        cp -r ${../config/phenix/skills}/* "$out/share/phenix/skills/"
        cp ${../config/phenix/NOTICE.md} "$out/share/phenix/NOTICE.md"
      '';

      phenixAcpSmoke = pkgs.rustPlatform.buildRustPackage {
        pname = "phenix-acp-smoke";
        version = "0";
        src = rustSource;

        cargoLock.lockFile = ../rust/Cargo.lock;
        cargoBuildFlags = [
          "--package"
          "phenix-acp-presets"
          "--bin"
          "phenix-acp-smoke"
        ];
        doCheck = false;

        installPhase = ''
          runHook preInstall
          mkdir -p "$out/bin"
          smoke_binary="$(find target -path '*/release/phenix-acp-smoke' -type f -print -quit)"
          test -n "$smoke_binary"
          cp "$smoke_binary" "$out/bin/phenix-acp-smoke"
          runHook postInstall
        '';
      };

      supportedPhenix = self.packages.${system}.phenix;
      firstPartyPlugins = builtins.attrValues self.phenixPlugins.${system};
      phenixProductSmoke =
        pkgs.runCommand "phenix-product-smoke"
          {
            nativeBuildInputs = [
              phenixAcpSmoke
              supportedPhenix
              pkgs.jq
            ]
            ++ firstPartyPlugins;
          }
          ''
            PHENIX_HARNESS=${supportedPhenix}/bin/phenix-harness phenix-acp-smoke

            test -f ${supportedPhenix}/share/phenix/runtime.json
            test -f ${supportedPhenix}/share/phenix/skills/write/SKILL.md
            test -f ${supportedPhenix}/share/phenix/skills/pstack-LICENSE
            test -f ${supportedPhenix}/share/phenix/NOTICE.md

            export PHENIX_STATE_DB="$TMPDIR/harness.sqlite"
            phenix --list-services > "$TMPDIR/services.json"
            jq -e '
              (.plugins | length == 17)
              and ([.plugins[] | select(startswith("phenix.basic-"))] | length == 0)
              and (.services | index("phenix.sessions@1") != null)
              and (.services | index("phenix.context@1") != null)
              and (.services | index("phenix.execution@1") != null)
              and (.services | index("phenix.execution.configuration@1") != null)
              and (.services | index("phenix.options@1") != null)
              and (.services | index("phenix.sdk.config@1") != null)
              and (.services | index("phenix.repository.worker-queue@1") != null)
            ' "$TMPDIR/services.json" >/dev/null

            touch "$out"
          '';
    in
    {
      packages = {
        phenix-harness-runtime = phenixHarnessRuntime;
        phenix-harness-resources = phenixHarnessResources;
        phenix-acp-smoke = phenixAcpSmoke;
      };

      checks.phenix-product-smoke = phenixProductSmoke;
    };
}
