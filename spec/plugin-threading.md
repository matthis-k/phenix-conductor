# Blocking threaded kernel runtime

status: specification-only

## Purpose

Define the kernel and `PluginHost` concurrency boundary without constraining a Plugin's private implementation.

The canonical Plugin API is synchronous and message-oriented. The kernel does not require an async executor. A Plugin or Runtime Provider bridge may use async internally as an implementation choice, but executor-specific types do not cross the Plugin API.

## Boundary rule

Kernel, `PluginHost`, and bridged Plugin contracts do not require:

- `Future` or async `Stream` as ABI types;
- Tokio, async-std, or another executor as a kernel dependency;
- async mutexes or channels as required kernel mechanisms;
- executor-aware lifecycle or cancellation semantics.

Long operations use `PluginHost`-managed workers and typed Interface messages at the Plugin API boundary.

## Rust authoring versus Plugin API

Rust authoring syntax and the Plugin API are separate concerns.

A Rust Plugin may use ordinary synchronous methods. A Plugin implementation may also use `async fn` internally when its dependencies or private design justify it.

If the authoring SDK accepts an async handler, generated or generic plumbing must adapt it behind the Plugin API boundary. No `Future`, executor handle, or async stream becomes part of Interface metadata, cross-Plugin dispatch, Runtime Provider protocol, lifecycle semantics, or cancellation semantics.

Canonical examples should prefer synchronous handlers when async behavior is not relevant to the example. Async syntax must never imply that every Execution Runtime needs a Rust async executor.

## Concurrency model

```text
short kernel or Host Capability operation
  -> run synchronously

long or blocking I/O
  -> dedicated thread or bounded blocking worker pool
  -> typed Interface message or result channel

CPU-bound parallel work
  -> ordinary CPU pool or Rayon when justified
```

Do not use a CPU pool for long-lived blocking network, subprocess, authentication, or Plugin I/O.

## Streaming

Streaming is a sequence of typed Interface messages, not an async ABI type.

```text
Provider worker
  -> ServiceEvent::Delta
  -> ServiceEvent::Progress
  -> ...
  -> ServiceEvent::Completed
```

The userspace Interface owns message semantics. The kernel and Runtime Provider bridges transport correlated `PhenixValue` messages and provenance.

Model streams, subprocess output, ACP traffic, and similar product behavior use this generic mechanism without becoming kernel domains.

## Cancellation

Every long-running `PluginHost`-managed task or Plugin call that supports cancellation receives an explicit cancellation capability.

Possible implementation mechanisms include an atomic flag, cancellation channel, or closing an owned process, socket, or pipe handle.

Cancellation semantics do not depend on dropping a `Future`.

The kernel rejects late results from cancelled live-call scopes. Userspace Interfaces may define additional domain cancellation behavior.

## Blocking operations

Blocking is expected on dedicated workers.

A blocked worker must not hold broad kernel mutable-state locks or kernel persistence transactions unless that exact critical section requires it.

Perform slow I/O outside broad critical sections. Return normalized messages or results before committing shared kernel state.

## Process model

The normal Harness may run trusted embedded Rust Plugins in the same process as the kernel.

Kernel authority checks define supported `PluginHost` semantics, but they cannot provide OS isolation from arbitrary native code in the same process.

Use a Runtime Provider bridge when enforceable isolation is required. A process-backed Runtime Provider bridge can provide an OS process boundary while preserving the canonical Plugin API. Runtime Providers may use any private concurrency implementation as long as executor-specific types do not cross that API.

## Dependency policy

Kernel and `PluginHost` code should prefer blocking-native dependencies where practical and avoid retaining an async executor only as compatibility infrastructure.

A Plugin may depend on an async executor for its own implementation. That dependency stays private to the Plugin or Runtime Provider and does not become a kernel requirement.

## Invariants

- Kernel and Plugin API contracts are synchronous and message-oriented.
- The normal Harness requires no async executor.
- Plugin-private async implementation is allowed without changing the Plugin API.
- Long waits run on dedicated workers rather than blocking global kernel progress.
- Streaming uses typed Interface messages rather than async ABI types.
- Cancellation is explicit and observable.
- CPU work and blocking I/O use appropriate scheduling.
- Userspace Interface schemas remain userspace-owned.
- One-process embedding does not move Plugin semantics into Core.
- Runtime Provider bridges expose no executor-specific types through the canonical Plugin API.

## Required regressions

- kernel and `PluginHost` APIs compile without an async executor;
- the Harness starts without an async executor;
- a blocking mock Provider emits incremental typed messages;
- cancellation prevents a late Provider result from being accepted;
- concurrent Plugin calls progress on separate workers;
- a blocked worker does not prevent unrelated kernel transitions;
- an embedded synchronous Plugin and a Plugin with private async internals expose the same logical Interface;
- a Runtime Provider bridge can translate an async guest without exposing async ABI types;
- a process-backed Runtime Provider bridge works without an async Host runtime;
- no Rust dynamic Plugin loader or executor is required by the canonical Plugin API.
