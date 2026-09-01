# Plugin role enforcement completion

## Status

Temporary implementation slice for `plugin-architecture-enforcement.md`. Delete this file when the migration is complete.

## Goal

Finish the ownership migration so the repository contains one present-tense contract and package model. The final tree must not retain structures whose only purpose is describing, accepting, or adapting the previous architecture.

## Required changes

1. Classify every first-party workspace package that participates in composition, distribution, application integration, SDK use, bindings, transports, catalogs, presets, or test support.
2. Make Source validation require a valid role for that full set. Do not infer the role from a `phenix-plugin-*` prefix.
3. Enforce allowed dependency directions between roles. A `passive-library` must not depend on a `runtime-plugin` for contracts, request or response types, interfaces, schemas, manifests, default handles, or authoring helpers.
4. Move every shared runtime contract to its canonical neutral owner. Consumers must compile against that owner rather than a default implementation crate.
5. Remove every `contract-debt` edge. Do not preserve a known contract violation as accepted metadata.
6. After the last debt edge is removed, delete the `contract-debt` metadata path and migration-only checker logic unless it still enforces a present-tense invariant.
7. Keep actual implementation reuse explicit through `implementation-dependencies`. This is for deliberate current implementation sharing, not compatibility or migration debt.
8. Reject one dependency edge appearing in both migration and implementation classifications while migration metadata still exists.
9. Remove legacy package names, plugin IDs, component IDs, service namespaces, package-set entries, compatibility aliases, and translation layers that exist only for the previous internal architecture.
10. Keep Cargo metadata as the package-role source of truth. Do not add a second complete package-role registry.

## Canonical contract rule

Each shared concept has one authoritative contract definition.

A migration is incomplete while any of these remain:

- old and new contract definitions coexist;
- a compatibility re-export keeps an old contract path alive;
- a consumer imports a default implementation crate to name a shared contract;
- an adapter translates only between two Phenix-internal generations of the same contract;
- migration metadata permits an otherwise forbidden dependency;
- a deprecated alias remains for callers that have not moved.

This is a prerelease repository. Change consumers with the contract instead of preserving backwards compatibility.

Mocks and placeholders are allowed. They must implement the same canonical contracts as real implementations and must not require mock-specific consumer APIs or compatibility paths.

## SDK interaction

#454 owns the SDK split. The converged state requires `phenix-sdk` to depend only on neutral or passive contract and authoring libraries. It must not use runtime plugin crates as contract libraries.

Any contract dependencies temporarily restored to make #454 compile remain migration work owned by this PR. They must not survive this PR's completion.

## Validation

Add focused negative fixtures that prove the checker rejects:

- an unclassified composition-relevant package;
- `passive-library -> runtime-plugin` contract coupling;
- undeclared `runtime-plugin -> runtime-plugin` implementation coupling;
- overlapping migration and implementation declarations while migration metadata exists;
- a legacy command-toolbelt identity alias;
- a compatibility alias for a replaced internal contract;
- reintroduction of migration-only `contract-debt` metadata after convergence.

The final exact head must pass Source, Rust, Product, and Maintenance validation.

## Completion

This slice is complete only when:

- every current composition-relevant first-party package has one role;
- the role dependency graph is mechanically checked;
- every shared contract has one canonical owner;
- `contract-debt` is empty;
- migration-only debt machinery has been deleted;
- no backwards-compatibility path remains for superseded internal contracts or identities;
- mocks and placeholders use the normal canonical contracts;
- no temporary old/new contract bridge remains;
- this implementation-slice file can be deleted without losing a current architecture rule.

Move any lasting rule into its canonical normative specification before deleting this file.