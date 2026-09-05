# Persistence backend and durable schemas

status: partial
coverage:
  - rust/crates/phenix-core/src/persistence.rs
  - rust/crates/phenix-core/src/persistence_bootstrap.rs
  - rust/crates/phenix-core/src/persistence_provider.rs
  - rust/crates/phenix-core/src/runtime/persistence_bootstrap.rs
  - rust/crates/phenix-core/tests/persistence_backend_conformance.rs
  - rust/crates/phenix-harness/src/persistence.rs

The resolved schema set is materialized during initial activation. Harness construction
can select a bootstrap Provider before Store opening. Core retains its prepared Store
Binding, and Provider preparation receives transition and active Store metadata.
Transactional schema preparation and Store Binding replacement during live
reconciliation remain incomplete.

## Purpose

Give the kernel a Provider-neutral persistence mechanism for kernel infrastructure Durable State and Plugin-owned Plugin Resources.

Persistence is infrastructure. Product-domain schemas remain outside Core.

This document extends `spec/plugin-authoring-macro.md` and `spec/plugin-durable-data.md`.

## Ownership

The kernel owns:

- Store identity;
- durable namespace identity;
- schema registration and versioning;
- transaction boundaries and atomicity;
- migration ordering and startup gating;
- recovery validation;
- corruption and failure semantics;
- namespace authority;
- Persistence Provider feature negotiation;
- Persistence Provider selection and bootstrap.

Plugins own:

- product field meaning;
- Plugin Resource schemas;
- Plugin Resource migrations;
- domain reconstruction and invariants;
- higher-level search, indexing, replication, archival, or other product behavior unless a separate generic Interface owns it.

The kernel does not define session, artifact, context, worker, planning, memory, model, or other product records.

## Baseline persistence provider

Core may ship one narrow local Persistence Provider so the kernel can bootstrap and persistence conformance tests do not require another Plugin.

The baseline Persistence Provider may use SQLite internally. SQLite is not part of the Plugin API and is never exposed to Plugins.

The baseline Persistence Provider should provide only generic persistence capabilities such as:

- local Durable State storage;
- atomic transactions;
- schema materialization and migrations;
- exact key lookup and bounded declared queries;
- durable metadata and version checks;
- recovery of registered schemas.

FTS, vector search, graph traversal, replication, archival, and product-specific query policy should remain separate Interfaces rather than expanding the persistence mechanism indefinitely.

## Registered schemas

The selected Persistence Provider receives the complete validated schema set for one Graph Generation:

```text
kernel infrastructure schemas
+ Plugin Resource schemas
= persistence materialization plan
```

A relational Persistence Provider may create tables and indexes. Another Persistence Provider may use another representation.

Callers never issue SQL or receive Persistence Provider-native handles.

## Kernel schemas

Kernel-private schemas contain only Durable State required for kernel mechanisms, for example:

- Graph Generation and Plugin Runtime generation metadata;
- kernel policy and configuration identity;
- durable namespace and schema metadata;
- transaction, migration, and recovery metadata where required.

They do not contain product-domain models.

## Plugin resources

Static Plugin authors declare Plugin Resources through the authoring API. They do not manually register namespaces during startup when the declaration already contains the required schema and ownership metadata.

The kernel performs mechanical schema registration and migration coordination from the resolved Plugin Resource declarations.

A Persistence Provider persists Plugin Resources without understanding their product meaning.

## Persistence providers

A Persistence Provider is an infrastructure Provider with an explicit bootstrap position, not a privileged product Plugin.

Core always has the baseline Persistence Provider available for bootstrap. An alternate Persistence Provider may replace it only when product composition explicitly selects that Provider and the Provider can be instantiated before the target Store is opened.

Persistence Provider selection therefore happens in a bootstrap phase before ordinary Store-backed Plugin activation.

Conceptually:

```text
inspect product composition and Plugin metadata
  -> resolve bootstrap-capable Persistence Provider
  -> validate Persistence Provider features and Store Binding
  -> open or claim Store
  -> register complete schema set
  -> run required migrations
  -> continue ordinary candidate preparation and activation
```

A runtime Plugin that provides persistence may itself use only an Execution Runtime available before Store opening. Initially this means `embedded`, or a Runtime Provider chain that can bootstrap without the target Store.

The kernel resolver rejects a bootstrap cycle such as:

```text
Persistence Provider A needs Plugin state from Store S
Store S cannot open until Persistence Provider A starts
```

Persistence bootstrap authority is separate from ordinary Plugin authority. A Persistence Provider does not gain product-domain authority or unrelated Host Capabilities merely because it stores data.

## Store binding and provider changes

Persistence Provider selection occurs before opening or claiming the Store Binding.

Provider priority alone must not silently move an existing Store Binding to another Persistence Provider.

Changing Persistence Providers requires one of:

- an explicit migration or export/import operation;
- a declared compatible shared storage format;
- a new Store Binding.

A Persistence Provider change that cannot satisfy these conditions fails candidate preparation and leaves the active Graph Generation unchanged.

## Provider feature negotiation

Plugin Resource schemas declare required generic features such as:

```text
transactions
unique_keys
foreign_keys
ordered_append
indexed_range
```

A Persistence Provider is eligible only when it implements the complete requirement set.

Persistence Provider feature negotiation occurs before the Store is opened for the candidate Graph Generation.

## Transactions

A transaction may contain mutations from several Plugin namespaces and kernel infrastructure where one declared operation requires atomicity.

The kernel validates Plugin Resource ownership and Effective Authority, then submits one complete structural mutation set. The Persistence Provider commits everything or nothing.

The Persistence Provider receives structural operations and schema metadata. It does not receive product-specific mutable objects.

## Recovery

The Persistence Provider reconstructs stored structural records according to registered schemas.

The kernel validates kernel infrastructure Durable State. Each Plugin validates its own domain relationships after load.

A Persistence Provider does not declare product semantics valid.

## First-party and third-party equality

An alternate Persistence Provider must be able to implement the same documented Persistence Provider contract as the baseline Provider.

The baseline implementation may have bootstrap code inside Core, but it must not expose private product capabilities that an equivalent alternate Persistence Provider cannot represent through the persistence contract.

## Invariants

- Persistence is a kernel mechanism, not the product data model.
- Product Durable State belongs to Plugin-owned Plugin Resources.
- Static authors declare Plugin Resources once; kernel startup performs mechanical registration and migration.
- Callers never depend on SQLite or Persistence Provider-specific APIs.
- Alternate Persistence Providers use one explicit bootstrap contract.
- Persistence Provider selection happens before the target Store Binding opens.
- Bootstrap dependency cycles are rejected.
- Persistence Provider selection does not redefine Plugin Resource schemas.
- Persistence Provider changes never happen implicitly because of Provider priority.
- Multi-Plugin state may commit atomically through the generic transaction mechanism.
- Persistence Provider authority does not grant product-domain authority.

## Required regressions

- kernel-only infrastructure Durable State persists and restores through the baseline Persistence Provider;
- a static Plugin Resource materializes without manual startup registration;
- arbitrary mock Plugin data round-trips through the baseline Persistence Provider;
- session, artifact, context, and memory schemas persist without Persistence Provider-specific product code;
- a multi-Plugin transaction commits atomically;
- a Plugin cannot access another private namespace or kernel-private records;
- an alternate mock Persistence Provider passes the same generic conformance fixture;
- an alternate Persistence Provider can be selected before Store opening;
- a Persistence Provider bootstrap cycle is rejected before activation;
- unsupported schema features make a Persistence Provider ineligible before Store open;
- changing Persistence Provider for an existing Store Binding requires explicit compatibility or migration;
- a failed Persistence Provider candidate leaves the active Graph Generation unchanged;
- no product semantic module outside a Plugin issues SQL or receives a Persistence Provider-native handle.
