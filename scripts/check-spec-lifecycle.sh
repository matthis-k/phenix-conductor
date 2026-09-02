#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

failures=()

fail() { failures+=("$1"); }

check_file() {
  local file="$1"
  local metadata status temporary coverage_path
  local -a coverage=()
  metadata="$(sed -n '1,24p' "$file")"
  status="$(printf '%s\n' "$metadata" | sed -n 's/^status:[[:space:]]*//p')"
  temporary="$(printf '%s\n' "$metadata" | sed -n 's/^temporary:[[:space:]]*//p')"
  if [[ "$temporary" == "true" ]]; then
    if [[ -n "$status" ]]; then fail "$file: temporary specs must not also declare lasting status"; fi
    return
  fi
  case "$status" in
    specification-only|partial|enforced) ;;
    '') fail "$file: missing status metadata in first 24 lines"; return ;;
    *) fail "$file: invalid status '$status'"; return ;;
  esac
  while IFS= read -r coverage_path; do
    [[ -n "$coverage_path" ]] && coverage+=("$coverage_path")
  done < <(awk '/^coverage:[[:space:]]*$/ { in_coverage = 1; next } in_coverage && /^  - / { sub(/^  - /, ""); print; next } in_coverage { exit }' <<<"$metadata")
  if [[ "$status" == "enforced" && "${#coverage[@]}" -eq 0 ]]; then fail "$file: enforced specs require at least one coverage pointer"; fi
  for coverage_path in "${coverage[@]}"; do
    if [[ "$coverage_path" = /* || "$coverage_path" == *'..'* ]]; then fail "$file: coverage pointer must be a repository-relative path: $coverage_path"; continue; fi
    if [[ ! -e "$coverage_path" ]]; then fail "$file: stale coverage pointer: $coverage_path"; fi
  done
}
if (( $# > 0 )); then
  for file in "$@"; do check_file "$file"; done
else
  while IFS= read -r -d '' file; do check_file "$file"; done < <(find spec -maxdepth 1 -type f -name '*.md' -print0 | sort -z)
fi
if (( ${#failures[@]} > 0 )); then
  printf 'Spec lifecycle failures:\n' >&2
  printf '  - %s\n' "${failures[@]}" >&2
  exit 1
fi
printf 'Spec lifecycle metadata is valid.\n'
