#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root/rust"

errors="$({
  cargo metadata --format-version 1 --no-deps --locked |
    jq -r '
      .packages as $packages
      | ($packages | map({ key: .name, value: (.metadata.phenix.role // null) }) | from_entries) as $roles
      | (
          [
            $packages[]
            | select(.name == "phenix-plugin-cli")
            | "legacy runtime plugin package is forbidden: \(.name)"
          ]
          +
          [
            $packages[]
            | select(.name | startswith("phenix-plugin-"))
            | select((.metadata.phenix.role // "") == "")
            | "missing plugin package role: \(.name)"
          ]
          +
          [
            $packages[] as $package
            | ($package.metadata.phenix.role // null) as $role
            | select($role != null)
            | select((["runtime-plugin", "passive-library", "application", "assembly", "test-support"] | index($role)) == null)
            | "invalid plugin package role: \($package.name) = \($role)"
          ]
          +
          [
            $packages[] as $package
            | select(($package.metadata.phenix.role // null) == "runtime-plugin")
            | ($package.metadata.phenix["contract-debt"] // {}) as $debt
            | ($package.metadata.phenix["implementation-dependencies"] // {}) as $implementation
            | $package.dependencies[] as $dependency
            | select(($dependency.kind == null) or ($dependency.kind == "build"))
            | select($roles[$dependency.name] == "runtime-plugin")
            | select(((($debt | has($dependency.name)) or ($implementation | has($dependency.name)))) | not)
            | "undeclared runtime plugin dependency: \($package.name) -> \($dependency.name)"
          ]
          +
          [
            $packages[] as $package
            | (($package.metadata.phenix["contract-debt"] // {}) + ($package.metadata.phenix["implementation-dependencies"] // {}))
            | to_entries[] as $declaration
            | select(
                [
                  $package.dependencies[]
                  | select((.kind == null) or (.kind == "build"))
                  | select(.name == $declaration.key)
                ]
                | length == 0
              )
            | "stale plugin dependency declaration: \($package.name) -> \($declaration.key)"
          ]
          +
          [
            $packages[] as $package
            | (($package.metadata.phenix["contract-debt"] // {}) + ($package.metadata.phenix["implementation-dependencies"] // {}))
            | to_entries[] as $declaration
            | select((($declaration.value | type) != "string") or (($declaration.value | length) == 0))
            | "plugin dependency declaration needs a reason: \($package.name) -> \($declaration.key)"
          ]
        )
      | .[]
    '
} 2>&1)" || {
  printf '%s\n' "$errors" >&2
  exit 1
}

if [[ -n "$errors" ]]; then
  printf '%s\n' "$errors" >&2
  exit 1
fi
