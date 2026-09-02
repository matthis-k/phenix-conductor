# Plugin host

status: specification-only

Status: implementation contract.

## Purpose

Define the one runtime boundary through which executable plugin code receives kernel capabilities and returns plugin results.

Hosting mode changes transport and isolation. It does not change plugin identity, interface semantics, authority, lifecycle, graph resolution, persistence ownership, or provenance.

This document extends `spec/plugin-authoring-macro.md`, `spec/plugin-contributions.md`, `spec/plugin-resolution.md`, and `spec/plugin-runtime-bridges.md`.

## Roles

**Plugin authoring API.** Lets an author declare plugin state, behavior, dependencies, resources, and semantic metadata.

**Core descriptor.** The generated or runtime-derived data used by the resolver. Static plugin authors do not maintain this by hand.

**PluginHost.** The kernel-owned execution boundary for one executable plugin instance.

**Runtime provider.** A plugin that maps another execution environment onto the canonical Plugin API.

**Host capability.** An authority-bearing handle for a kernel- or environment-owned operation such as filesystem, process, network, clock, credentials, terminal, or frontend callback access.

These roles are distinct. A runtime provider translates execution. It does not gain composition authority. A host capability grants one bounded operation. It does not expose mutable kernel internals.

## Kernel ownership

The kernel owns:

- plugin instance identity and graph generation;
- lifecycle state and transitions;
- authority grants and attenuation;
- host capability construction;
- interface dispatch through the resolved graph;
- event delivery;
- controller and task scheduling;
- cancellation and live-call tracking;
- normalized host and bridge failures;
- provenance.

A plugin never receives mutable access to the kernel runtime, raw registries, persistence backend handles, or database connections.

## Hosting modes

The canonical model is:

```text
Plugin API
  |
  +-- embedded runtime
  |     `-- generated or generic Rust PluginInstance adapter
  |
  +-- runtime-provider plugin
  |     `-- guest plugin in WASM, TypeScript, Python, process, remote, ...
  |
  `-- resource-only plugin
        `-- no executable instance
```

Core initially supplies only `embedded`. Other runtimes are ordinary runtime-provider plugins as defined in `spec/plugin-runtime-bridges.md`.

A runtime provider may use a process transport internally. "External plugin" is therefore an execution arrangement, not a second semantic plugin model.

Rust dynamic libraries are not an implicit plugin hosting mode.

## Lifecycle

Executable instances move through kernel-owned lifecycle states such as:

```text
starting
ready
degraded
unavailable
stopping
stopped
```

Each active instance belongs to one graph generation and artifact revision. Process-local handles belong to that generation and are never durable state.

Resource-only plugins have no executable lifecycle callbacks. Their declarations still participate in graph construction and resource activation.

## Host capabilities

Executable plugins receive only host capabilities granted by the resolved authority policy.

Examples include:

- invoking an imported interface through kernel dispatch;
- reading or mutating an authorized durable resource;
- emitting an event;
- inspecting permitted runtime metadata;
- requesting a bounded controller or task operation;
- filesystem access;
- process execution;
- network access;
- clock access;
- credential access;
- terminal or frontend callbacks.

Product-domain operations such as sessions, models, tools, skills, context, memory, orchestration, or repository behavior are not special `PluginHost` methods. Plugins use the corresponding neutral interface contracts through ordinary imports.

Every host operation rechecks effective authority. Holding one host capability does not imply ambient access to another.

## Invocation

The resolved graph chooses the provider and layer chain before execution.

Invocation follows this shape:

```text
pin graph generation
  -> establish effective authority
  -> create live call scope
  -> enter resolved layer/provider chain
  -> adapt through runtime provider when required
  -> normalize result or error to PhenixValue
  -> record provenance
  -> close live call scope
```

Provider code cannot execute before compatibility, graph binding, and authority checks complete.

Callbacks from a plugin re-enter ordinary kernel dispatch and authority enforcement.

## Runtime providers

A runtime provider receives authority for its own bridge implementation and a separate attenuated guest host for the guest plugin.

Bridge authority never becomes guest authority.

The bridge may translate:

- artifacts;
- lifecycle calls;
- `PhenixValue` to native values and back;
- guest capability handles;
- private concurrency and error models.

It may not redefine plugin identity, interfaces, authority, persistence ownership, graph semantics, lifecycle semantics, or provenance.

## Cancellation

Each in-flight executable call has a kernel-owned live scope.

Cancellation may:

- prevent undispatched work from starting;
- signal an embedded worker through an explicit cancellation handle;
- send correlated cancellation to a runtime provider;
- terminate a guest process or runtime instance when policy permits hard cancellation;
- reject late results from cancelled scopes.

Userspace interfaces may define additional domain cancellation semantics. The host boundary only guarantees the generic runtime behavior.

## Errors

Kernel-facing host failures distinguish at least:

- provider unavailable;
- authority denied;
- invalid structural request or response;
- provider execution failure;
- runtime-provider or protocol failure;
- cancelled;
- host capability denied;
- guest crashed or disconnected.

Userspace interfaces may define richer typed domain failures.

## First-party and third-party equality

A first-party runtime plugin receives no private host method, implicit authority, provider priority, persistence privilege, or lifecycle path unavailable to a compatible third-party implementation.

If a first-party implementation requires a capability that an equivalent third-party plugin cannot request through the canonical Plugin API, the architecture is incomplete.

## Invariants

- `PluginHost` is the one executable plugin boundary.
- Hosting mode changes transport and isolation, not semantics.
- Static authoring does not require hand-written factories or host registration.
- Resource-only plugins need no fake executable instance.
- Host capabilities remain generic and authority-bearing.
- Product-domain behavior uses ordinary neutral interfaces rather than private host methods.
- Plugin callbacks cannot bypass graph resolution, authority, or resource ownership.
- Runtime-provider authority is separate from guest authority.
- Process-local handles are generation-bound and never reconstructed as durable state.
- First-party plugins receive no private privileged path.

## Required regressions

- embedded and bridged providers expose the same logical interface semantics;
- provider code cannot execute before authority enforcement;
- an ungranted host capability is rejected;
- a plugin cannot obtain mutable runtime, registry, store-backend, or database handles;
- an alternate third-party provider can request every supported host capability needed by an equivalent first-party provider;
- product concepts are absent from the generic host API;
- callbacks re-enter normal graph and authority checks;
- cancellation and crash/disconnect always close live-call scopes;
- runtime-provider authority cannot leak into guest authority;
- a resource-only plugin activates without an executable runtime;
- static Rust plugin authors do not maintain manual `PluginFactory` wiring.
