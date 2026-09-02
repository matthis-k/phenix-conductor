# Plugin contributions

Status: implementation contract.

## Purpose

Define the data a plugin contributes for kernel composition without introducing product-specific registries or requiring authors to maintain Core wiring by hand.

This document extends `spec/plugin-authoring-macro.md` and `spec/plugin-durable-data.md`.

## Author declaration versus Core representation

Plugin authors declare semantic intent through the Rust authoring API or an equivalent runtime-neutral builder.

Authors write domain state, behavior, typed dependencies, resources, configuration, and semantic annotations. They do not maintain `PluginManifest`, `ComponentManifest`, registration tables, factories, dispatch ladders, or resource-registration lists for static plugins.

Macros and kernel-generic authoring code lower those declarations into Core contribution descriptors.

When this document refers to a manifest or contribution descriptor, it means the generated or runtime-derived Core representation consumed by the resolver. It does not require manual author wiring.

## Contribution vocabulary

The Core representation may contain generic contributions such as:

```text
components
imports and exports
terminal providers
layers
events and listeners
controllers
resources and durable schemas
configuration schemas
public callables and values
host requirements
runtime requirements
lifecycle metadata
```

Every contribution has stable plugin ownership, versioned semantic identity where needed, graph-generation provenance, and explicit authority requirements where applicable.

## Shared contracts and implementations

Shared semantic contracts live in neutral passive owners when independent providers and consumers need to name them.

Examples include contracts for:

```text
sessions
context
models
tools
skills
workspace
execution
artifacts
memory
orchestration
language services
```

A default runtime plugin implements such a contract. It does not own the only public copy of request types, response types, IDs, or schemas used by consumers.

The kernel remains unaware of the product meaning of these contracts. It sees interface identities, schemas, authority, resources, and composition metadata.

A plugin may keep a private typed registry as implementation state. That registry is not a kernel registry and does not become a second plugin-composition system.

## Providers

A provider implements one or more versioned interfaces.

Provider selection follows `spec/plugin-resolution.md`. The resolver checks compatibility and authority, applies composition policy, and pins the provider plan to the graph generation.

A plugin advertises capability and compatibility. It does not assign itself effective global priority or authority.

## Durable resources

A plugin may declare durable resources and schemas for canonical or derived state.

The kernel owns namespace isolation, schema registration, transaction coordination, migration ordering, backend dispatch, and authority checks.

The plugin owns field meaning, domain invariants, migrations for its schema, and reconstruction of its service state.

A plugin cannot read or mutate another plugin's private durable namespace unless an explicit shared contract and authority allow it.

## Composition

The resolver consumes the complete contribution set and constructs one candidate graph generation.

Conceptually:

1. inspect plugin metadata and generated/runtime-derived descriptors;
2. validate plugin and nested identities;
3. resolve concrete plugin dependencies;
4. validate interface and structural compatibility;
5. resolve provider plans and layer order;
6. validate resources, schemas, runtime requirements, and authority;
7. reject collisions and dependency cycles;
8. prepare and start the candidate;
9. commit the generation atomically.

Source order and registration order do not change semantics.

A plugin never registers itself into the active runtime while executing lifecycle code. Its relationships are already present in the contribution descriptors used to build the candidate graph.

## Cross-contribution references

A plugin may depend on another interface, a concrete plugin dependency, or an explicitly shared durable identity.

References grant no authority by themselves.

Concrete plugin dependencies and interface imports remain distinct. A concrete dependency selects one implementation. An interface import asks the resolver for a compatible provider.

## Removal and replacement

Load, unload, and replacement are kernel reconciliation operations.

Removing a plugin stops new work from entering its contributions after the new generation commits. Existing work remains pinned to its old generation until drain policy completes.

Removing a plugin does not silently delete its durable resources. A compatible replacement or later reactivation may recover them after schema validation and migration.

## Invariants

- Authors declare semantics and behavior, not manual Core wiring.
- Core contribution descriptors are generated or runtime-derived from one authoring source of truth.
- The kernel contribution model stays product-domain neutral.
- Shared semantic contracts live in neutral passive owners, not default provider crates.
- Runtime plugins own implementations and product behavior.
- Plugin state uses isolated durable resources.
- Composition is deterministic, complete, and generation-pinned.
- Lifecycle code does not self-register services, listeners, dependencies, or resources.
- First-party and third-party plugins use the same contribution path.
- Cross-plugin references never grant authority.
- Plugin removal does not silently delete durable state.

## Required regressions

- a static plugin author does not write a manual manifest, factory, registry, or dispatch table;
- generated Core descriptors contain the declared components, interfaces, resources, and runtime metadata;
- a mock non-Phenix interface composes without kernel code changes;
- session, context, and model implementations consume neutral shared contracts rather than each other's implementation crates;
- an alternate provider replaces a first-party provider through the same resolver;
- a plugin durable resource round-trips through the baseline backend;
- a plugin cannot mutate another private durable namespace;
- one invalid contribution causes candidate graph construction to fail atomically;
- forward references and source order do not change composition;
- no product-domain parallel registry is required in the kernel.
