# Plugin architecture enforcement

status: partial
coverage:
  - scripts/check-plugin-architecture.sh

## Status

Normative companion to `runtime-host-interfaces.md`, `application-integration-terminology.md`, and `plugin-runtime-bridges.md`.

This specification defines how repository validation detects architecture drift in the current plugin model.

## Goal

Make plugin ownership rules mechanically testable.

A source change that introduces a forbidden package role, package-set entry, or plugin implementation dependency must fail Source validation before behavioral tests run.

Behavior that cannot be proven from repository structure remains covered by Rust or Product tests.

## Package roles

First-party workspace packages that participate in runtime composition declare one role in Cargo package metadata.

The initial role vocabulary is deliberately small:

- `runtime-plugin`: independently activatable runtime behavior;
- `passive-library`: shared contracts, SDKs, macros, transports, or other imported code that activates nothing;
- `application`: user-facing software outside the runtime graph;
- `assembly`: catalogs, presets, bundles, and package-set metadata;
- `test-support`: fixtures used only by tests.

A role describes runtime ownership. It does not replace more specific architecture terms such as adapter, client SDK, binding, or transport.

Package names are not authoritative. A `phenix-plugin-*` prefix does not make a package a runtime plugin, and a package without that prefix may still need migration if it exports activatable runtime behavior.

## Cargo metadata

Use package-local metadata as the machine-readable source of truth.

Example:

```toml
[package.metadata.phenix]
role = "runtime-plugin"
```

Passive authoring support uses the same form:

```toml
[package.metadata.phenix]
role = "passive-library"
```

Repository validation reads this through `cargo metadata`. Do not maintain a second complete package-role table in a script or Nix module.

## Runtime plugin dependencies

A runtime plugin must communicate with another runtime plugin through stable Core/domain contracts and resolved component imports.

For normal and build dependencies, a direct `runtime-plugin -> runtime-plugin` Cargo edge is rejected unless the consumer declares that edge as intentional implementation reuse.

Development-only dependencies are outside this rule because tests may compose concrete implementations.

### Intentional implementation reuse

Rare implementation reuse is declared next to the consuming package and must include a reason.

Conceptually:

```toml
[package.metadata.phenix.implementation-dependencies]
phenix-plugin-example = "Shares the provider-specific parser; no runtime contract exists for this implementation detail."
```

The validation check requires every declared implementation dependency to match an actual direct normal or build dependency. Stale declarations fail.

A contract import, request type, response type, interface ID, schema, manifest helper, default-provider handle, or shared domain value is never sufficient reason for an implementation dependency.

## Forbidden migration metadata

`contract-debt` is migration-only metadata and is forbidden in the current architecture. Shared contracts belong to neutral owners; a runtime plugin must not retain a direct runtime-plugin dependency merely to import a contract.

Source validation rejects any package that reintroduces `contract-debt`, as well as undeclared `runtime-plugin -> runtime-plugin` implementation edges. Deliberate implementation sharing uses `implementation-dependencies` with a non-empty reason and must match a real direct normal or build dependency.

## Runtime package set

`phenixPlugins.${system}` contains runtime plugins only.

Repository validation must reject entries that resolve to packages whose declared role is `passive-library`, `application`, `assembly`, or `test-support`.

The check must also reject multiple independently named plugin entries that are packaged from one implementation crate when those plugins are specified as independently packaged. The current basic model, tools, skills, and context split is the first required case.

Catalogs and presets may reference runtime plugins but do not appear as runtime plugins unless they independently export activatable behavior.

The Rust SDK, SDK macros, client SDKs, language bindings, transports, and user-facing applications never appear in `phenixPlugins` solely because they are distributed by the same flake.

## Application and protocol categories

The application-integration terminology is enforced at package boundaries:

- applications do not get runtime plugin IDs merely to reach Phenix;
- adapters may be runtime plugins because they implement an external protocol on the Phenix side;
- client SDKs, bindings, and transports are passive libraries;
- transport choice cannot create a parallel runtime package category;
- the prerelease `phenixClients` compatibility category is removed when its remaining consumers move to the canonical application/client-SDK model.

The terminal CLI migration follows `application-cli.md`, which is the canonical source for application and command-toolbelt identities.

Repository enforcement must reject the legacy runtime package `phenix-plugin-cli`, runtime plugin/component ID `phenix.cli`, `phenix.cli.*` service IDs, and a `phenixPlugins.${system}.cli` entry. Rename the runtime package, plugin ID, component ID, and service IDs together. Keep no prerelease compatibility alias.

## Runtime bridge enforcement

The runtime-bridge specification adds behavioral invariants that source classification alone cannot prove.

Rust tests must prove at least:

- Core owns only the `embedded` bootstrap runtime initially;
- runtime providers enter through one open runtime-provider interface rather than a closed runtime enum;
- runtime dependency cycles are rejected;
- graph generations pin exact artifact revisions and runtime providers;
- failed candidate preparation does not replace the active generation;
- guest authority is derived independently from runtime-bridge authority;
- replacing an implementation through the same component contract requires no consumer source change.

These tests belong near the kernel/runtime code that owns the behavior. Do not mirror their expected values in a source-classification script.

## Validation command

Maintenance gains one Source check named `plugin-architecture`.

The check reads Cargo metadata and the evaluated package-set metadata needed to verify the rules above. It fails with concrete package and dependency names.

The check must remain read-only and deterministic. It must not build plugin artifacts, start the conductor, or inspect network state.

Rust and Product tests continue to own runtime behavior.

## Migration order

Apply enforcement in small changes:

1. add package-role metadata and the dependency gate with exact current contract debt;
2. add package-set role validation and expose current packaging violations;
3. move shared contracts to neutral owners and delete debt entries as edges disappear;
4. reclassify SDK, command toolbelt, catalog, provider bundles, and basic default packages;
5. add the runtime-bridge behavioral tests as the management API lands;
6. finish with zero contract debt and no packaging exceptions.

Each migration PR updates the enforcement data in the same commit that changes the architecture. Do not land a migration that leaves the gate knowingly stale.

## Completion criteria

Architecture enforcement is complete when:

- every composition-relevant first-party package has one declared role;
- Source validation rejects undeclared runtime-plugin implementation edges;
- intentional implementation dependencies are exact and documented;
- contract debt is empty;
- `phenixPlugins` contains runtime plugins only;
- independently specified default plugins have independent package ownership;
- the terminal CLI, SDKs, bindings, client SDKs, transports, catalogs, and presets are absent from the runtime plugin set unless they independently export runtime behavior;
- runtime command probing matches the canonical command-toolbelt identity in `application-cli.md`;
- runtime-bridge invariants are covered by behavioral tests;
- exact-head Source, Rust, Product, and Maintenance validation passes.

## Simplification audit

Before adding a new rule, check whether Cargo metadata, the component graph, or the evaluated package set already contains the fact. Derive from existing state instead of maintaining a second registry. Keep source checks structural and keep runtime semantics in behavioral tests.
