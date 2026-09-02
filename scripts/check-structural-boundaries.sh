#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

pattern='std::any::Any|std::any::\{[^}]*Any|any::Any|dyn[[:space:]]+Any'
json_field_pattern='pub[[:space:]]+[[:alnum:]_]+[[:space:]]*:[^,]*serde_json::Value'
raw_invoke_pattern='\.invoke_value[[:space:]]*\('

check_input() {
  if rg -n --pcre2 "$pattern"; then
    printf '%s\n' "public Phenix structural boundaries must not depend on std::any::Any" >&2
    return 1
  fi
}

if [[ -n "${PHENIX_STRUCTURAL_BOUNDARY_FIXTURE:-}" ]]; then
  printf '%s\n' "$PHENIX_STRUCTURAL_BOUNDARY_FIXTURE" | check_input
  exit
fi

check_json_fields() {
  if rg -n --pcre2 "$json_field_pattern"; then
    printf '%s\n' "neutral Phenix contracts must use PhenixValue instead of serde_json::Value" >&2
    return 1
  fi
}

if [[ -n "${PHENIX_JSON_BOUNDARY_FIXTURE:-}" ]]; then
  printf '%s\n' "$PHENIX_JSON_BOUNDARY_FIXTURE" | check_json_fields
  exit
fi

check_raw_invoke() {
  if rg -n --pcre2 "$raw_invoke_pattern"; then
    printf '%s\n' "statically known plugin consumers must use typed invocation helpers" >&2
    return 1
  fi
}

if [[ -n "${PHENIX_RAW_INVOKE_FIXTURE:-}" ]]; then
  printf '%s\n' "$PHENIX_RAW_INVOKE_FIXTURE" | check_raw_invoke
  exit
fi

targets=(
  rust/crates/phenix-core/src/contract.rs
  rust/crates/phenix-core/src/plugin.rs
  rust/crates/phenix-core/src/plugin_context.rs
  rust/crates/phenix-sdk/src
)

for crate in rust/crates/phenix-plugin-*; do
  [[ -d "$crate/src" ]] || continue
  targets+=("$crate/src")
done

rg -n --pcre2 "$pattern" "${targets[@]}" && {
  printf '%s\n' "public Phenix structural boundaries must not depend on std::any::Any" >&2
  exit 1
}

json_targets=(
  rust/crates/phenix-core/src/agent.rs
  rust/crates/phenix-core/src/configuration.rs
  rust/crates/phenix-core/src/manifest.rs
  rust/crates/phenix-domain/src/lib.rs
  rust/crates/phenix-domain/src/debug.rs
  rust/crates/phenix-sdk/src/contracts/frontend.rs
  rust/crates/phenix-sdk/src/contracts/models.rs
)

rg -n --pcre2 "$json_field_pattern" "${json_targets[@]}" && {
  printf '%s\n' "neutral Phenix contracts must use PhenixValue instead of serde_json::Value" >&2
  exit 1
}

for crate in rust/crates/phenix-plugin-*; do
  [[ -d "$crate/src" ]] || continue
  [[ "$crate" == "rust/crates/phenix-plugin-debug" ]] && continue
  if rg -n --pcre2 "$raw_invoke_pattern" "$crate/src"; then
    printf '%s\n' "statically known plugin consumers must use typed invocation helpers" >&2
    exit 1
  fi
done

fixture='pub type ErasedBoundary = Box<dyn std::any::Any>;'
if PHENIX_STRUCTURAL_BOUNDARY_FIXTURE="$fixture" \
  bash "$repo_root/scripts/check-structural-boundaries.sh" >/dev/null 2>&1; then
  printf '%s\n' "structural boundary fixture unexpectedly accepted std::any::Any" >&2
  exit 1
fi

json_fixture='pub struct NeutralBoundary { pub payload: serde_json::Value }'
if PHENIX_JSON_BOUNDARY_FIXTURE="$json_fixture" \
  bash "$repo_root/scripts/check-structural-boundaries.sh" >/dev/null 2>&1; then
  printf '%s\n' "structural boundary fixture unexpectedly accepted serde_json::Value" >&2
  exit 1
fi

raw_invoke_fixture='context.sdk.models.invoke_value(&request);'
if PHENIX_RAW_INVOKE_FIXTURE="$raw_invoke_fixture" \
  bash "$repo_root/scripts/check-structural-boundaries.sh" >/dev/null 2>&1; then
  printf '%s\n' "structural boundary fixture unexpectedly accepted raw static invocation" >&2
  exit 1
fi

exit 0
