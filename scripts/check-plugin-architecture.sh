#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root/rust"

metadata() {
  if [[ -n "${PHENIX_PLUGIN_ARCHITECTURE_METADATA_JSON:-}" ]]; then
    printf '%s\n' "$PHENIX_PLUGIN_ARCHITECTURE_METADATA_JSON"
    return
  fi
  cargo metadata --format-version 1 --no-deps --locked
}

errors="$({
  metadata |
    jq -r '
      .packages as $packages
      | ($packages | map({ key: .name, value: (.metadata.phenix.role // null) }) | from_entries) as $roles
      | (
          [
            $packages[]
            | select((.name == "phenix-plugin-cli") or (.name == "phenix-plugin-sdk"))
            | "legacy runtime plugin package is forbidden: \(.name)"
          ]
          +
          [
            $packages[]
            | select((.metadata.phenix.role // "") == "")
            | "missing package role: \(.name)"
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
            | select(($package.metadata.phenix.role // null) == "passive-library")
            | $package.dependencies[] as $dependency
            | select(($dependency.kind == null) or ($dependency.kind == "build"))
            | select($roles[$dependency.name] == "runtime-plugin")
            | "passive library cannot depend on runtime plugin: \($package.name) -> \($dependency.name)"
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
            | ($package.metadata.phenix["contract-debt"] // {}) as $debt
            | ($package.metadata.phenix["implementation-dependencies"] // {}) as $implementation
            | $debt
            | keys[] as $dependency
            | select($implementation | has($dependency))
            | "plugin dependency cannot be both contract debt and implementation sharing: \($package.name) -> \($dependency)"
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

if [[ -z "${PHENIX_PLUGIN_ARCHITECTURE_FIXTURE_MODE:-}" ]]; then
  fixture='{"packages":[{"name":"phenix-plugin-a","metadata":{"phenix":{"role":"runtime-plugin","contract-debt":{"phenix-plugin-b":"legacy contract"},"implementation-dependencies":{"phenix-plugin-b":"shared implementation"}}},"dependencies":[{"name":"phenix-plugin-b","kind":null}]},{"name":"phenix-plugin-b","metadata":{"phenix":{"role":"runtime-plugin"}},"dependencies":[]}]}'
  if fixture_errors="$(
    PHENIX_PLUGIN_ARCHITECTURE_METADATA_JSON="$fixture" \
      PHENIX_PLUGIN_ARCHITECTURE_FIXTURE_MODE=1 \
      bash "$repo_root/scripts/check-plugin-architecture.sh" 2>&1
  )"; then
    printf '%s\n' "plugin architecture fixture unexpectedly passed: overlapping dependency declarations" >&2
    exit 1
  fi

  expected="plugin dependency cannot be both contract debt and implementation sharing: phenix-plugin-a -> phenix-plugin-b"
  if [[ "$fixture_errors" != *"$expected"* ]]; then
    printf '%s\n' "plugin architecture fixture failed for the wrong reason" >&2
    printf '%s\n' "$fixture_errors" >&2
    exit 1
  fi

  fixture='{"packages":[{"name":"phenix-sdk","metadata":{"phenix":{"role":"passive-library"}},"dependencies":[{"name":"phenix-plugin-default","kind":null}]},{"name":"phenix-plugin-default","metadata":{"phenix":{"role":"runtime-plugin"}},"dependencies":[]}]}'
  if fixture_errors="$(
    PHENIX_PLUGIN_ARCHITECTURE_METADATA_JSON="$fixture" \
      PHENIX_PLUGIN_ARCHITECTURE_FIXTURE_MODE=1 \
      bash "$repo_root/scripts/check-plugin-architecture.sh" 2>&1
  )"; then
    printf '%s\n' "plugin architecture fixture unexpectedly passed: passive library runtime dependency" >&2
    exit 1
  fi

  expected="passive library cannot depend on runtime plugin: phenix-sdk -> phenix-plugin-default"
  if [[ "$fixture_errors" != *"$expected"* ]]; then
    printf '%s\n' "plugin architecture fixture failed for the wrong reason" >&2
    printf '%s\n' "$fixture_errors" >&2
    exit 1
  fi

  fixture='{"packages":[{"name":"phenix-unclassified","metadata":{},"dependencies":[]}]}'
  if fixture_errors="$(
    PHENIX_PLUGIN_ARCHITECTURE_METADATA_JSON="$fixture" \
      PHENIX_PLUGIN_ARCHITECTURE_FIXTURE_MODE=1 \
      bash "$repo_root/scripts/check-plugin-architecture.sh" 2>&1
  )"; then
    printf '%s\n' "plugin architecture fixture unexpectedly passed: unclassified workspace package" >&2
    exit 1
  fi

  expected="missing package role: phenix-unclassified"
  if [[ "$fixture_errors" != *"$expected"* ]]; then
    printf '%s\n' "plugin architecture fixture failed for the wrong reason" >&2
    printf '%s\n' "$fixture_errors" >&2
    exit 1
  fi

  fixture='{"packages":[{"name":"phenix-invalid-role","metadata":{"phenix":{"role":"service"}},"dependencies":[]}]}'
  if fixture_errors="$(
    PHENIX_PLUGIN_ARCHITECTURE_METADATA_JSON="$fixture" \
      PHENIX_PLUGIN_ARCHITECTURE_FIXTURE_MODE=1 \
      bash "$repo_root/scripts/check-plugin-architecture.sh" 2>&1
  )"; then
    printf '%s\n' "plugin architecture fixture unexpectedly passed: invalid workspace package role" >&2
    exit 1
  fi

  expected="invalid plugin package role: phenix-invalid-role = service"
  if [[ "$fixture_errors" != *"$expected"* ]]; then
    printf '%s\n' "plugin architecture fixture failed for the wrong reason" >&2
    printf '%s\n' "$fixture_errors" >&2
    exit 1
  fi

  fixture='{"packages":[{"name":"phenix-plugin-a","metadata":{"phenix":{"role":"runtime-plugin","implementation-dependencies":{"phenix-plugin-b":"shared implementation"}}},"dependencies":[]},{"name":"phenix-plugin-b","metadata":{"phenix":{"role":"runtime-plugin"}},"dependencies":[]}]}'
  if fixture_errors="$(
    PHENIX_PLUGIN_ARCHITECTURE_METADATA_JSON="$fixture" \
      PHENIX_PLUGIN_ARCHITECTURE_FIXTURE_MODE=1 \
      bash "$repo_root/scripts/check-plugin-architecture.sh" 2>&1
  )"; then
    printf '%s\n' "plugin architecture fixture unexpectedly passed: stale dependency declaration" >&2
    exit 1
  fi

  expected="stale plugin dependency declaration: phenix-plugin-a -> phenix-plugin-b"
  if [[ "$fixture_errors" != *"$expected"* ]]; then
    printf '%s\n' "plugin architecture fixture failed for the wrong reason" >&2
    printf '%s\n' "$fixture_errors" >&2
    exit 1
  fi

  fixture='{"packages":[{"name":"phenix-plugin-a","metadata":{"phenix":{"role":"runtime-plugin","implementation-dependencies":{"phenix-plugin-b":""}}},"dependencies":[{"name":"phenix-plugin-b","kind":null}]},{"name":"phenix-plugin-b","metadata":{"phenix":{"role":"runtime-plugin"}},"dependencies":[]}]}'
  if fixture_errors="$(
    PHENIX_PLUGIN_ARCHITECTURE_METADATA_JSON="$fixture" \
      PHENIX_PLUGIN_ARCHITECTURE_FIXTURE_MODE=1 \
      bash "$repo_root/scripts/check-plugin-architecture.sh" 2>&1
  )"; then
    printf '%s\n' "plugin architecture fixture unexpectedly passed: empty dependency reason" >&2
    exit 1
  fi

  expected="plugin dependency declaration needs a reason: phenix-plugin-a -> phenix-plugin-b"
  if [[ "$fixture_errors" != *"$expected"* ]]; then
    printf '%s\n' "plugin architecture fixture failed for the wrong reason" >&2
    printf '%s\n' "$fixture_errors" >&2
    exit 1
  fi

  fixture='{"packages":[{"name":"phenix-plugin-a","metadata":{"phenix":{"role":"runtime-plugin"}},"dependencies":[{"name":"phenix-plugin-b","kind":null}]},{"name":"phenix-plugin-b","metadata":{"phenix":{"role":"runtime-plugin"}},"dependencies":[]}]}'
  if fixture_errors="$(
    PHENIX_PLUGIN_ARCHITECTURE_METADATA_JSON="$fixture" \
      PHENIX_PLUGIN_ARCHITECTURE_FIXTURE_MODE=1 \
      bash "$repo_root/scripts/check-plugin-architecture.sh" 2>&1
  )"; then
    printf '%s\n' "plugin architecture fixture unexpectedly passed: undeclared runtime dependency" >&2
    exit 1
  fi

  expected="undeclared runtime plugin dependency: phenix-plugin-a -> phenix-plugin-b"
  if [[ "$fixture_errors" != *"$expected"* ]]; then
    printf '%s\n' "plugin architecture fixture failed for the wrong reason" >&2
    printf '%s\n' "$fixture_errors" >&2
    exit 1
  fi

  fixture='{"packages":[{"name":"phenix-plugin-cli","metadata":{"phenix":{"role":"runtime-plugin"}},"dependencies":[]}]}'
  if fixture_errors="$(
    PHENIX_PLUGIN_ARCHITECTURE_METADATA_JSON="$fixture" \
      PHENIX_PLUGIN_ARCHITECTURE_FIXTURE_MODE=1 \
      bash "$repo_root/scripts/check-plugin-architecture.sh" 2>&1
  )"; then
    printf '%s\n' "plugin architecture fixture unexpectedly passed: legacy command toolbelt package" >&2
    exit 1
  fi

  expected="legacy runtime plugin package is forbidden: phenix-plugin-cli"
  if [[ "$fixture_errors" != *"$expected"* ]]; then
    printf '%s\n' "plugin architecture fixture failed for the wrong reason" >&2
    printf '%s\n' "$fixture_errors" >&2
    exit 1
  fi
fi
