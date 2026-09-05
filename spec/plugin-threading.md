# Kernel concurrency and cancellation

status: implemented
coverage:
  - rust/crates/phenix-core/src/tasks.rs
  - rust/crates/phenix-core/src/runtime/dispatch.rs
  - rust/crates/phenix-core/src/runtime/tests.rs
  - rust/crates/phenix-core/src/runtime_provider_host_regression.rs
  - rust/crates/phenix-core/tests/kernel_concurrency_contract.rs
  - rust/crates/phenix-sdk/src/authoring/static_dispatch.rs
  - rust/crates/phenix-provider-sdk/src/runtime.rs

## Purpose

Define the kernel and `PluginHost` concurrency boundary without constraining a Plugin's private implementation.

The canonical Plugin API is synchronous and message-oriented. Core does not require an async executor. A Plugin or Runtime Provider may use async internally, but executor-specific types do not cross the Plugin API.

## Boundary rule

Kernel, `PluginHost`, Interface metadata, cross-Plugin dispatch, and Runtime Provider contracts do not require:

- `Future` or async `Stream` as ABI types;
- Tokio, async-std, or another executor as a Core dependency;
- async mutexes or channels as required kernel mechanisms;
- executor-aware lifecycle or cancellation semantics.

`PluginInstance` lifecycle and invocation callbacks are synchronous. A caller may legitimately wait for short work to complete inline.

Long blocking work is explicit. Plugin code that must wait without stalling unrelated kernel work uses the kernel-owned task scope exposed by `PluginHost` or an equivalent bounded worker owned by a Runtime Provider. Core does not silently move every Plugin invocation onto a hidden executor or worker pool.

## Rust authoring versus Plugin API

Rust authoring syntax and the Plugin API are separate concerns.

A Rust Plugin may use ordinary synchronous methods. A Plugin implementation may also use `async fn` internally when its dependencies or private design justify it.

If the authoring SDK accepts an async handler, generated or generic plumbing adapts it behind the synchronous Plugin API boundary. No `Future`, executor handle, or async stream becomes part of Interface metadata, cross-Plugin dispatch, Runtime Provider protocol, lifecycle semantics, or cancellation semantics.

Private implementations may own an executor. For example, the Provider SDK may block its synchronous Plugin callback on a private Tokio runtime. That executor remains an implementation detail of that Plugin crate.

Canonical examples should prefer synchronous handlers when async behavior is not relevant.

## Concurrency model

```text
short kernel or Plugin operation
  -> synchronous call

long or blocking Plugin work
  -> PluginHost task scope or Runtime Provider worker
  -> dedicated thread or bounded blocking worker pool
  -> typed result or Interface message

CPU-bound parallel work
  -> ordinary CPU pool or Rayon when justified
```

The kernel task scope pins:

- owning Plugin when spawned through `PluginHost`;
- Graph Generation;
- attenuated authority;
- explicit cancellation state.

Do not use a CPU work-stealing pool as the normal home for long-lived blocking network, subprocess, authentication, or Plugin I/O.

## Streaming

Streaming is a sequence of typed userspace messages, not an async ABI type.

Model output, subprocess output, ACP traffic, and similar product behavior define their own correlated event or message contracts. Core transports normal typed values and provenance; it does not define a universal async stream primitive.

## Cancellation

Cancellation is explicit and observable.

A synchronous Plugin invocation receives a kernel-owned live-call cancellation token through `PluginHost`. A spawned kernel task receives its own cancellation token carrying the task's Graph Generation and attenuated authority.

Stopping or replacing a Plugin generation cancels its matching live calls and owned tasks. After a Plugin callback returns, kernel dispatch checks the live-call token before accepting the result. A result returned after cancellation is rejected as a cancelled invocation rather than entering canonical state.

Task cancellation uses an atomic cancellation flag and emits the ordinary kernel task-cancelled event. Cancellation does not depend on dropping a `Future`.

Userspace Interfaces may define additional domain cancellation behavior.

## Blocking operations and locks

A blocked worker must not retain broad kernel mutable-state locks or kernel persistence transactions unless that exact critical section requires it.

`TaskRuntime` workers receive cloned task state and communicate through explicit result channels. The kernel does not hold the Plugin instance lock while a task-scope worker waits. Unrelated kernel transitions therefore remain able to progress while such a worker is blocked.

Embedded native Plugin code remains trusted in-process code. Core cannot prevent arbitrary native code from blocking its own caller thread or taking private locks. Long blocking work that must not stall the caller must use the explicit task/worker boundary.

## Runtime Providers

Runtime Providers expose the same synchronous Plugin API to Core and may use any private concurrency implementation behind it.

A Runtime Provider may translate cancellation into a correlated guest request, process termination, socket closure, or another runtime-specific mechanism. Executor types remain private to the bridge.

Process-backed runtime behavior, transport correlation, and isolation are owned by `spec/plugin-process-runtime-bridge.md`; those additive features are not prerequisites for the executor-independent Core contract.

## Dependency policy

Core and `PluginHost` use blocking-native standard synchronization and do not depend on Tokio or another async runtime.

A Plugin, Provider implementation, adapter, or Runtime Provider may depend on an async executor when its private implementation requires one. That dependency must not escape into canonical Plugin contracts.

## Invariants

- Kernel and Plugin API contracts are synchronous and message-oriented.
- Core requires no async executor.
- Plugin-private async implementation is allowed without changing the Plugin API.
- Long blocking work that must not stall unrelated kernel progress uses an explicit task or worker boundary.
- Streaming uses typed userspace messages rather than async ABI types.
- Cancellation is explicit and observable.
- Late results from cancelled live-call scopes are rejected.
- Task authority and Graph Generation are pinned when work is spawned.
- CPU work and blocking I/O use appropriate scheduling mechanisms.
- Runtime Provider bridges expose no executor-specific types through the canonical Plugin API.

## Regression coverage

The repository verifies that:

- task cancellation is explicit, authority-attenuated, and Graph Generation-scoped;
- live-call cancellation is owner- and generation-scoped;
- service dispatch rejects a result after its live-call token is cancelled;
- Runtime Provider hosts receive the same explicit cancellation boundary;
- a blocked `PluginHost` task does not prevent an unrelated kernel transition;
- Rust authoring may adapt private async handlers behind the synchronous Plugin API;
- the Provider SDK may use a private Tokio runtime without exposing it through Plugin contracts.

## Component invocation and state ownership

Proposed follow-up; implementation and regressions are pending. The baseline status above does not certify this boundary.

Component resolution accepts acyclic imports between components owned by one plugin, but dispatch rejects the repeated PluginId. The generated adapter also holds a whole-plugin mutable lock across outbound calls.

1. Keep PluginId as the authority, lifecycle, resource-ownership, and replacement boundary. Track causal invocation entries by generation, component, operation, and layer participant. Permit distinct endpoints in the same plugin; reject actual endpoint recursion before acquiring handler state. Layer delegation remains the existing checked continuation.

2. Use shared-receiver invocation endpoints and immutable dispatch tables. Generated plugin objects expose callable behavior through shared receivers; mutable domain state uses explicit, narrowly scoped state or resource handles. Initialization and stop obtain exclusive access only after work has quiesced.

3. Migrate generated handlers and first-party implementations off the implicit whole-root Mutex. Root structs and nested fields remain the authoring model; direct mutable root borrowing is reserved for local/lifecycle code. Emit a precise macro diagnostic for an exported mutable receiver that requires the removed whole-root lock.

4. An outbound interface call uses canonical resolution, layers, authority attenuation, generation pinning, cancellation, and provenance regardless of packaging. Plugin code releases private mutable state guards before outbound calls; the SDK must not retain a generated lock across them.

5. Keep the synchronous executor-independent ABI. Document that generated async handlers synchronously await completion; show long I/O on the existing task boundary with a correlated completion. Do not introduce hidden per-call threads or require an async executor.

Acceptance requires:

- A -> B through typed imports succeeds when both components share one plugin, including a root-to-child call.
- A -> B -> C may revisit a plugin at a different endpoint; A -> B -> A rejects true recursion without hanging.
- Same-plugin calls retain required layers, authority attenuation, cancellation, and generation provenance.
- Compile diagnostics and migrated examples make state ownership explicit; a blocked task does not stall unrelated kernel work.
