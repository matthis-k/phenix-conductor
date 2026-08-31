# Spec coverage lifecycle

## Status

Normative repository process for connecting specifications to implementation and regression coverage.

## Goal

Make unfinished normative work visible without turning prose into a second implementation registry.

A specification may describe future behavior. The repository must also expose whether that behavior is specification-only, partially implemented, or enforced.

## Metadata

Every normative file under `spec/` declares a compact status block near the top.

Required fields:

```yaml
status: specification-only | partial | enforced
```

When `status` is `partial` or `enforced`, add one or more repository-owned coverage pointers:

```yaml
coverage:
  - rust/crates/phenix-core/src/example_regression.rs
  - scripts/check-example.sh
```

Coverage pointers name the checks that fail when the corresponding requirement regresses. They do not restate expected values.

A specification may also point to another normative specification when that file owns the rule. Use references instead of copying requirements between files.

## Source validation

Add a deterministic Source check for the spec tree.

The check must:

1. require valid status metadata on normative specs;
2. require every declared coverage path to exist;
3. reject `enforced` when no coverage pointer exists;
4. reject duplicate complete ownership of one rule when a spec explicitly delegates that rule to another normative file;
5. report specification paths and broken coverage pointers directly.

The check must not infer semantic completeness from file names or grep implementation text. Runtime semantics remain proven by Rust or Product tests.

## Lifecycle

`specification-only` means no implementation claim.

`partial` means some requirements have implementation or regression coverage, but the completion criteria are not all proven.

`enforced` means the specification's completion criteria are covered by structural checks, behavioral tests, or both, and the exact-head validation required by that specification has passed before the status change lands.

A change that removes the last effective regression for an enforced specification must update its status or replacement coverage in the same PR.

Temporary implementation checklists should be deleted when completed rather than kept as historical status records.

## Migration

Introduce metadata incrementally without weakening Source validation:

1. classify the current normative spec set;
2. add real coverage pointers where checks already exist;
3. mark known incomplete architecture work as `partial` or `specification-only`;
4. enable the Source gate only after the initial classification is complete;
5. thereafter require metadata for every new normative spec.

Do not mark a spec `enforced` merely because an implementation PR exists or because CI is green. The referenced checks must prove its completion criteria.

## Validation

Add negative fixtures that prove Source rejects:

- missing or invalid status;
- an `enforced` spec without coverage;
- a stale coverage path;
- removal of the only declared coverage for an enforced spec.

The final exact head must pass Source, Rust, Product, and Maintenance validation.

## Completion

This slice is complete when the repository can answer which normative specs are unfinished from checked metadata, and an enforced spec cannot silently lose its regression coverage.
