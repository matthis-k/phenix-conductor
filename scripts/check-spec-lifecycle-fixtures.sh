#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
validator="$repo_root/scripts/check-spec-lifecycle.sh"
fixture_dir="$(mktemp -d)"
trap 'rm -rf "$fixture_dir"' EXIT

expect_failure() {
  local name="$1"
  local expected="$2"
  local file="$fixture_dir/$name.md"
  shift 2
  printf '%s\n' "$@" > "$file"
  local output
  if output="$("$validator" "$file" 2>&1)"; then
    printf 'fixture unexpectedly passed: %s\n' "$name" >&2
    exit 1
  fi
  if [[ "$output" != *"$expected"* ]]; then
    printf 'fixture failed for wrong reason: %s\n%s\n' "$name" "$output" >&2
    exit 1
  fi
}

expect_failure \
  missing-status \
  'missing status metadata' \
  '# Missing status'

expect_failure \
  invalid-status \
  "invalid status 'complete'" \
  '# Invalid status' \
  'status: complete'

expect_failure \
  temporary-with-status \
  'temporary specs must not also declare lasting status' \
  '# Temporary with status' \
  'temporary: true' \
  'status: implemented'

expect_failure \
  enforced-without-coverage \
  'enforced specs require at least one coverage pointer' \
  '# Enforced without coverage' \
  'status: enforced'

expect_failure \
  stale-coverage \
  'stale coverage pointer: scripts/does-not-exist.sh' \
  '# Stale coverage' \
  'status: enforced' \
  'coverage:' \
  '  - scripts/does-not-exist.sh'

expect_failure \
  removed-only-coverage \
  'enforced specs require at least one coverage pointer' \
  '# Removed only coverage' \
  'status: enforced'
