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

      supportedPhenix = self.packages.${system}.phenix;
      firstPartyPlugins = builtins.attrValues self.phenixPlugins.${system};
      phenixProductSmoke =
        pkgs.runCommand "phenix-product-smoke"
          {
            nativeBuildInputs = [
              supportedPhenix
              pkgs.jq
            ]
            ++ firstPartyPlugins;
          }
          ''
            export PHENIX_STATE_DB="$TMPDIR/acp-smoke.sqlite"
            printf '%s\n' \
              '{"id":1,"service":"phenix.sessions@1","input":{"type":"variant","value":{"tag":"Create","value":{"type":"table","value":{"id":{"type":"string","value":"acp-smoke"}}}}}}' \
              '{"id":2,"service":"phenix.sessions@1","input":{"type":"variant","value":{"tag":"Get","value":{"type":"table","value":{"id":{"type":"string","value":"acp-smoke"}}}}}}' \
              | ${supportedPhenix}/bin/phenix-harness > "$TMPDIR/acp-smoke.jsonl"
            jq -se '
              length == 2
              and .[0].id == 1
              and .[0].status == "ok"
              and .[0].output.type == "variant"
              and .[0].output.value.tag == "Created"
              and .[0].output.value.value.type == "table"
              and .[0].output.value.value.value.session.type == "table"
              and .[0].output.value.value.value.session.value.id.type == "string"
              and .[0].output.value.value.value.session.value.id.value == "acp-smoke"
              and .[1].id == 2
              and .[1].status == "ok"
              and .[1].output.type == "variant"
              and .[1].output.value.tag == "Session"
              and .[1].output.value.value.type == "table"
              and .[1].output.value.value.value.session.type == "option"
              and .[1].output.value.value.value.session.value.type == "table"
              and .[1].output.value.value.value.session.value.value.id.type == "string"
              and .[1].output.value.value.value.session.value.value.id.value == "acp-smoke"
            ' "$TMPDIR/acp-smoke.jsonl" >/dev/null

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
      };

      checks.phenix-product-smoke = phenixProductSmoke;
    };
}
