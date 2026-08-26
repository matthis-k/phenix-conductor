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

      phenixConductor = pkgs.rustPlatform.buildRustPackage {
        pname = "phenix-conductor";
        version = "0";
        src = rustSource;

        cargoLock.lockFile = ../rust/Cargo.lock;
        cargoBuildFlags = [
          "--package"
          "phenix-conductor"
          "--bin"
          "phenix-conductor"
        ];
        nativeBuildInputs = [
          pkgs.cmake
          pkgs.makeWrapper
        ];
        doCheck = false;

        installPhase = ''
          runHook preInstall
          mkdir -p "$out/bin" "$out/libexec"
          conductor_binary="$(find target -path '*/release/phenix-conductor' -type f -print -quit)"
          test -n "$conductor_binary"
          cp "$conductor_binary" "$out/libexec/phenix-conductor"
          makeWrapper "$out/libexec/phenix-conductor" "$out/bin/phenix-conductor" \
            --set PHENIX_BASH "${pkgs.bash}/bin/bash" \
            --set PHENIX_BWRAP "${pkgs.bubblewrap}/bin/bwrap" \
            --set PHENIX_MKDIR "${pkgs.coreutils}/bin/mkdir" \
            --set PHENIX_RG "${pkgs.ripgrep}/bin/rg" \
            --set PHENIX_RM "${pkgs.coreutils}/bin/rm" \
            --set PHENIX_RSYNC "${pkgs.rsync}/bin/rsync" \
            --set PHENIX_SLIRP4NETNS "${pkgs.slirp4netns}/bin/slirp4netns" \
            --prefix PATH : "${
              pkgs.lib.makeBinPath [
                pkgs.coreutils
                pkgs.iproute2
                pkgs.util-linux
              ]
            }" \
            --set SSL_CERT_FILE "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt" \
            --set NIX_SSL_CERT_FILE "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
          runHook postInstall
        '';
      };

      # Until the first-party suite has moved behind kernel plugin contracts, the
      # supported Harness package remains the existing product binary. The package
      # name is stable now; the implementation behind it changes in this PR.
      phenixHarness = phenixConductor;

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
              pkgs.gnugrep
              pkgs.jq
            ];
          }
          ''
            phenix-acp-smoke

            conductor="${phenixHarness}/bin/phenix-conductor"
            "$conductor" --help > "$TMPDIR/conductor-help.txt"
            grep -F -- '--acp-command' "$TMPDIR/conductor-help.txt" >/dev/null

            export PHENIX_CREDENTIAL_FILE="$TMPDIR/credentials.json"
            export PHENIX_MODEL="openai-codex/product-smoke-model"
            export XDG_STATE_HOME="$TMPDIR/state"
            response="$TMPDIR/conductor.jsonl"
            {
              printf '%s\n' '{"id":1,"command":{"type":"initialize","after_sequence":null}}'
              printf '%s\n' '{"id":2,"command":{"type":"create_session","parent_session":null,"name":"product smoke","target":{"kind":"fixed","value":{"backend":"phenix","provider":"openai-codex","model":"product-smoke-model","inference":{}}}}}'
              printf '%s\n' '{"id":3,"command":{"type":"get_callable_catalog"}}'
            } | "$conductor" > "$response"

            jq -s -e '
              ([
                .[]
                | select(
                    .type == "response"
                    and .id == 1
                    and .status == "ok"
                    and .result.type == "initialized"
                  )
                | .result.backends[]
                | select(.backend == "phenix")
                | .models[]
                | select(
                    .target.backend == "phenix"
                    and .target.provider == "openai-codex"
                    and .target.model == "product-smoke-model"
                  )
              ] | length == 1)
              and ([
                .[]
                | select(
                    .type == "response"
                    and .id == 2
                    and .status == "ok"
                    and .result.type == "session"
                  )
                | .result.session
                | select(
                    .default_target.kind == "fixed"
                    and .default_target.value.backend == "phenix"
                    and .default_target.value.provider == "openai-codex"
                    and .default_target.value.model == "product-smoke-model"
                    and .default_target.value.inference.effort == null
                  )
              ] | length == 1)
              and ([
                .[]
                | select(
                    .type == "response"
                    and .id == 3
                    and .status == "ok"
                    and .result.type == "callable_catalog"
                  )
                | .result.callables[]
                | select(.kind == "tool" and (.id == "bash" or .id == "edit" or .id == "grep" or .id == "read" or .id == "write"))
                | .id
              ] | sort == ["bash", "edit", "grep", "read", "write"])
            ' "$response" >/dev/null

            touch "$out"
          '';
    in
    {
      packages = {
        phenix-kernel = phenixKernel;
        phenix-conductor = phenixConductor;
        phenix-harness = phenixHarness;
        phenix = phenixHarness;
        phenix-acp-smoke = phenixAcpSmoke;
        default = phenixHarness;
      };

      apps = {
        phenix-kernel.program = "${phenixKernel}/bin/phenix-kernel";
        phenix-conductor.program = "${phenixConductor}/bin/phenix-conductor";
        phenix-harness.program = "${phenixHarness}/bin/phenix-conductor";
        phenix.program = "${phenixHarness}/bin/phenix-conductor";
        default.program = "${phenixHarness}/bin/phenix-conductor";
      };

      checks = {
        phenix-kernel-smoke = phenixKernelSmoke;
        phenix-product-smoke = phenixProductSmoke;
      };
    };
}
