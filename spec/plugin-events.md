# Plugin event transport and subscriptions

status: specification-only

## Purpose

Provide the kernel-owned Event transport for facts that have already occurred. Events let Plugins observe and react without adding product-specific Event semantics to the kernel.

This document extends `spec/plugin-authoring-macro.md`, `spec/plugin-contributions.md`, and `spec/plugin-host.md`.

## Event versus interception

Events and Layers have different semantics.

**Event.** A fact that already exists. A Listener may observe the fact and perform later work. It cannot reject, transform, or roll back the operation that produced the fact.

**Layer.** Synchronous interposition around an operation that has not completed. A Layer may transform input, handle the operation, delegate once, deny, or fail. See `spec/plugin-service-layering.md`.

Use an Event for observation and reaction. Use a Layer when behavior must affect the originating operation.

There is no pre-commit Event veto mode. A boundary that can still deny or transform an operation is a Layer/interposition boundary, not an Event boundary.

## Event model

The kernel owns generic Event transport and envelope metadata:

```text
EventTypeId
EventVersion
EmitterPluginId
CausalityId
GraphGeneration
payload: PhenixValue
```

The owning Event contract defines payload meaning.

The kernel may define infrastructure Events for kernel and Plugin Runtime lifecycle. Product Events remain userspace Interface contracts.

An Event subscription contains:

- stable subscription identity;
- owning Plugin identity;
- Event type and version;
- optional Listener dependency edges for the same Event;
- Event Delivery failure policy;
- Listener handler binding;
- Graph Generation provenance.

Provider priority is not part of Event dispatch.

## Ordering

Subscriptions for one Event may form an explicit Listener dependency DAG. Registration order and source order have no semantic meaning.

Independent Listeners may run concurrently when the Event contract permits it. If Event Delivery must be serial, declared Listener dependencies determine the order.

Cross-Event dependency edges belong in a controller or userspace scheduler, not in the kernel Event graph.

## Recursion and causality

Each Event Delivery carries causal provenance. The kernel rejects causal re-entry of the same subscription when configured recursion rules would otherwise loop.

A later independent Event may invoke the same subscription normally.

## Listener failure policy

Generic Listener policies may include:

```text
ignore
warn
fail_delivery
```

These policies describe Event Delivery to one Listener. They do not change the already-produced Event or the completed operation that emitted it.

`fail_delivery` reports Event Delivery failure to the caller or controller that requested delivery when that contract needs a synchronous result. It is not an operation veto and does not roll back unrelated Listeners.

Structural payload mismatch uses the canonical kernel diagnostic and never panics.

## Listener actions

A Listener may return an Event Delivery result and use authority-bearing Host Capabilities or ordinary Interface Imports.

There are no Event-specific kernel actions for context, tools, callables, orchestration, sessions, models, or other product domains.

A Listener is not a scheduler. Multi-step or recurring behavior belongs in a kernel-scheduled controller or an ordinary userspace Interface implementation.

## Hooks

Hooks are authoring concepts, not a second Event runtime.

A Hook that only observes a completed fact lowers to an Event and Listener. A Hook that may transform, deny, wrap, or otherwise affect an operation lowers to a Layer.

`phenix-plugin-hooks` may own configurable Hook definitions and user-facing policy, but it receives no privileged kernel path.

## Durability

Event Delivery state is runtime-local unless the Event contract explicitly requires Durable State or durable evidence.

A Listener that produces Durable State writes through its owning Plugin Resource or another declared Interface contract. The kernel does not turn arbitrary Events into product history.

## Invariants

- Events represent facts that already exist.
- Listeners cannot reject, transform, or roll back the originating operation.
- Operation interception uses Layers and continuations.
- Events have zero or more Listeners; Provider priority never selects one Listener.
- Event semantics belong to the Event contract, not the kernel.
- Listener ordering follows declared dependencies, not registration order.
- Same-subscription causal re-entry is bounded.
- Listener failures affect Event Delivery according to policy and do not create hidden veto semantics.
- Listeners use ordinary Host Capabilities and Interface dispatch.
- Event Delivery does not become Durable State or product history unless a userspace contract stores it.

## Required regressions

- two Listeners both receive one Event;
- subscription dependency cycles are rejected;
- deterministic serial ordering follows the declared Listener DAG;
- independent Listeners may run concurrently when allowed;
- recursive same-subscription causal re-entry is blocked;
- first-party and alternate Listeners use the same Event mechanism;
- a Listener cannot bypass Plugin authority;
- Listener failure cannot veto or roll back the originating operation;
- a pre-operation policy is implemented through a Layer rather than an Event;
- a Phenix observation Hook can be implemented through the ordinary Event mechanism;
- the kernel Event module contains no orchestration, context, tool, model, or session-specific action type.
