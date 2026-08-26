# Plugin host

Status: implementation contract.

## Purpose

Provide one canonical runtime host for embedded, external, and resource-only plugins. Hosting mode changes transport and isolation, not authority, lifecycle, provider semantics, or provenance.

Requires `spec/plugins.md`, `spec/plugin-contributions.md`, and `spec/plugin-resolution.md`.

## Ownership

`PluginHost` is the only runtime boundary through which executable plugin code receives kernel mechanisms or returns plugin results.

The kernel owns:

- plugin instance identity and runtime generation;
- start/stop/health lifecycle;
- permission-bound host handles;
- generic service/capability dispatch;
- generic event subscription dispatch;
- contribution registration;
- cancellation and live-call tracking;
- normalized host/protocol failures;
- kernel-operation provenance.

A plugin never receives mutable access to kernel runtime internals, persistence backend handles, raw registries, or SQLite connections.

## Hosting modes

```text
Plugin contract
  |
  +-- EmbeddedPluginAdapter -> statically linked Rust PluginFactory
  +-- ExternalPluginAdapter -> versioned blocking process transport
  `-- ResourcePluginAdapter -> manifest + static resources
```

First-party, third-party, embedded, external, and resource-only are independent classifications.

Cross-process contracts use transport-safe typed values. No logical plugin contract may require Rust object identity, borrowed kernel references, or a dynamic Rust ABI.

## Lifecycle

Executable runtime states:

```text
registered
starting
ready
degraded
unavailable
stopping
stopped
```

Each activation creates a runtime generation/epoch. Process-local handles belong to that generation and are never treated as durable state.

Resource-only plugins register static contributions without an executable state machine.

## Host services

An executable plugin receives a permission-bound `PluginHostHandle` exposing a deliberately small set of generic kernel operations, for example:

- invoke a permitted service/capability provider;
- perform a permitted durable-data operation in an authorized namespace;
- emit or subscribe to permitted generic events;
- inspect allowed kernel/plugin runtime metadata;
- request another bounded kernel task;
- emit normalized diagnostics/results.

The host API must not expose Phenix session, artifact, context, skill, tool, callable, orchestration, model, repository, or other product-specific operations.

A userspace plugin that needs those concepts calls the corresponding userspace service contract through generic provider dispatch.

Every host operation rechecks effective authority. Possessing a host handle does not imply ambient access.

## Capability invocation

Invocation order:

```text
resolve provider
  -> establish effective authority
  -> create live call scope
  -> dispatch provider through selected adapter
  -> normalize result/error
  -> attach provider provenance
  -> close live call scope
```

Provider code cannot run before resolution and authority enforcement.

Callbacks through `PluginHostHandle` re-enter the same kernel mechanisms and authority checks as other plugin calls.

## Cancellation

Each in-flight executable plugin call has a process-local live scope.

Cancellation:

- prevents undispatched work from starting;
- signals running embedded workers through explicit cancellation handles;
- sends correlated cancellation or terminates an external generation when policy requires hard cancellation;
- closes the live scope on every terminal path;
- rejects late results from cancelled scopes.

The kernel does not impose userspace cancellation semantics beyond this host-call boundary.

## Errors

Kernel-facing host failures distinguish at least:

- unavailable provider;
- permission denied;
- invalid request/response;
- provider execution failure;
- protocol/contract mismatch;
- cancelled;
- host operation denied;
- provider crashed/disconnected.

Userspace services may define richer domain failures in their own contracts.

## Product composition

A Phenix Plugin Suite component is an ordinary plugin enabled by Harness policy. It receives no host API, permission, priority, or durable privilege unavailable to a compatible alternate implementation.

Removing a first-party plugin from configuration removes its new-call contributions without kernel changes. Embedded code may remain linked but inactive.

## Invariants

- One `PluginHost` boundary owns executable plugin interaction.
- Embedded and external plugins use the same logical service, authority, lifecycle, and provenance contracts.
- Resource-only plugins need no fake executable.
- Host APIs remain domain-neutral.
- Plugin callbacks cannot bypass authority or persistence namespace checks.
- Runtime generation changes on activation/restart.
- Process-local handles are never reconstructed as durable state.
- Every executable call records provider identity/generation and kernel policy provenance.
- First-party plugins receive no private privileged host path.
- Rust dynamic libraries are not a plugin hosting mode.

## Required regressions

- embedded and external providers invoke through the same logical host contract;
- provider cannot execute before authority enforcement;
- host handle rejects an ungranted operation;
- plugin cannot obtain mutable runtime/store/backend handles;
- alternate provider can use every host mechanism required by an equivalent first-party provider;
- host interface contains no agent-domain service method;
- cancellation and panic/disconnect always remove live-call scopes;
- resource-only plugin registers without executable runtime.