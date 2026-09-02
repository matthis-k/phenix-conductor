# Blocking threaded kernel runtime

status: implemented

## Purpose

Define synchronous Rust and OS threads as the first-party kernel/plugin-host concurrency model.

The normal Harness may run the kernel and trusted embedded userspace plugins in one process. Long operations block worker threads and report progress through typed messages. Async Rust is not required by the kernel or plugin contract.

## Runtime rule

First-party kernel and plugin-host contracts must not require:

- `async fn` or `.await`;
- `Future` or async `Stream` interfaces;
- Tokio, async-std, or another executor;
- async mutexes/channels as required kernel mechanisms;
- executor-aware lifecycle or cancellation semantics.

First-party userspace plugins should follow the same synchronous model unless a concrete implementation has an internal reason not to. External plugins may use any internal model, but async types never cross the plugin protocol.

## Concurrency model

```text
short kernel/host operation
  -> run synchronously

long/blocking I/O
  -> dedicated thread or bounded blocking worker pool
  -> typed message/result channel

CPU-bound parallel work
  -> ordinary CPU pool or Rayon when justified
```

Do not use Rayon as the pool for long-lived blocking network, subprocess, authentication, or plugin I/O.

## Streaming

Streaming is a sequence of typed service messages, not an async Rust type.

Example:

```text
userspace provider worker
  -> blocking read
  -> ServiceEvent::Delta
  -> blocking read
  -> ServiceEvent::Progress
  -> ...
  -> ServiceEvent::Completed
```

The event schema belongs to the userspace service. The kernel/plugin host only transports correlated messages.

Model streams, subprocess output, ACP traffic, and similar Phenix features are implemented by userspace plugins over this generic threaded/message mechanism.

## Cancellation

Every long-running kernel task or plugin call that requires cancellation receives an explicit cancellation handle.

Possible mechanisms include an atomic flag, cancellation channel, or closing an owned process/socket/pipe handle.

Cancellation does not depend on dropping a `Future`.

Late results from cancelled kernel scopes cannot be accepted by the host. Userspace services define any additional domain cancellation semantics.

## Blocking operations

Blocking is expected on worker threads.

Examples include persistence calls, external plugin transport, embedded plugin calls, and generic IPC.

Model providers, authentication, ACP adapters, repository commands, and other product operations are userspace examples. Their presence does not make those domains kernel concerns.

A blocked worker must not hold broad kernel mutable-state locks or persistence transactions unless that exact critical section requires it.

## State ownership

Prefer short synchronous critical sections or single-owner transitions. Perform slow I/O outside broad critical sections and return normalized results/messages before committing durable state.

Do not introduce async locks to compensate for unclear ownership.

## Process model

The normal Harness may run the kernel and trusted embedded plugin factories in one process.

Embedded native code is trusted from an OS-isolation perspective. Kernel authority checks still define Phenix host semantics, but they cannot sandbox arbitrary memory access in the same process.

External subprocess hosting is used when independent distribution or enforceable isolation is required. Its transport also uses blocking reads/writes on dedicated threads.

## Dependency policy

During migration:

1. identify direct async-runtime dependencies;
2. identify the actual concurrency each path requires;
3. replace ordinary waiting with blocking worker-thread code;
4. replace async streams with typed message channels;
5. replace future cancellation with explicit cancellation handles;
6. prefer blocking-native dependencies where practical;
7. remove async-runtime dependencies once no first-party kernel/host path requires them.

Do not keep async infrastructure solely as a compatibility path.

## Invariants

- Kernel/plugin-host contracts are synchronous and message-oriented.
- The normal Harness does not require an async executor.
- Long waits block workers, not global kernel progress.
- Streaming does not imply async Rust.
- Cancellation is explicit and observable.
- CPU parallelism and blocking I/O use appropriate separate scheduling.
- Userspace event schemas remain userspace-owned.
- One-process embedding does not move plugin semantics into the kernel.
- Plugin contracts expose no executor-specific types.

## Required regressions

- kernel and host API require no async runtime;
- Harness starts without an async executor;
- blocking mock provider emits incremental typed messages;
- cancellation prevents a late provider result from being accepted;
- concurrent plugin calls progress on separate workers;
- blocked worker does not prevent unrelated kernel state transitions;
- embedded invocation uses host-managed worker execution;
- external transport works without an async host runtime;
- no Rust dynamic plugin loader is required.