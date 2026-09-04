# Blocking threaded kernel runtime

status: specification-only

Status: normative runtime contract.

## Purpose

Define the kernel and plugin-host concurrency boundary without constraining how a plugin implements its private internals.

The canonical Phenix runtime contract is synchronous and message-oriented. The kernel does not require an async executor. A plugin or runtime bridge may use async internally as an implementation choice, but executor-specific types do not cross the Plugin API.

## Boundary rule

Kernel, host, and cross-runtime plugin contracts do not require:

- `Future` or async `Stream` as ABI types;
- Tokio, async-std, or another executor as a kernel dependency;
- async mutexes or channels as required kernel mechanisms;
- executor-aware lifecycle or cancellation semantics.

Long operations use host-managed workers and typed messages at the Phenix boundary.

## Rust authoring versus runtime ABI

Rust authoring syntax and the runtime ABI are separate concerns.

A Rust plugin may use ordinary synchronous methods. An implementation may also use `async fn` internally when its dependencies or private design justify it.

If the authoring SDK accepts an async handler, generated or generic plumbing must adapt it behind the plugin boundary. No `Future`, executor handle, or async stream becomes part of interface metadata, cross-plugin dispatch, runtime-bridge protocol, lifecycle semantics, or cancellation semantics.

Canonical examples should prefer synchronous handlers when async behavior is not relevant to the example. Async syntax must never imply that all plugin runtimes need a Rust async executor.

## Concurrency model

```text
short kernel or host operation
  -> run synchronously

long or blocking I/O
  -> dedicated thread or bounded blocking worker pool
  -> typed message or result channel

CPU-bound parallel work
  -> ordinary CPU pool or Rayon when justified
```

Do not use a CPU pool for long-lived blocking network, subprocess, authentication, or plugin I/O.

## Streaming

Streaming is a sequence of typed service messages, not an async ABI type.

```text
provider worker
  -> ServiceEvent::Delta
  -> ServiceEvent::Progress
  -> ...
  -> ServiceEvent::Completed
```

The userspace interface owns message semantics. The kernel and runtime bridges transport correlated `PhenixValue` messages and provenance.

Model streams, subprocess output, ACP traffic, and similar product behavior use this generic mechanism without becoming kernel domains.

## Cancellation

Every long-running host-managed task or plugin call that supports cancellation receives an explicit cancellation capability.

Possible implementation mechanisms include an atomic flag, cancellation channel, or closing an owned process, socket, or pipe handle.

Cancellation semantics do not depend on dropping a `Future`.

The host rejects late results from cancelled scopes. Userspace interfaces may define additional domain cancellation behavior.

## Blocking operations

Blocking is expected on dedicated workers.

A blocked worker must not hold broad kernel mutable-state locks or persistence transactions unless that exact critical section requires it.

Perform slow I/O outside broad critical sections. Return normalized messages or results before committing shared kernel state.

## Process model

The normal Harness may run trusted embedded Rust plugins in the same process as the kernel.

Kernel authority checks define supported Phenix host semantics, but they cannot provide OS isolation from arbitrary native code in the same process.

Use a runtime-provider bridge when enforceable isolation is required. A process-backed bridge can provide an OS process boundary while preserving the canonical Plugin API. Runtime bridges may use any private concurrency implementation as long as executor-specific types do not cross that API.

## Dependency policy

Kernel and host code should prefer blocking-native dependencies where practical and avoid retaining an async runtime only as compatibility infrastructure.

A plugin may depend on an async runtime for its own implementation. That dependency stays private to the plugin or runtime bridge and does not become a kernel requirement.

## Invariants

- Kernel and Plugin API contracts are synchronous and message-oriented.
- The normal Harness requires no async executor.
- Plugin-private async implementation is allowed without changing the ABI.
- Long waits run on dedicated workers rather than blocking global kernel progress.
- Streaming uses typed messages rather than async ABI types.
- Cancellation is explicit and observable.
- CPU work and blocking I/O use appropriate scheduling.
- Userspace message schemas remain userspace-owned.
- One-process embedding does not move plugin semantics into Core.
- Runtime bridges expose no executor-specific types through the canonical Plugin API.

## Required regressions

- kernel and host APIs compile without an async runtime;
- the Harness starts without an async executor;
- a blocking mock provider emits incremental typed messages;
- cancellation prevents a late provider result from being accepted;
- concurrent plugin calls progress on separate workers;
- a blocked worker does not prevent unrelated kernel transitions;
- an embedded synchronous plugin and a plugin with private async internals expose the same logical interface;
- a runtime bridge can translate an async guest without exposing async ABI types;
- a process-backed runtime bridge works without an async host runtime;
- no Rust dynamic plugin loader or executor is required by the canonical plugin contract.
