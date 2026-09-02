# Spec coverage lifecycle

status: partial
coverage:
  - scripts/check-spec-lifecycle.sh
  - scripts/check-spec-lifecycle-fixtures.sh

## Status

Normative repository process for connecting specifications to implementation and regression coverage.

## Goal

Distinguish current guarantees from future work without keeping repository history as architecture.

A specification may describe future behavior. It must not preserve a superseded internal contract, compatibility path, package identity, or migration state after the current architecture has converged.

## Metadata

Every lasting normative file under `spec/` declares a compact status block near the top.

Required field:

```yaml
status: specification-only | implemented | partial | enforced
```

Temporary implementation slices instead declare:

```yaml
temporary: true
```

When `status` is `implemented`, `partial`, or `enforced`, add repository-owned coverage pointers where real regression coverage exists:

```yaml
coverage:
  - rust/crates/phenix-core/src/example_regression.rs
  - scripts/check-example.sh
```

Coverage pointers name checks that fail when the corresponding requirement regresses. They do not restate expected values.

A specification may point to another normative specification when that file owns the rule. Use references instead of copying requirements between files.

## Meaning of status

`specification-only` means future behavior is defined but no implementation claim is made.

`implemented` means the specification describes the current implemented contract, but complete regression enforcement is not claimed.

`partial` means an additive feature or active implementation is incomplete. It does not make known contract duplication, backwards compatibility, migration debt, or old/new coexistence acceptable in a clean baseline.

`enforced` means the specification's completion criteria are covered by structural checks, behavioral tests, or both, and the exact-head validation required by that specification passed before the status change landed.

A change that removes the last effective regression for an enforced specification updates its status or replacement coverage in the same PR.

## Present-tense rule

The lasting spec tree describes either the current architecture or genuine future functionality.

Do not retain completed migration records as normative architecture. Delete temporary implementation slices, migration checklists, old-to-new mappings, compatibility plans, and debt inventories once their work is complete.

A future feature may stay `specification-only` indefinitely. That is backlog.

A migration of an existing contract or package model is different. While it is active, a temporary spec may describe the work. Completion requires one canonical current state and deletion of migration-only residue. Do not use `partial` as a permanent label for old and new internal contracts coexisting.

Mocks and placeholders do not make a spec partial by themselves. A mock that satisfies the canonical contract is a valid implementation.

## Source validation

Add a deterministic Source check for the lasting normative spec tree.

The check must:

1. require valid status metadata on normative specs;
2. require every declared coverage path to exist;
3. reject `enforced` when no coverage pointer exists;
4. reject a file that declares both temporary migration state and lasting status;
5. report specification paths and broken coverage pointers directly.

The check must not infer semantic completeness from file names or grep implementation text. Runtime semantics remain proven by Rust or Product tests.

Semantic ownership and migration completion remain explicit repository decisions. Source validates their lifecycle representation; it does not infer those decisions from prose.

Temporary migration specs may carry explicit temporary metadata while active. They must be deleted at migration completion rather than promoted into permanent historical records.

`scripts/check-spec-lifecycle.sh` is the canonical validator. During initial classification it may be run against explicit paths. After every lasting normative spec is classified, wire its default whole-tree mode into the Source validation order so new unclassified specs fail CI.

The validator only checks metadata shape and coverage-path existence. It does not claim semantic completeness from metadata.

## Migration

Introduce metadata without weakening Source validation:

1. classify the current lasting normative spec set;
2. add real coverage pointers where checks already exist;
3. distinguish genuine future work from active migrations of existing architecture;
4. enable the Source gate after the initial classification is complete;
5. thereafter require metadata for every new lasting normative spec;
6. delete temporary migration specs as their migrations converge.

Do not mark a spec `enforced` merely because an implementation PR exists or CI is green. The referenced checks must prove its completion criteria.

## Validation

Add negative fixtures that prove Source rejects:

- missing or invalid status;
- an `enforced` spec without coverage;
- a stale coverage path;
- removal of the only declared coverage for an enforced spec.

Add repository cleanup coverage that prevents completed temporary migration specifications from being retained as canonical architecture when their completion condition says to delete them.

The final exact head must pass Source, Rust, Product, and Maintenance validation.

## Completion

This slice is complete when the repository can distinguish current guarantees from future backlog, enforced specs cannot silently lose regression coverage, and completed migrations leave no permanent historical contract layer in the normative spec tree.