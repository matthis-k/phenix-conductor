# Plugin event transport and subscriptions

status: implemented

## Purpose

Provide a generic kernel event mechanism for plugin coordination without making Phenix lifecycle hooks, orchestration, context, or other userspace semantics part of the kernel.

Requires `spec/plugin-contributions.md` and `spec/plugin-host.md`.

## Event model

The kernel owns generic event transport and envelope metadata:

```text
EventTypeId
EventVersion
EmitterPluginId
CausalityId
KernelPolicyRevision
payload: typed transport value
```

The emitting userspace service owns the event type's semantic payload contract.

The kernel may define a small set of kernel-infrastructure events for plugin/runtime lifecycle. It must not define product events merely because the Phenix Harness uses them.

A subscription contains:

- stable subscription identity;
- owning plugin identity;
- event type/version;
- optional dependency edges for the same event;
- generic delivery/failure policy allowed by the event contract;
- handler binding;
- immutable kernel policy provenance.

Provider priority is not part of subscription dispatch.

## Ordering

Subscriptions for one event may form an explicit dependency DAG. Registration/source order is not semantic.

Unrelated subscriptions use deterministic ordering only where the event contract requires serial delivery. Independent observers may run concurrently when permitted.

Cross-event dependency edges are invalid unless a higher-level userspace scheduler explicitly models that dependency outside the kernel event mechanism.

## Recursion and causality

Each dispatch carries causal provenance. The kernel rejects causal re-entry of the same subscription when configured recursion rules would otherwise loop.

A later independent event may invoke the same subscription normally.

## Failure policy

Generic policies may include:

```text
ignore
warn
fail_operation
```

`fail_operation` is valid only for kernel or userspace event contracts that explicitly identify a pre-commit veto boundary and route the veto back to the owning operation.

The kernel does not invent product veto semantics.

## Handler actions

A handler may return a result or request permitted generic host operations through `PluginHostHandle`.

It may invoke userspace services only through normal generic service/capability dispatch. There are no special kernel actions for context, tools, callables, orchestration, sessions, or other Phenix domains.

A subscription is not a hidden scheduler. Multi-step product behavior belongs to userspace orchestration/services.

## Lifecycle hooks

Phenix lifecycle hooks should be implemented as userspace event producers/subscribers in the Phenix Plugin Suite where possible.

Only kernel/plugin-runtime lifecycle events remain kernel-defined. Product lifecycle hooks do not justify a second privileged extension runtime.

## Durability

Event delivery state is process-local unless an event contract explicitly requires durable evidence.

Durable facts produced by a handler are written through the owning userspace service's durable schema or another declared service contract.

The kernel does not turn arbitrary event payloads into canonical product history.

## Invariants

- Events have zero or more subscribers; provider priority never chooses a subscriber.
- Event semantics belong to emitters/contracts, not automatically to the kernel.
- Subscription ordering follows declared dependencies, not registration order.
- Same-subscription causal re-entry is bounded.
- Product hook/action semantics remain userspace.
- Handler callbacks use only generic kernel host operations and ordinary service dispatch.
- Process-local notification state does not become durable product history accidentally.

## Required regressions

- two subscribers both receive one event;
- dependency cycles are rejected;
- deterministic ordering follows the declared DAG;
- recursive same-subscription causal re-entry is blocked;
- first-party and alternate subscribers use the same event mechanism;
- handler cannot bypass plugin authority;
- a Phenix lifecycle hook can be implemented without a kernel-specific hook API;
- kernel event module contains no orchestration/context/tool/session-specific action type.