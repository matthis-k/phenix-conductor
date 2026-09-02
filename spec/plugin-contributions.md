# Plugin contributions

status: implemented

## Purpose

Let plugins define userspace services and persist their state without introducing domain-specific kernel registries.

Requires `spec/plugins.md` and `spec/plugin-durable-data.md`.

## Kernel contribution vocabulary

`PluginManifest` may declare generic contributions:

```text
services
capability providers
resources
event subscriptions/handlers
durable schemas
persistence providers
```

Every contribution has stable plugin ownership, contract/schema version where applicable, immutable configuration identity when pinned, and exact provenance when consumed.

## Userspace-defined contracts

Higher-level concepts are defined by plugins, not the kernel.

For example, the Phenix Plugin Suite may define contracts and schemas for:

```text
sessions
artifacts
context
skills
tools
callables
orchestration
workers
models/providers
language services
```

The kernel does not need a separate built-in registry for each concept. Plugins expose them through generic service/capability/resource mechanisms.

A suite service may itself maintain a typed registry as part of that service's implementation. That registry remains userspace-owned and replaceable.

## Capability providers

A provider implements one or more versioned service contracts.

Provider resolution uses kernel permission eligibility, explicit binding, configured priority, availability, and deterministic tie-breaking.

The contract owner defines semantic input/output and composition rules. The kernel only mediates registration, selection, authority, invocation, and provenance.

## Durable schemas

A plugin may contribute a namespaced `DurableSchema` for canonical or derived service state.

The kernel owns schema registration, namespace isolation, transactions, migration lifecycle, and backend dispatch. The plugin owns field meaning, validation, and service reconstruction.

Examples:

```text
phenix-sessions
  session records/events

phenix-session-tree
  lineage/navigation

qml
  QML-specific feature state
```

A plugin cannot read or mutate another plugin namespace unless an explicit shared contract and authority permit it.

## Registration

Plugin registration is atomic per kernel configuration snapshot:

1. parse the complete manifest;
2. validate plugin/contribution identities;
3. validate service/capability versions;
4. validate durable namespaces/schemas and backend requirements;
5. resolve declared dependencies against the complete configuration;
6. reject collisions and invalid dependency cycles;
7. activate the complete contribution set as one pinned configuration.

Source registration order must not change semantics.

## Cross-contribution references

A plugin service may depend on another service/capability or reference another plugin's durable identity through a declared contract.

References grant no authority by themselves.

## Removal and reload

Configuration reload creates a new immutable kernel policy snapshot. Existing calls/tasks keep pinned provider identities where required by their contracts.

Removing a plugin stops new use of its contributions but does not silently delete its durable namespace. A compatible re-enabled plugin may recover it after schema validation.

## Invariants

- Kernel contribution APIs remain domain-neutral.
- Agent-domain registries live in userspace services.
- Plugin state uses namespaced durable schemas.
- Registration is deterministic and atomic.
- First-party plugins use the same generic registration path as alternatives.
- Cross-service references never grant authority.
- Plugin removal does not silently delete durable state.

## Required regressions

- mock non-Phenix service registers without kernel code changes;
- Phenix session/artifact/context services register through generic service contracts;
- alternate provider replaces a first-party provider through the same resolver;
- plugin durable schema round-trips through the baseline backend;
- plugin cannot mutate another durable namespace;
- manifest registration fails atomically on one invalid contribution/schema;
- forward references are order-independent;
- no Phenix-domain parallel registry is required in the kernel.