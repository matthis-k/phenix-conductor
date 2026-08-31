# Plugin hygiene

## Status

Specification only. This PR defines ownership and packaging rules for the existing plugin set. It depends conceptually on #444, which defines stable domain and host interfaces.

## Goal

Make `plugin` mean one thing: an independently activatable runtime implementation selected through the component graph.

Shared contracts, authoring support, macros, presets, catalogs, and application code must have explicit non-plugin ownership.

The migration must preserve current runtime behavior unless this specification names a behavior change.

## Classification rule

A runtime component belongs in a plugin when one or more of these are true:

- users may disable it;
- another implementation may replace it;
- it integrates with an external system;
- it adds policy or higher-level behavior over simpler mechanisms;
- it owns runtime authority, lifecycle, state, or dependency relationships.

Code required to define, compile, load, validate, connect, or author plugins belongs outside runtime plugins.

## Ownership

### Core

`phenix-core` owns plugin-runtime mechanisms and stable interface definitions from #444:

- plugin and component identity;
- manifests, activation, import/export binding, and graph resolution;
- authority and attenuation;
- `PhenixValue`, schema, contract compatibility, and typed interface handles;
- persistence mechanisms exposed to plugins;
- stable domain and host interface contracts required for composition.

Core does not own rich first-party implementations of those interfaces.

### Domain

`phenix-domain` owns shared domain values that are useful across implementations and applications but do not implement runtime behavior.

Examples include session IDs, execution IDs, model targets, descriptors, failure records, and workspace records.

A plugin implementation may use domain values. Domain values must not depend on plugin implementation crates.

### SDK

`phenix-sdk` is the Rust authoring library.

It owns or re-exports:

- plugin authoring helpers;
- typed clients and convenience wrappers over Core interfaces;
- standard domain types needed by plugin authors;
- provider authoring helpers where appropriate;
- `phenix-sdk-macros` derives and attributes.

Importing `phenix-sdk` must not activate runtime behavior.

### Macros

`phenix-sdk-macros` remains compile-time support. It is never a runtime plugin.

### Runtime plugins

Runtime plugin crates own implementations, plugin manifests, component manifests, persistent plugin state, and implementation-specific configuration.

A consumer must not depend on a default implementation crate merely to name an interface, request, response, or shared value.

### Presets and catalogs

Presets select groups of plugins and supply default configuration. Catalogs enumerate or re-export available plugins. Neither is a runtime plugin unless it independently exports runtime behavior.

## Required migration

### Neutralize interface contracts

Move standard interface definitions out of plugin implementation crates.

At minimum, neutralize the contracts currently imported across plugin crate boundaries for:

- execution and agent-loop behavior;
- sessions and session mutation;
- workspace access;
- context;
- model routing;
- jobs;
- planning;
- frontend services;
- artifacts;
- language services;
- hooks;
- CLI integration where another component consumes its contract.

The stable interface ID, schema, command/response types, and typed handle belong with Core/domain contracts from #444. `phenix-sdk` may re-export ergonomic wrappers.

After migration, replacing a first-party provider must not require consumer source changes or a dependency on the replaced implementation crate.

### Remove contract-only plugin dependencies

Feature plugins must not depend on another feature-plugin crate solely for contract types.

Examples that must disappear as contract-only dependencies include:

- frontend -> execution implementation;
- context -> execution implementation;
- CLI -> workspace implementation;
- session-tree -> sessions implementation;
- debug -> sessions/context/planning/jobs/models/frontend implementations;
- hooks -> context/execution implementations.

Intentional implementation reuse may remain only when the dependency is implementation-specific and cannot be expressed through a runtime interface. Such dependencies must be documented in the consuming crate.

Add a regression check that detects new contract-only dependencies between runtime plugin crates.

### Split the SDK roles

The current `phenix-plugin-sdk` mixes an authoring API with an activatable runtime facade.

Move authoring-only helpers into `phenix-sdk`.

If the runtime facade services remain useful, keep them as an ordinary plugin under a runtime name such as `phenix-plugin-api`. Its responsibilities are limited to runtime services such as session, tool, skill, or config convenience APIs.

`phenix-sdk` and `phenix-sdk-macros` must remain usable without activating that facade plugin.

### Make option ownership local

`phenix.options` remains a runtime plugin because it owns option definition, persistence, precedence, scope, and resolution behavior.

Feature-specific option definitions move to the feature that owns them.

For example:

- `session.*` defaults belong to the session implementation or session policy plugin;
- `model.*` defaults belong to model/routing plugins;
- `tools.*` defaults belong to tool policy plugins.

The options implementation stores and resolves definitions supplied by other components. It must not contain a growing registry of unrelated feature defaults.

### Reclassify provider bundles

Individual providers remain ordinary runtime plugins.

The current common-provider collection becomes preset or bundle metadata. It must not present itself as a peer runtime provider when it exports no runtime service.

The common preset may still enable OpenAI, Anthropic, OpenRouter, Groq, Gemini, DeepSeek, Together, Mistral, xAI, Fireworks, or later provider plugins.

### Make basic defaults independently packaged

`phenix-plugin-basic-agent` currently contains independently named model, tool, skill, and context plugins.

Each independently activatable plugin must have independent package ownership. Prefer separate crates for the basic model, tools, skills, and context implementations. A default bundle may depend on those crates for convenience.

A deterministic echo model used only as a fixture belongs in test support. A shipped basic-model plugin must implement useful default runtime behavior.

### Keep feature plugins as plugins

The following behavior remains plugin-owned:

- artifacts;
- CLI integration;
- context policy;
- debug aggregation;
- execution and agent-loop policy;
- frontend services;
- hooks;
- jobs;
- language services;
- model routing;
- options resolution;
- planning;
- sessions;
- session trees;
- workspace integration.

`repository-workers` also remains a plugin, but it is a first-party domain extra rather than part of the minimal default harness.

### Keep catalogs non-runtime

`phenix-plugin-catalog` may remain as an assembly/re-export crate. It must not acquire a plugin manifest merely because it lists plugins.

Default bundles and application presets decide which catalog entries activate.

## Dependency direction

The target Rust dependency direction is:

```text
phenix-core
    ^
    |
phenix-domain
    ^
    |
phenix-sdk ----> phenix-sdk-macros
    ^
    |
runtime plugin implementations
    ^
    |
presets / catalogs / applications
```

Core may expose domain-independent contract mechanisms directly. `phenix-domain` may depend on Core base types where necessary.

Runtime plugin implementations must not form compile-time dependency chains merely to communicate at runtime. They communicate through resolved Core interfaces.

## Runtime composition

The migration must preserve the existing composition model:

- imports bind before activation;
- replacement providers use the same stable interface contract;
- authority follows the resolved import/export edge;
- missing required providers fail composition;
- optional providers remain explicit optional imports;
- no hidden Core fallback is introduced;
- no string-based runtime service locator is introduced.

Session-tree remains the reference example. Flat sessions are a simple implementation; session-tree adds richer behavior through stable session contracts rather than compile-time ownership of the flat-session implementation.

## Naming

Use names that identify role rather than implementation accident.

- `phenix-sdk`: authoring library;
- `phenix-sdk-macros`: compile-time macros;
- `phenix-plugin-*`: activatable runtime implementations;
- `phenix-plugin-catalog`: catalog of runtime plugins;
- preset or bundle crates: composition metadata, not fake runtime plugins.

Avoid a crate named `plugin` when its only job is authoring support or static grouping.

## Compatibility

This is a prerelease codebase. Complete the ownership migration rather than retaining duplicate legacy contracts or compatibility shims.

There must be one authoritative definition for each stable interface and shared domain type after the migration.

## Validation

Implementation is complete when all of the following hold:

- every runtime plugin crate has actual activatable runtime behavior;
- shared authoring crates activate nothing;
- presets and catalogs are not registered as runtime providers unless they export runtime behavior;
- consumers compile against neutral contracts rather than default implementation crates;
- first-party implementations can be replaced without consumer source changes;
- option defaults are contributed by their owning features;
- the common provider bundle is represented as composition metadata;
- basic model/tools/skills/context have independent package ownership;
- repository-workers is excluded from the minimal default harness;
- no duplicate legacy contract definitions remain;
- a dependency regression test prevents contract-only plugin-to-plugin crate edges;
- `cargo fmt --check`, workspace tests, workspace Clippy, Product, and Maintenance pass at the exact PR head.

## Simplification audit

Before completion, check whether the migration introduced code that can be deleted, duplicated an existing abstraction, stored derivable state, kept an unnecessary intermediate representation, or added custom error handling where propagation is sufficient.
