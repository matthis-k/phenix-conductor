# Workspace language services

## Purpose

The conductor owns language-service selection for each workspace. Frontends may supply a live provider. Phenix may also run a managed provider. Agents and backends never select a frontend connection or language-server process directly.

This slice defines provider identity, capability negotiation, selection, epochs, and configuration ownership. Concrete LSP process management and execution tools belong to the next slice.

## Service key

One provider is active for each workspace and language-service kind.

```text
WorkspaceId x LanguageServiceKind -> active provider epoch
```

`LanguageServiceKind` identifies one semantic language service, such as Rust language intelligence. It is stable across provider replacements.

The conductor never merges state from two providers for one kind. If a provider cannot satisfy the required capability set, the conductor chooses another eligible provider or a managed provider.

## Provider sources

A provider is either:

```text
frontend-linked
managed
```

A frontend-linked provider uses the frontend-service transport. Its registration is process-local and disappears with the frontend connection.

A managed provider is defined by the pinned `CompiledConfiguration`. Its launch definition is configuration semantics and contributes to the configuration fingerprint. The process handle and protocol state are process-local.

## Capabilities

Language providers advertise typed capabilities:

```text
requests
notifications
shared_diagnostics
background_documents
dirty_buffers
```

Capabilities describe behavior the provider can perform. They do not grant execution authority or callable delegation.

A frontend must advertise only behavior it can provide without violating frontend semantics. In particular, `background_documents` means the conductor can make a document known to the provider without visibly opening editor UI.

The conductor evaluates one provider against the complete required capability set. It does not combine partial providers.

## Selection

Provider selection is deterministic.

1. Keep the current provider when it remains live and satisfies the required capabilities.
2. Apply an explicit configured provider preference when one exists and is eligible.
3. Otherwise choose one eligible frontend provider by stable provider identity and connection identity.
4. If no eligible frontend provider exists, select the configured managed provider for the service kind.
5. If no provider satisfies the contract, report the service as unavailable.

Workspace-owned selection uses the live frontend provider catalog introduced by `spec/frontend-services.md`. It addresses the selected connection directly. It does not create synthetic execution ownership.

## Epochs

Each active provider lifetime has a monotonically increasing epoch within the workspace service.

```text
provider epoch N
  -> disconnect, replacement, restart, or incompatible reconfiguration
provider epoch N+1
```

The epoch changes whenever provider identity or live provider state changes enough that old request state cannot be assumed.

A request belongs to the epoch in which it was dispatched. If that epoch ends before the request completes, the request fails with a typed provider-change error. The conductor does not replay it against the replacement provider.

A process restart starts a new live epoch even when the same provider is selected again. Each live managed-process registration therefore carries a process generation distinct from the immutable managed provider definition.

## Frontend-linked providers

A frontend language provider advertises through the generic frontend-service catalog. Its descriptor identifies the service kind, provider identity, and language capabilities.

The generic frontend-service descriptor uses this canonical encoding:

```text
provider id: language/<service-kind>/<provider-id>

capabilities:
language.requests
language.notifications
language.shared_diagnostics
language.background_documents
language.dirty_buffers
```

The language-service layer ignores unrelated frontend providers. It rejects malformed `language/` descriptors and converts valid descriptors to the typed language-provider model before selection.

Frontend notifications such as diagnostics or provider-state changes retain their source connection identity. The language-service layer accepts them only when they match the provider and epoch currently selected for that service.

Dirty frontend buffers are allowed only when the provider advertises `dirty_buffers`. Results that may depend on unsaved content carry provenance. The next slice defines durable consumed observations.

## Managed providers

A managed provider definition contains at least:

```text
service kind
provider identity
command
arguments
capabilities
```

Definitions are immutable configuration semantics. Reload creates a new configuration revision. Existing configuration revisions retain their original definition.

This slice stores and fingerprints managed definitions but does not launch the process. Process launch, LSP initialize/shutdown, document acquisition, diagnostics, and semantic requests belong to the managed-LSP slice.

## Workspace ownership

Language-service selection is workspace-scoped runtime state.

It is not owned by a session, execution, frontend, or backend. Executions may later borrow language operations through conductor-owned callables. They never receive the provider transport itself.

Live provider identity, epoch, connection identity, managed process generation, diagnostics cache, and managed process handles are process-local. Managed definitions remain configuration-owned.

## Failure rules

Selection fails when no complete provider satisfies the required contract.

A frontend disconnect invalidates every selected service using that connection and advances its epoch before another provider can become active.

A provider capability update that removes a required capability invalidates the active provider.

Unknown, stale, or wrong-connection notifications are rejected by the language-service layer. Provider replacement never makes a stale response valid.

## Scope

This slice owns:

- language-service identities and typed capabilities;
- managed provider definitions in compiled configuration;
- semantic fingerprint coverage;
- frontend-provider discovery and eligibility;
- deterministic provider selection;
- workspace-scoped active provider state;
- provider epochs and stale-request rejection;
- selection and failover regression coverage.

The next slice owns managed LSP processes, document synchronization, typed semantic operations, diagnostics state, and durable observations consumed by executions.
