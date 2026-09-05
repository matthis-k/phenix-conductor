# Persistence backend and durable schemas

status: implemented
coverage:
  - rust/crates/phenix-core/tests/persistence_backend_conformance.rs
  - rust/crates/phenix-core/src/persistence_bootstrap.rs
  - rust/crates/phenix-core/src/runtime/persistence_bootstrap.rs
  - rust/crates/phenix-harness/src/persistence.rs
  - rust/crates/phenix-sdk/tests/plugin_attribute_only_gate.rs

## Purpose

Define the current Provider-neutral persistence mechanism used by Core infrastructure and Plugin-owned durable resources.

Persistence is infrastructure. Product-domain records and invariants remain owned by Plugins.

This document extends `plugin-authoring-macro.md` and `plugin-durable-data.md`.

## Current baseline

The implemented baseline provides:

- one generic `PersistenceBackend` contract;
- a local SQLite implementation hidden behind that contract;
- an alternate-backend conformance fixture;
- Plugin-owned durable namespaces;
- schema registration and ordered migrations;
- required backend-feature checks;
- atomic transactions across multiple namespaces;
- namespace ownership checks;
- bootstrap selection of a Persistence Provider before its Store is opened;
- bootstrap-cycle rejection;
- materialization of the resolved schema set before Store-backed Plugin startup;
- a prepared Store Binding retained by Core.

Live Persistence Provider replacement during ordinary graph reconciliation is not part of this baseline contract.

## Ownership

The kernel owns:

- Store Binding identity;
- durable namespace identity;
- schema registration and migration ordering;
- generic transaction atomicity;
- bootstrap Provider eligibility;
- bootstrap-cycle detection;
- recovery of kernel-owned persistence metadata;
- namespace ownership checks.

Plugins own:

- product field meaning;
- Plugin Resource schemas and migrations;
- domain reconstruction and invariants;
- product-specific indexing, search, replication, and archival behavior.

The persistence mechanism does not define sessions, artifacts, context, workers, memory, models, or other product records.

## Plugin Resources

Static Rust authors declare Plugin Resources through the authoring surface. Generated metadata supplies stable resource identity, ownership, schema version, migrations, and required generic backend features.

The resolved schema set is prepared before Store-backed Plugin startup. Authors do not manually register a namespace during `start` when the resource declaration already contains the required metadata.

A resource-only Plugin is valid.

## Backend contract

Callers use structural persistence operations. They never issue SQL or receive SQLite-native handles.

The baseline backend contract supports generic operations such as:

- schema registration;
- schema migration;
- exact key reads;
- atomic namespace transactions;
- multi-namespace transactions;
- feature negotiation.

`persistence_backend_conformance.rs` runs the same generic contract against the local SQLite backend and an alternate in-memory backend.

## Transactions

A transaction contains structural operations such as put, delete, and value assertions.

Before commit, the backend validates namespace ownership. Multi-namespace transactions stage all participating mutations and commit everything or nothing.

A failed assertion or invalid participant leaves the pre-transaction records unchanged.

Product transaction meaning remains outside Core.

## Migrations

Schemas have stable namespace identity and version.

Migrations are explicit ordered transitions owned by the Plugin Resource. Missing required migration steps or incompatible schema versions fail rather than being guessed.

Migration operations use the same structural mutation vocabulary as ordinary persistence transactions.

## Backend features

A durable schema may require generic features such as transactions, unique keys, or migrations.

A Provider is eligible only when it supports every required feature in the resolved schema set. Unsupported requirements fail before the namespace is claimed or the target Store is opened.

## Bootstrap

An alternate Persistence Provider is selected and prepared before Store-backed Plugin activation.

Bootstrap resolution validates:

- Provider identity and supported features;
- Store Binding compatibility information;
- bootstrap dependencies;
- the complete resolved schema set.

A Provider that depends on the target Store to start is rejected as a bootstrap cycle before Store opening and before Plugin startup.

The Provider receives bootstrap metadata and returns the generic `PersistenceBackend` used by the kernel.

## Authority and isolation

A Plugin may access only durable namespaces it owns through kernel-mediated persistence capability paths.

Persistence Provider status does not grant product-domain authority. The Provider stores structural data without gaining ownership of Plugin semantics.

## Recovery

The backend reconstructs stored structural records according to registered schemas. Core validates kernel-owned persistence metadata, and each Plugin remains responsible for validating its own domain relationships after load.

A backend does not declare product semantics valid.

## Invariants

- Persistence is a kernel mechanism, not the product data model.
- Product durable state belongs to Plugin-owned resources.
- Callers do not depend on SQLite or backend-native APIs.
- Alternate backends implement the same generic persistence contract.
- Schema features are checked before Store opening.
- Bootstrap dependency cycles fail before activation.
- Multi-namespace transactions are atomic.
- Namespace ownership is enforced by the persistence boundary.
- Persistence Provider authority does not imply product-domain authority.
- Live Provider replacement is not silently inferred from provider priority or graph reconciliation.

## Owner-prepared cross-plugin commits

Proposed follow-up; implementation and regressions are pending. The baseline status above does not certify this boundary.

The host accepts caller-created foreign NamespaceTransaction operations when any qualifying import exists. The provider need not have approved those writes, so an importer can bypass its domain invariants.

1. Core owns a transaction scope and immutable prepared-mutation registry. Only the active namespace owner may prepare operations through its scoped host; Core derives owner, store binding, generation, scope, and attenuated commit authority rather than accepting caller claims.

2. Preparation returns an opaque scope-local handle. The coordinator receives the handle and domain result, never an editable foreign operation list. Embedded handles use private constructors; bridged handles resolve in the same host-owned scoped registry and are checked against the requesting scope and participant. Knowing a handle identifier grants no authority.

3. The coordinator commits owner-prepared participants together with its own prepared mutations. Core validates scope, owner, store, generation, authority, cancellation, and outstanding status before invoking the backend once. Assertions are checked in that atomic transaction. Each commit attempt consumes its handles; failure requires fresh preparation.

4. Scope exit, cancellation, provider replacement, and commit release prepared state. Single-owner writes keep the existing owner-only path. Backend raw transaction operations remain internal to trusted persistence infrastructure; imports alone never authorize foreign writes.

5. Migrate session creation plus lineage creation to this protocol. Domain validation and assertion construction stay in the sessions and session-tree plugins. Remove the plugin-facing raw foreign NamespaceTransaction commit path.

Acceptance requires:

- A qualifying importer cannot fabricate, modify, reuse, or transfer another owner's mutation handle.
- Reject wrong scope, generation, store, owner, attenuated authority, cancelled scope, and replaced provider before writes.
- Session and lineage creation commit atomically; failed assertions roll back every participant.
- Embedded and bridged callers enforce the same scope checks; abandoned preparations release storage.
