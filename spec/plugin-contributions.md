# Plugin contributions

status: specification-only

## Purpose

Define the contribution data a Plugin provides to the kernel resolver without introducing product-specific registries or requiring authors to maintain Core wiring by hand.

This document extends `spec/plugin-authoring-macro.md` and `spec/plugin-durable-data.md`.

## Author declaration versus Core representation

Plugin authors declare semantic intent through the Rust authoring API or an equivalent runtime-neutral Plugin builder.

Authors write domain state, behavior, typed dependencies, Plugin Resources, configuration, and semantic annotations. They do not maintain `PluginManifest`, `ComponentManifest`, registration tables, factories, dispatch ladders, or Plugin Resource registration lists for static Plugins.

Macros and kernel-generic authoring code lower those declarations into Core contribution descriptors.

When this document refers to a manifest or contribution descriptor, it means the generated or runtime-derived Core representation consumed by the kernel resolver. It does not require manual author wiring.

## Contribution vocabulary

The Core representation may contain generic contributions such as:

```text
Components
Imports and Exports
Interface Provider endpoints
Layers
Events and Listeners
controllers
Plugin Resources and durable schemas
configuration schemas
public callables and values
Host Capability requirements
Runtime Provider requirements
lifecycle metadata
```

Every contribution has stable Plugin ownership, versioned semantic identity where needed, Graph Generation provenance, and explicit authority requirements where applicable.

## Shared contracts and implementations

Shared semantic contracts live in neutral passive owners when independent Providers and consumers need to name them.

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

A default runtime Plugin implements such a contract. It does not own the only public copy of request types, response types, IDs, or schemas used by consumers.

The kernel remains unaware of the product meaning of these contracts. The kernel sees Interface identities, schemas, authority, Plugin Resources, and composition metadata.

A Plugin may keep a private typed registry as implementation state. That registry is not a kernel registry and does not become a second Plugin-composition system.

## Providers

A Provider implements one or more versioned Interfaces. A Component Export is the Core representation of that executable Interface endpoint and is the only representation used for Interface Provider resolution.

Generated Component Exports are not mirrored into terminal `ServiceContribution`s. Raw service contributions remain valid for explicitly authored raw service dispatch and Layer interposition, but they do not form a second Provider registry for Component Imports.

Provider selection follows `spec/plugin-resolution.md`. The kernel resolver checks compatibility and authority, applies composition policy, and pins the resolved Provider plan to the Graph Generation.

A Plugin advertises capability and compatibility. It does not assign itself effective global priority or authority.

## Durable resources

A Plugin may declare Plugin Resources and schemas for canonical or derived Durable State.

The kernel owns namespace isolation, schema registration, transaction coordination, migration ordering, Persistence Provider dispatch, and authority checks.

The owning Plugin defines field meaning, domain invariants, migrations for its schema, and reconstruction of its service state.

A Plugin cannot read or mutate another Plugin's private durable namespace unless an explicit shared Interface contract and Effective Authority allow it.

## Composition

The kernel resolver consumes the complete contribution set and constructs one candidate Graph Generation.

Conceptually:

1. inspect Plugin metadata and generated or runtime-derived descriptors;
2. validate Plugin and nested identities;
3. resolve concrete Plugin dependencies;
4. validate Interface and structural compatibility;
5. resolve Provider plans and Layer order;
6. validate Plugin Resources, schemas, Runtime Provider requirements, and authority;
7. reject collisions and dependency cycles;
8. prepare and start the candidate;
9. commit the Graph Generation atomically.

Source order and registration order do not change semantics.

A Plugin never registers itself into the active runtime while executing lifecycle code. Its relationships are already present in the contribution descriptors used to build the candidate Graph Generation.

## Cross-contribution references

A Plugin may depend on another Interface, a concrete Plugin dependency, or an explicitly shared durable identity.

References grant no authority by themselves.

Concrete Plugin dependencies and Interface Imports remain distinct. A concrete dependency selects one Plugin implementation. An Interface Import asks the kernel resolver for a compatible Provider.

## Removal and replacement

Load, unload, and replacement are kernel reconciliation operations.

Removing a Plugin stops new work from entering its contributions after the new Graph Generation commits. Existing work remains pinned to its old Graph Generation until drain policy completes.

Removing a Plugin does not silently delete its Plugin Resources. A compatible replacement or later reactivation may recover them after schema validation and migration.

## Invariants

- Authors declare semantics and behavior, not manual Core wiring.
- Core contribution descriptors are generated or runtime-derived from one authoring source of truth.
- Component Exports are the canonical Interface Provider endpoints and are not duplicated into a raw terminal service registry.
- The kernel contribution model stays product-domain neutral.
- Shared semantic contracts live in neutral passive owners, not default Provider crates.
- Runtime Plugins own implementations and product behavior.
- Plugin Durable State uses isolated Plugin Resources.
- Composition is deterministic, complete, and Graph Generation-pinned.
- Plugin lifecycle code does not self-register services, Listeners, dependencies, or Plugin Resources.
- First-party and third-party Plugins use the same contribution path.
- Cross-Plugin references never grant authority.
- Plugin removal does not silently delete Durable State.

## Required regressions

- a static Plugin author does not write a manual manifest, factory, registry, or dispatch table;
- generated Core descriptors contain the declared Components, Interfaces, Plugin Resources, and Runtime Provider metadata;
- generated Component Exports do not synthesize duplicate terminal service contributions;
- a mock non-Phenix Interface composes without kernel code changes;
- session, context, and model implementations consume neutral shared contracts rather than each other's implementation crates;
- an alternate Provider replaces a first-party Provider through the same kernel resolver;
- a Plugin Resource round-trips through the baseline Persistence Provider;
- a Plugin cannot mutate another private durable namespace;
- one invalid contribution causes candidate Graph construction to fail atomically;
- forward references and source order do not change composition;
- no product-domain parallel registry is required in the kernel.
