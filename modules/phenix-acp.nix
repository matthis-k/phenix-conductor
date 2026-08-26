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
