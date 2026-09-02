---
temporary: true
---

# Architecture closure

## Goal

Finish the post-migration audit so every baseline kernel/plugin architecture specification describes current implemented behavior with concrete regression coverage, while unfinished additive product features remain clearly `specification-only`.

## Required audit

Review at least:

- `plugin-authoring-macro.md`;
- `plugin-contributions.md`;
- `plugin-resolution.md`;
- `plugin-host.md`;
- `plugin-runtime-bridges.md`;
- `plugin-persistence.md`;
- `plugin-events.md`;
- `plugin-threading.md`;
- `plugin-architecture-enforcement.md`;
- `typed-structural-boundaries.md`.

For each completion criterion, either:

1. point to a repository-owned structural or behavioral regression that proves it; or
2. implement the missing baseline behavior and add coverage; or
3. move a genuinely additive future feature into a narrowly owned `specification-only` contract.

Do not leave a broad architecture document `partial` or `specification-only` merely because lifecycle metadata was not reconciled after implementation.

## Enforcement

- Extend Source checks only for mechanically derivable repository structure.
- Keep runtime semantics in Rust or Product tests.
- Reject reintroduction of obsolete package identities, manual static plugin wiring, closed runtime enums, parallel registries, raw JSON structural boundaries, and other already-converged architecture debt.
- Keep one authoritative owner for every requirement and use coverage pointers instead of copied expected values.

## Completion

- no unfinished specification describes how the existing kernel/plugin architecture is supposed to work;
- remaining `specification-only` files describe additive future functionality only;
- no broad architecture spec remains `partial`;
- every `enforced` spec has concrete coverage pointers;
- Source, Rust, Product, Docs, and Maintenance pass on the exact head;
- delete this temporary implementation slice before merge.
