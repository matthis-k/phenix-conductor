# Specification lifecycle

`spec/catalog.toml` is the source of truth for specification state and enforcement.

Each specification has one lifecycle state:

- `target`: accepted design that is not yet fully implemented;
- `implemented`: current behavior that must stay covered by automated checks;
- `transitional`: current design or implementation is being migrated and the catalog note names the gap;
- `superseded`: retained only for historical context and not normative.

`proposed` is not used for accepted repository contracts. A design that is still exploratory belongs in a pull request until accepted or is cataloged as `target` only when the repository intends to implement it.

Enforcement is separate from lifecycle:

- `none`: no repository check proves the contract yet;
- `partial`: implementation or regressions cover only part of the contract;
- `strong`: focused tests, source checks, or type boundaries cover the material invariants.

An `implemented` spec must name at least one `enforced_by` path. `strong` means the listed checks cover the contract's material invariants, not merely that related code exists.

## Precedence

When specifications overlap, the catalog decides which document is current. `target` and `implemented` documents may refine older `transitional` documents. A `superseded` document is never a source of current requirements.

A specification must not rely on a missing specification. Cross-spec requirements use repository paths that exist in the same revision.

## Updating a specification

A change that alters architecture or product behavior must update the affected catalog entry in the same pull request. When implementation catches up, move the entry from `target` or `transitional` to `implemented` and name the focused enforcement paths.

Do not mark a spec `implemented` because CI is green. CI must exercise the contract itself.

## Repository checks

The enforcement work following this specification update must check:

1. every `spec/*.md` file except this index has exactly one catalog entry;
2. every catalog path exists and duplicate paths fail;
3. lifecycle and enforcement values are from the closed sets above;
4. `implemented` entries have non-empty `enforced_by` paths and every path exists;
5. `strong` entries have non-empty `enforced_by` paths;
6. literal `spec/*.md` references in active specifications resolve;
7. `superseded` specifications cannot be listed as prerequisites of active specifications.
