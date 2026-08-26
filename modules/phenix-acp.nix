_: {
  perSystem =
    { pkgs, ... }:
    let
      rustSource = pkgs.lib.cleanSource ../rust;

      phenixKernel = pkgs.rustPlatform.buildRustPackage {
        pname = "phenix-kernel";
        version = "0";
        src = rustSource;

        cargoLock.lockFile = ../rust/Cargo.lock;
        cargoBuildFlags = [
          "--package"
          "phenix-kernel"
          "--bin"
          "phenix-kernel"
        ];
        doCheck = false;

        installPhase = ''
          runHook preInstall
          mkdir -p "$out/bin"
          kernel_binary="$(find target -path '*/release/phenix-kernel' -type f -print -quit)"
          test -n "$kernel_binary"
          cp "$kernel_binary" "$out/bin/phenix-kernel"
          runHook postInstall
        '';
      };

      phenixKernelSmoke =
        pkgs.runCommand "phenix-kernel-smoke" { nativeBuildInputs = [ phenixKernel ]; }
          ''
            phenix-kernel
            touch "$out"
          '';

      phenixHarness = pkgs.rustPlatform.buildRustPackage {
        pname = "phenix-harness";
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

      phenixProductSmoke =
        pkgs.runCommand "phenix-product-smoke"
          {
            nativeBuildInputs = [
              phenixAcpSmoke
              phenixHarness
              pkgs.jq
            ];
          }
          ''
            phenix-acp-smoke

            export PHENIX_STATE_DB="$TMPDIR/harness.sqlite"
            phenix-harness --list-services > "$TMPDIR/services.json"
            jq -e '
              (.plugins | length == 14)
              and (.services | index("phenix.sessions@1") != null)
              and (.services | index("phenix.context@1") != null)
              and (.services | index("phenix.execution@1") != null)
              and (.services | index("phenix.repository.worker-queue@1") != null)
            ' "$TMPDIR/services.json" >/dev/null

            printf '%s\n' '{"id":1,"service":"phenix.sessions@1","input":{"operation":"create","id":"product-smoke","parent":null}}' \
              | phenix-harness > "$TMPDIR/create.json"
            cat "$TMPDIR/create.json"
            jq -e '
              .id == 1
              and .status == "ok"
              and .output.result == "created"
              and .output.session.id == "product-smoke"
            ' "$TMPDIR/create.json" >/dev/null

            printf '%s\n' '{"id":2,"service":"phenix.sessions@1","input":{"operation":"get","id":"product-smoke"}}' \
              | phenix-harness > "$TMPDIR/restore.json"
            cat "$TMPDIR/restore.json"
            jq -e '
              .id == 2
              and .status == "ok"
              and .output.result == "session"
              and .output.session.id == "product-smoke"
            ' "$TMPDIR/restore.json" >/dev/null

            printf '%s\n' '{"id":3,"service":"phenix.artifacts@1","input":{"operation":"store","content":[112,114,111,100,117,99,116],"provenance":{"producer":"product-smoke","provider_identity":null,"configuration_identity":null,"source_observations":{}}}}' \
              | phenix-harness > "$TMPDIR/artifact.json"
            jq -e '.id == 3 and .status == "ok" and .output.response == "stored" and .output.reused == false' "$TMPDIR/artifact.json" >/dev/null

            printf '%s\n' '{"id":4,"service":"phenix.context@1","input":{"operation":"register","resource_id":"product:context","kind":"external","source":"product-smoke","scope":"workspace","content":[99,111,110,116,101,120,116]}}' \
              | phenix-harness > "$TMPDIR/context.json"
            jq -e '.id == 4 and .status == "ok" and .output.result == "registered" and .output.resource.descriptor.resource_id == "product:context"' "$TMPDIR/context.json" >/dev/null

            printf '%s\n' '{"id":5,"service":"phenix.execution@1","input":{"operation":"runnable_tasks"}}' \
              | phenix-harness > "$TMPDIR/execution.json"
            jq -e '.id == 5 and .status == "ok" and .output.response == "runnable_tasks"' "$TMPDIR/execution.json" >/dev/null

            printf '%s\n' '{"id":6,"service":"phenix.planning@1","input":{"operation":"create_objective","id":"product-objective","title":"Harness product parity","parent":null}}' \
              | phenix-harness > "$TMPDIR/planning.json"
            jq -e '.id == 6 and .status == "ok" and .output.response == "objective" and .output.objective.id == "product-objective"' "$TMPDIR/planning.json" >/dev/null

            printf '%s\n' '{"id":7,"service":"phenix.language@1","input":{"operation":"current_diagnostics","workspace_id":"product-workspace"}}' \
              | phenix-harness > "$TMPDIR/language.json"
            jq -e '.id == 7 and .status == "ok" and .output.response == "diagnostics" and .output.result == null' "$TMPDIR/language.json" >/dev/null

            printf '%s\n' '{"id":8,"service":"phenix.models.routing@1","input":{"operation":"list_profiles"}}' \
              | phenix-harness > "$TMPDIR/models.json"
            jq -e '.id == 8 and .status == "ok" and .output.kind == "profiles"' "$TMPDIR/models.json" >/dev/null

            printf '%s\n' '{"id":9,"service":"phenix.jobs@1","input":{"operation":"list"}}' \
              | phenix-harness > "$TMPDIR/jobs.json"
            jq -e '.id == 9 and .status == "ok" and .output.response == "resources"' "$TMPDIR/jobs.json" >/dev/null

            printf '%s\n' '{"id":10,"service":"phenix.frontend-services@1","input":{"operation":"catalog"}}' \
              | phenix-harness > "$TMPDIR/frontend.json"
            jq -e '.id == 10 and .status == "ok" and .output.response == "providers"' "$TMPDIR/frontend.json" >/dev/null

            printf '%s\n' '{"id":11,"service":"phenix.hooks@1","input":{"operation":"get_configuration","revision":"missing"}}' \
              | phenix-harness > "$TMPDIR/hooks.json"
            jq -e '.id == 11 and .status == "ok" and .output.response == "configuration" and .output.configuration == null' "$TMPDIR/hooks.json" >/dev/null

            printf '%s\n' '{"id":12,"service":"phenix.debug@1","input":{"operation":"snapshot"}}' \
              | phenix-harness > "$TMPDIR/debug.json"
            jq -e '.id == 12 and .status == "ok" and .output.response == "snapshot" and (.output.snapshot.services | to_entries | all(.value.available == true))' "$TMPDIR/debug.json" >/dev/null

            printf '%s\n' '{"id":13,"service":"phenix.workspace@1","input":{"operation":"shell","command":"printf harness-workspace"}}' \
              | phenix-harness > "$TMPDIR/workspace.json"
            jq -e '.id == 13 and .status == "ok" and .output.response == "process" and .output.exit_code == 0 and .output.stdout == "harness-workspace"' "$TMPDIR/workspace.json" >/dev/null

            printf '%s\n' '{"id":14,"service":"phenix.cli.discover@1","input":{"name":"jq"}}' \
              | phenix-harness > "$TMPDIR/cli.json"
            jq -e '.id == 14 and .status == "ok" and .output.name == "jq" and (.output.availability == "available" or .output.availability == "limited")' "$TMPDIR/cli.json" >/dev/null

            touch "$out"
          '';
    in
    {
      packages = {
        phenix-kernel = phenixKernel;
        phenix-harness = phenixHarness;
        phenix = phenixHarness;
        phenix-acp-smoke = phenixAcpSmoke;
        default = phenixHarness;
      };

      apps = {
        phenix-kernel.program = "${phenixKernel}/bin/phenix-kernel";
        phenix-harness.program = "${phenixHarness}/bin/phenix-harness";
        phenix.program = "${phenixHarness}/bin/phenix";
        default.program = "${phenixHarness}/bin/phenix";
      };

      checks = {
        phenix-kernel-smoke = phenixKernelSmoke;
        phenix-product-smoke = phenixProductSmoke;
      };
    };
}
