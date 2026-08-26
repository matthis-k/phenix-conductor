# Persistence backend and durable schemas

Status: implementation contract.

## Purpose

Give the kernel a provider-neutral persistence mechanism for kernel infrastructure state and arbitrary plugin-owned durable schemas.

Persistence is infrastructure. Agent-product schemas are userspace.

Requires `spec/plugin-kernel-primitives.md` and `spec/plugin-durable-data.md`.

## Kernel persistence mechanism

The kernel owns:

- store identity;
- durable namespace identity;
- schema registration/versioning;
- transaction boundaries and atomicity;
- migration ordering/startup gating;
- recovery validation;
- corruption/failure semantics;
- namespace permissions;
- backend feature negotiation and selection.

The kernel does not predefine Phenix session, artifact, context, worker, planning, model, or other product records.

## Baseline local backend

The kernel may ship one simple local backend so infrastructure can boot and plugin schemas can be exercised without installing a separate persistence provider.

The backend may reuse SQLite. SQLite remains an implementation detail and is never exposed to plugins.

The baseline backend should stay narrow:

- local store;
- atomic transactions;
- schema materialization/migrations;
- exact key lookup and bounded declared queries;
- durable metadata/version checks;
- recovery of registered schemas.

It should not absorb product FTS, semantic search, replication, archival, or feature-specific query logic.

## Registered schemas

The active backend receives the complete validated schema set:

```text
kernel infrastructure schemas
+ plugin durable schemas
= physical backend schema
```

A relational backend may materialize tables/indexes. Another backend may choose another representation.

Callers never issue SQL or receive backend handles.

## Kernel schemas

Kernel-private schemas contain only state required for kernel mechanisms, for example:

- plugin/runtime generation metadata;
- kernel policy/configuration snapshots;
- durable namespace/schema metadata;
- transaction/migration/recovery metadata where required.

They do not contain miniature agent-harness models.

## Plugin schemas

Plugins register their product schemas through `spec/plugin-durable-data.md`.

A backend persists them without understanding what a session, artifact, context resource, QML cache, worker result, or GitHub integration means.

## Alternative persistence providers

A persistence provider may replace the baseline backend when explicitly configured and compatible.

Selection occurs before opening/claiming the store. Priority must not silently migrate an existing store to a different backend.

Switching backend requires explicit migration/export-import unless the implementations share a declared storage format.

## Backend feature negotiation

Schemas declare required generic features such as:

```text
transactions
unique_keys
foreign_keys
ordered_append
indexed_range
```

A backend is eligible only when it implements the complete requirement set.

FTS/vector/graph operations should remain separate services rather than continuously expanding the persistence mechanism.

## Transactions

A transaction may contain mutations from several plugin namespaces and kernel infrastructure when necessary.

The kernel validates schema ownership and authority, then submits one complete mutation set to the backend. The backend commits everything or nothing.

## Recovery

The backend reconstructs stored records according to registered schemas. The kernel validates kernel infrastructure state. Each plugin validates its own domain relationships after load.

The backend does not declare plugin semantics valid.

## Invariants

- Kernel persistence is a mechanism, not the Phenix data model.
- Product data belongs to plugin schemas.
- Baseline backend makes kernel infrastructure and plugin conformance tests durable, not a miniature Harness.
- Backend choice does not redefine plugin schemas.
- Multi-plugin state can commit atomically.
- No caller depends on SQLite APIs.

## Required regressions

- kernel-only infrastructure state persists/restores through the baseline backend;
- arbitrary mock plugin schema materializes and round-trips;
- Phenix session/artifact/context schemas persist without backend-specific code;
- multi-plugin transaction commits atomically;
- plugin cannot access another namespace or kernel-private records;
- alternate mock backend passes the same generic conformance fixture;
- unsupported schema feature makes a backend ineligible before store open;
- no product semantic module outside a plugin issues SQL;
- higher-priority backend cannot silently take over an existing store.