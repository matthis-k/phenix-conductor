# Plugin event transport and subscriptions

Status: implementation contract.

## Purpose

Provide a generic kernel event mechanism for facts that have already occurred. Events let plugins observe and react without adding product-specific event semantics to the kernel.

This document extends `spec/plugin-authoring-macro.md`, `spec/plugin-contributions.md`, and `spec/plugin-host.md`.

## Event versus interception

Events and interception have different semantics.

**Event.** A fact that already exists. A listener may observe the fact and perform later work. It cannot reject, transform, or roll back the operation that produced the fact.

**Layer.** Synchronous interposition around an operation that has not completed. A layer may transform input, handle the operation, delegate once, deny, or fail. See `spec/plugin-service-layering.md`.

Use an event for observation and reaction. Use a layer when behavior must affect the originating operation.

There is no pre-commit event veto mode. A boundary that can still deny or transform an operation is an interception boundary, not an event.

## Event model

The kernel owns generic transport and envelope metadata:

```text
EventTypeId
EventVersion
EmitterPluginId
CausalityId
GraphGeneration
payload: PhenixValue
```

The event contract owns payload meaning.

The kernel may define infrastructure events for kernel and plugin-runtime lifecycle. Product events remain userspace contracts.

A subscription contains:

- stable subscription identity;
- owning plugin identity;
- event type and version;
- optional dependency edges for the same event;
- delivery failure policy;
- handler binding;
- graph-generation provenance.

Provider priority is not part of event dispatch.

## Ordering

Subscriptions for one event may form an explicit dependency DAG. Registration order and source order have no semantic meaning.

Independent listeners may run concurrently when the event contract permits it. If delivery must be serial, declared dependencies determine the order.

Cross-event dependency edges belong in a controller or userspace scheduler, not in the kernel event graph.

## Recursion and causality

Each dispatch carries causal provenance. The kernel rejects causal re-entry of the same subscription when configured recursion rules would otherwise loop.

A later independent event may invoke the same subscription normally.

## Listener failure policy

Generic listener policies may include:

```text
ignore
warn
fail_delivery
```

These policies describe delivery of the event to the listener. They do not change the already-produced event or the completed operation that emitted it.

`fail_delivery` reports event-delivery failure to the caller or controller that requested delivery when that contract needs a synchronous result. It is not an operation veto and does not roll back unrelated listeners.

Structural payload mismatch uses the canonical kernel diagnostic and never panics.

## Handler actions

A listener may return a delivery result and use authority-bearing host capabilities or ordinary service imports.

There are no event-specific kernel actions for context, tools, callables, orchestration, sessions, models, or other product domains.

A listener is not a scheduler. Multi-step or recurring behavior belongs in a kernel-scheduled controller or an ordinary userspace service.

## Hooks

Hooks are authoring concepts, not a second event runtime.

A hook that only observes a completed fact lowers to an event and listener. A hook that may transform, deny, wrap, or otherwise affect an operation lowers to service interposition.

`phenix-plugin-hooks` may own configurable hook definitions and user-facing policy, but it receives no privileged kernel path.

## Durability

Event delivery state is process-local unless the event contract explicitly requires durable evidence.

A listener that produces durable state writes through the owning resource or another declared service contract. The kernel does not turn arbitrary events into product history.

## Invariants

- Events represent facts that already exist.
- Listeners cannot reject, transform, or roll back the originating operation.
- Operation interception uses layers and continuations.
- Events have zero or more listeners; provider priority never selects one listener.
- Event semantics belong to the event contract, not the kernel.
- Subscription ordering follows declared dependencies, not registration order.
- Same-subscription causal re-entry is bounded.
- Listener failures affect delivery according to policy and do not create hidden veto semantics.
- Listeners use ordinary host capabilities and service dispatch.
- Event delivery does not become durable product history unless a userspace contract stores it.

## Required regressions

- two listeners both receive one event;
- subscription dependency cycles are rejected;
- deterministic serial ordering follows the declared DAG;
- independent listeners may run concurrently when allowed;
- recursive same-subscription causal re-entry is blocked;
- first-party and alternate listeners use the same event mechanism;
- a listener cannot bypass plugin authority;
- listener failure cannot veto or roll back the originating operation;
- a pre-operation policy is implemented through a layer rather than an event;
- a Phenix observation hook can be implemented through the ordinary event mechanism;
- the kernel event module contains no orchestration, context, tool, model, or session-specific action type.
