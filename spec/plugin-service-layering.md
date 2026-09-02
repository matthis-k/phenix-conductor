# Layered service invocation

status: implemented

## Purpose

Allow plugins to extend or override a service without replacing its complete implementation.

A configured service call resolves an ordered set of layers plus one terminal provider. A layer may delegate to the next participant, return a result itself, or deny the call. Plugins that do not declare the requested contract and version do not participate.

This extends `spec/plugin-resolution.md` and `spec/plugin-contributions.md`. Provider fallback keeps its existing safety rules.

## Core and plugin boundary

`phenix-core` owns fundamental primitives and simple mechanisms. Plugins own non-trivial behavior, policy, discovery, management, composition, and product semantics.

A concept may be agent-specific and still belong in core when it is a primitive that most plausible plugins would otherwise need to reimplement or depend on another plugin to use. Core should expose the smallest useful representation and mechanism for that primitive, not the richer behavior built on top of it.

Examples of core primitives and mechanisms include:

```text
plugin identity, lifecycle, registration, and dependency mechanics
service contracts, terminal resolution, layering, and continuations
authority attenuation and trust-boundary enforcement
events, tasks, cancellation, persistence namespaces, transactions, and provenance
minimal model request/stream/tool-call primitives
minimal tool identity/schema/registration/invocation primitives
minimal skill identity/content/registration/injection primitives
minimal session identity/lifecycle/input/output primitives
minimal context attachment/injection primitives
```

Examples of plugin-owned behavior include:

```text
model providers, authentication, routing, budgets, fallback policy, and model catalogs
tool suites, shell/filesystem/MCP tools, discovery, and tool-set policy
skill discovery, catalogs, search, ranking, activation, precedence, and version management
session trees, branching policy, summaries, history search, and rich metadata
context discovery, ranking, compaction, dependency expansion, and repository context
orchestration, workers, planning, objectives, decisions, artifacts, hooks, jobs, and diagnostics
```

The decision rule is:

> Put a mechanism in core when most plausible plugins would otherwise need the same small primitive or mechanism. Keep non-trivial behavior and policy in plugins.

Do not move behavior into core only because several first-party plugins use it. A shared first-party library or service may still be the correct owner when the behavior is non-trivial or domain-specific.

For hooks specifically, core owns only the generic mechanisms needed to implement interception: events, ordered service layers, continuations, cancellation, authority, and provenance. Configurable hook definitions, conditions, actions, pre/post domain semantics, persistence, and user-facing hook management belong to `phenix-plugin-hooks`. The hooks plugin receives no privileged registration or execution path.

## Terms

**Terminal provider.** The selected implementation at the bottom of a service chain. Provider resolution selects one terminal provider before dispatch.

**Layer.** A plugin contribution that runs before the terminal provider. Higher configured layer priority runs earlier. Equal priority uses stable plugin identity.

**Continuation.** A kernel-issued, one-shot handle for invoking the next participant in the already resolved chain.

**Skip.** A contribution is absent from the chain because it is disabled, unavailable, unauthorized, out of scope, or incompatible with the requested contract/version. Skipped code is never invoked.

## Contribution roles

A service contribution declares one role:

```text
terminal
layer
```

The kernel keeps provider selection and layer ordering separate.

A plugin may contribute only the service contracts and versions it understands. A plugin that supports `phenix.sessions@2` but not `phenix.sessions@3` is skipped for an `@3` invocation. The `@3` chain continues through compatible layers to its selected terminal provider.

A plugin must not contribute the same service and version as both a layer and a terminal provider.

Resource-only plugins cannot contribute either executable role.

## Chain resolution

For one request the kernel:

1. pins the kernel and Harness policy snapshot;
2. resolves eligible layers for the exact compatible service contract/version;
3. orders layers by configured policy, then stable plugin identity;
4. resolves one eligible terminal provider through the existing provider resolver;
5. records the complete planned chain;
6. dispatches to the first layer, or directly to the terminal provider when no layers exist.

A valid chain requires a terminal provider even when a layer may handle a request without delegating. This keeps service availability decidable before plugin code runs.

Explicit terminal-provider binding selects the terminal provider. It does not remove configured layers. Harness policy may disable or bind layers separately.

Plugin registration or activation order never defines layer order.

## Layer behavior

A layer receives the normal service request plus a continuation bound to the remaining chain.

A layer has three semantic choices:

```text
handle    return a service result without invoking the continuation
delegate  invoke the continuation once and return or transform its result
deny      return an explicit denial without invoking the continuation
```

A layer that only wants pass-through behavior delegates and returns the result unchanged.

The continuation is single-use. A second call fails before another participant is invoked.

A continuation is valid only for its originating invocation, service contract/version, pinned policy, authority bound, and remaining chain position. It cannot be stored and reused by a later request.

A plugin cannot emulate continuation by recursively invoking the same service. Same-service causal re-entry remains denied. Only the kernel-issued continuation advances the current chain.

## Failure semantics

Failure and denial stop the chain.

A plugin error never means "try the next provider." This preserves the existing rule against unsafe post-dispatch provider switching for mutating or ambiguity-sensitive operations.

Unsupported contract/version behavior is represented by absence from the resolved chain, not by invoking a plugin and interpreting an error as pass-through.

A contract may define typed domain failures. Those remain results or errors of that contract and do not alter chain resolution unless the contract explicitly defines safe replay semantics.

## Authority

Each participant runs under the intersection of:

```text
caller authority
configured plugin grant
participant maximum authority
service operation requirements
```

A layer may attenuate authority before delegating. It cannot increase authority for the continuation or any later participant.

The kernel binds the continuation to the resulting authority. Retrying, wrapping, handling, denial, or delegation cannot regain removed authority.

## State and transactions

Layering does not make plugin state shared.

Each plugin keeps its own durable namespaces and schemas. A layer may attach state to an opaque domain identity through its own schema or an explicit shared contract.

When one user-visible mutation requires state changes across layers and the terminal provider to commit atomically, the owning service contract must use the existing kernel transaction mechanism. Layer ordering alone does not provide rollback for arbitrary side effects.

A layer must perform irreversible external side effects only under semantics declared by the service contract. The kernel does not retry a partially executed chain by default.

## Provenance

The kernel records both the planned chain and the executed path.

At minimum:

```text
service contract/version
pinned policy identity
caller/effective authority bound
ordered eligible layers
selected terminal provider
participants actually entered
for each entered layer: handled, delegated, denied, or failed
terminal invocation status when reached
```

A layer that delegates remains part of provenance even when it leaves the result unchanged.

## External plugins

External plugins receive the same semantics as embedded plugins.

The local plugin protocol represents a continuation as an opaque invocation-scoped token. A host operation advances that token once. The host validates service identity, policy identity, authority, invocation identity, and chain position before dispatching the next participant.

An external plugin crash or protocol failure stops the chain. It does not select another terminal provider.

## Session composition

Sessions have a minimal core primitive and richer userspace semantics.

Core may define stable session identity and the smallest lifecycle/input/output operations needed by most plugins. It must not absorb tree semantics, branching policy, summaries, search, rich metadata, or other non-trivial session behavior.

Session-related plugins add behavior in two ways:

```text
layer an existing session operation
add a separate session-related contract
```

Example:

```text
phenix-session-policy      layer phenix.sessions@1
phenix-session-tree        layer phenix.sessions@1 where child creation needs lineage
phenix-session-tree        provide phenix.session-tree@1
phenix-sessions            terminal phenix.sessions@1 when a replaceable richer provider is configured
```

A client that only understands `phenix.sessions@1` keeps working. A client that understands tree behavior may also call `phenix.session-tree@1`.

The tree plugin stores lineage in its own durable namespace keyed by the session identity defined by the session contract. It does not change the base session wire or Rust type.

If the tree plugin does not support a newer session contract version, it is skipped for that version. The compatible terminal session provider still handles the request unless Harness policy requires the tree layer.

Harness policy may mark a layer as required for a service/version. Startup or resolution fails when a required layer is unavailable or incompatible instead of silently degrading semantics.

## Configuration

Harness policy owns effective composition. It may configure:

```text
terminal provider binding and priority
layer enabled/disabled state
layer priority/order
required versus optional layer status
layer-specific authority grants and scope selectors
```

Plugins advertise roles and compatibility. They do not grant themselves effective priority, required status, or authority.

The resolved chain is part of the pinned configuration identity.

## Invariants

- Core owns fundamental primitives and simple mechanisms; plugins own non-trivial behavior and policy.
- Core may contain agent-specific primitives when most plausible plugins need them.
- Shared first-party use alone does not justify moving non-trivial behavior into core.
- Provider replacement and service layering are distinct mechanisms.
- Every valid chain has exactly one terminal provider.
- Layers fall through only by explicit continuation.
- Contract/version incompatibility skips before dispatch.
- Failure never means fallback.
- Denial is explicit and stops the chain.
- Continuations are one-shot and invocation-scoped.
- Same-service recursive invocation cannot bypass the continuation.
- Layering cannot expand authority.
- Layer order comes from pinned policy, not registration order.
- Plugin-owned state stays namespaced.
- Rich session extensions do not change the minimal session primitive.
- Required layers fail closed when absent.
- Configurable hook behavior is plugin-owned and uses only ordinary core mechanisms.
- Planned and executed chain provenance is inspectable.

## Required regressions

- no-layer invocation behaves exactly like current single-provider invocation;
- two layers delegate in deterministic configured order to one terminal provider;
- a layer may handle without invoking lower participants;
- a layer may deny and prevent lower participants from running;
- a layer failure prevents lower participants from running;
- a layer may delegate and transform the returned result;
- an incompatible layer version is skipped before invocation;
- an optional missing layer falls through to the terminal provider;
- a required missing or incompatible layer fails resolution;
- explicit terminal binding preserves configured layers;
- a continuation cannot be invoked twice;
- a continuation cannot be reused after its originating invocation;
- same-service recursive invocation is rejected while continuation succeeds;
- delegated authority can only stay equal or shrink;
- higher-priority unauthorized layer is excluded before execution;
- provider failure after dispatch does not trigger another terminal provider;
- provenance records the planned chain and actual handle/delegate/deny/fail path;
- embedded and external layers obey the same chain semantics;
- core model/tool/skill/session/context primitives remain usable without first-party management plugins;
- removing `phenix-plugin-hooks` removes configurable hook behavior without removing generic events or service layering;
- a replacement hook plugin can use the same ordinary core mechanisms without privileged APIs;
- session-tree behavior can layer session creation while ordinary session calls still reach the configured base session implementation;
- disabling an optional session-tree layer leaves basic session behavior available;
- making the session-tree layer required rejects a composition that cannot supply it.
