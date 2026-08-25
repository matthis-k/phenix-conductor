{ lib, ... }:
{
  perSystem =
    { pkgs, ... }:
    let
      transport = pkgs.writeShellApplication {
        name = "phenix-maintenance-split-transport";
        runtimeInputs = [
          pkgs.cargo
          pkgs.findutils
          pkgs.git
          pkgs.nixfmt
          pkgs.python3
          pkgs.rustfmt
          pkgs.statix
        ];
        text = ''
          set -euo pipefail

          if [[ "''${1:-}" != "fix" ]]; then
            echo "split transport only supports the maintenance fix command" >&2
            exit 2
          fi

          repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
          cd "$repo_root"

          python3 scripts/split_conductor_modules.py

          git fetch origin main
          git checkout origin/main -- .github/workflows/sync-maintenance.yml flake.nix
          rm -f modules/split-transport.nix
          rm -f scripts/split_conductor_modules.py
          rm -f scripts/.worker-395-transport

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
    in
    {
      apps.phenix-maintenance = lib.mkForce {
        type = "app";
        program = "${transport}/bin/phenix-maintenance-split-transport";
      };
    };
}
