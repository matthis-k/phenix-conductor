# Persistence backend and durable schemas

Status: implementation contract.

## Purpose

Give the kernel a provider-neutral persistence mechanism for kernel infrastructure state and plugin-owned durable resources.

Persistence is infrastructure. Product schemas remain outside Core.

This document extends `spec/plugin-authoring-macro.md` and `spec/plugin-durable-data.md`.

## Ownership

The kernel owns:

- store identity;
- durable namespace identity;
- schema registration and versioning;
- transaction boundaries and atomicity;
- migration ordering and startup gating;
- recovery validation;
- corruption and failure semantics;
- namespace authority;
- backend feature negotiation;
- persistence-provider selection and bootstrap.

Plugins own:

- product field meaning;
- resource schemas;
- resource migrations;
- domain reconstruction and invariants;
- higher-level search, indexing, replication, archival, or other product behavior unless a separate generic service owns it.

The kernel does not define session, artifact, context, worker, planning, memory, model, or other product records.

## Baseline backend

Core may ship one narrow local backend so the kernel can boot and persistence conformance tests do not require another plugin.

The baseline backend may use SQLite internally. SQLite is not part of the Plugin API and is never exposed to plugins.

The baseline backend should provide only generic persistence needs such as:

- local durable storage;
- atomic transactions;
- schema materialization and migrations;
- exact key lookup and bounded declared queries;
- durable metadata and version checks;
- recovery of registered schemas.

FTS, vector search, graph traversal, replication, archival, and product-specific query policy should remain separate services rather than expanding the persistence mechanism indefinitely.

## Registered schemas

The selected backend receives the complete validated schema set for one graph generation:

```text
kernel infrastructure schemas
+ plugin durable resource schemas
= backend materialization plan
```

A relational backend may create tables and indexes. Another backend may use another representation.

Callers never issue SQL or receive backend handles.

## Kernel schemas

Kernel-private schemas contain only state required for kernel mechanisms, for example:

- graph and runtime generation metadata;
- kernel policy and configuration identity;
- durable namespace and schema metadata;
- transaction, migration, and recovery metadata where required.

They do not contain product-domain models.

## Plugin resources

Static plugin authors declare durable resources through the authoring API. They do not manually register namespaces during startup when the declaration already contains the required schema and ownership metadata.

The kernel performs mechanical schema registration and migration coordination from the resolved resource declarations.

A backend persists plugin resources without understanding their product meaning.

## Persistence providers

A persistence provider is an infrastructure provider with a special bootstrap position, not a privileged product plugin.

Core always has the baseline backend available for bootstrap. An alternate persistence provider may replace it only when configuration explicitly selects that provider and the provider can be instantiated before the target store is opened.

Persistence-provider selection therefore happens in a bootstrap phase before ordinary store-backed plugin activation.

Conceptually:

```text
inspect configuration and plugin metadata
  -> resolve bootstrap-capable persistence provider
  -> validate backend features and store binding
  -> open or claim store
  -> register complete schema set
  -> run required migrations
  -> continue ordinary candidate preparation and activation
```

A runtime plugin that provides persistence may itself be hosted only by a runtime available before store opening. Initially this means `embedded`, or a runtime-provider chain that can bootstrap without the target persistence store.

The resolver rejects a bootstrap cycle such as:

```text
persistence provider A needs plugin state from store S
store S cannot open until persistence provider A starts
```

Persistence bootstrap authority is separate from ordinary plugin authority. A backend does not gain access to product semantics or unrelated host capabilities merely because it stores data.

## Store ownership and provider changes

Backend selection occurs before opening or claiming the store.

Priority alone must not silently move an existing store to another backend.

Changing persistence providers requires one of:

- an explicit migration or export/import operation;
- a declared compatible shared storage format;
- a new store binding.

A provider change that cannot satisfy these conditions fails candidate preparation and leaves the active generation unchanged.

## Backend feature negotiation

Resource schemas declare required generic features such as:

```text
transactions
unique_keys
foreign_keys
ordered_append
indexed_range
```

A backend is eligible only when it implements the complete requirement set.

Backend feature negotiation occurs before the store is opened for the candidate generation.

## Transactions

A transaction may contain mutations from several plugin namespaces and kernel infrastructure where one declared operation requires atomicity.

The kernel validates resource ownership and authority, then submits one complete mutation set. The backend commits everything or nothing.

The persistence provider receives structural operations and schema metadata. It does not receive product-specific mutable objects.

## Recovery

The backend reconstructs stored structural records according to registered schemas.

The kernel validates kernel infrastructure state. Each plugin validates its own domain relationships after load.

A persistence backend does not declare product semantics valid.

## First-party and third-party equality

An alternate persistence provider must be able to implement the same documented persistence-provider contract as the baseline backend.

The baseline implementation may have a bootstrap implementation inside Core, but it must not expose private product capabilities that an equivalent external provider cannot represent through the persistence contract.

## Invariants

- Persistence is a kernel mechanism, not the product data model.
- Product data belongs to plugin-owned resources.
- Static authors declare resources once; kernel startup performs mechanical registration and migration.
- Callers never depend on SQLite or backend-specific APIs.
- Alternate persistence providers use one explicit bootstrap contract.
- Persistence-provider selection happens before the target store opens.
- Bootstrap dependency cycles are rejected.
- Backend selection does not redefine plugin schemas.
- Backend changes never happen implicitly because of provider priority.
- Multi-plugin state may commit atomically through the generic transaction mechanism.
- Persistence-provider authority does not grant product-domain authority.

## Required regressions

- kernel-only infrastructure state persists and restores through the baseline backend;
- a static plugin durable resource materializes without manual startup registration;
- arbitrary mock plugin data round-trips through the baseline backend;
- session, artifact, context, and memory schemas persist without backend-specific product code;
- a multi-plugin transaction commits atomically;
- a plugin cannot access another private namespace or kernel-private records;
- an alternate mock backend passes the same generic conformance fixture;
- an alternate provider can be selected before store opening;
- a persistence-provider bootstrap cycle is rejected before activation;
- unsupported schema features make a backend ineligible before store open;
- changing backend for an existing store requires explicit compatibility or migration;
- a failed backend candidate leaves the active generation unchanged;
- no product semantic module outside a plugin issues SQL or receives a backend handle.
