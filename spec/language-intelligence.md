# Language intelligence

## Purpose

Expose language intelligence as conductor-owned semantic operations. The conductor may use a frontend-linked provider or launch the managed provider selected by the workspace language-service contract.

This slice owns managed LSP process lifetime, document synchronization, semantic execution operations, diagnostics state, and durable observations for results consumed by executions.

## Managed provider lifetime

A managed provider is one workspace-scoped process generation. The conductor launches it from the pinned `ManagedLanguageProviderDefinition`, initializes LSP over stdio, and keeps the process alive across executions.

Process exit, protocol failure, explicit restart, configuration replacement, or workspace shutdown ends the generation. Selection then receives a new live generation. The workspace language-service manager creates a new provider epoch.

The conductor sends `shutdown` and `exit` during normal teardown. Abnormal exit invalidates outstanding requests.

## Documents

Agents may acquire a workspace document without opening frontend UI.

Managed providers synchronize workspace-backed documents with LSP `didOpen`, `didChange`, and `didClose`. The conductor derives the content from the workspace and records the exact `FileVersion` used for a consumed result.

Frontend-linked providers may expose unsaved editor state only when they advertise `dirty_buffers`. Every consumed result records document provenance:

```text
workspace_backed
frontend_unsaved
mixed_or_unknown
```

Workspace verification treats only `workspace_backed` results with exact workspace versions as authoritative file evidence.

## Operations

Executions receive typed conductor operations:

```text
definition
references
implementations
hover
document_symbols
workspace_symbols
diagnostics
call_hierarchy
```

The conductor translates each operation to provider-specific LSP or frontend-service traffic. Executions never receive raw provider transport or arbitrary LSP method access.

The typed operation identity is durable. Provider wire payloads are implementation detail.

## Authority

Language operations are normal conductor callables. Configuration and execution delegation decide whether an execution can invoke them.

Provider availability does not add callable authority, filesystem authority, frontend IPC authority, or network authority.

Document acquisition for an execution may read only workspace paths that execution could otherwise read through the conductor workspace boundary.

## Diagnostics

The workspace language service keeps current diagnostics as process-local shared state. Managed LSP stdout is consumed continuously so `publishDiagnostics` notifications update that cache independently of execution requests. A frontend-linked provider normalizes its shared diagnostic state into a `language.diagnostics` frontend-service notification whose params are a `LanguageOperationResult`. The conductor accepts that notification only from the selected provider connection and epoch. Diagnostic notifications do not become durable history by themselves.

An execution may request current diagnostics. A frontend snapshot is used only while its provider epoch remains active. When a diagnostic result influences model execution, the conductor records the exact consumed result as a language observation.

## Consumed observations

A consumed language result becomes an immutable `LanguageObservation` containing:

```text
consuming execution
workspace
service kind
provider identity
provider epoch
typed operation
result
document identities and provenance
exact workspace FileVersion where available
```

The execution identity is mandatory. It binds the observation to the authority and history of the execution that actually consumed the result.

The observation records what the execution actually consumed, not the provider's whole diagnostics cache.

If the provider epoch ends before a request completes, the result is rejected and no successful observation is recorded.

## Failure rules

- Provider loss during a request returns `ProviderChanged`; the conductor does not replay the request against another provider.
- Malformed provider responses fail the operation.
- Managed process failure ends the process generation and invalidates its epoch.
- Frontend notifications must match the selected provider connection and epoch.
- A provider result that depends on unsaved frontend content keeps that provenance when persisted.

## Scope

This slice owns:

- typed language operations and result provenance;
- managed LSP stdio process management and initialization;
- workspace document acquisition and synchronization;
- frontend-linked and managed operation dispatch behind one conductor boundary;
- workspace diagnostics cache;
- execution-visible semantic callables with normal delegation checks;
- durable observations for consumed results;
- provider-loss, dirty-buffer, background-document, and restart regressions.

Context retrieval over these observations belongs to the later common context and history slices.
