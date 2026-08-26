# Kernel baseline

Status: normative architecture contract.

## Purpose

Define the smallest Phenix Kernel profile with no Phenix Plugin Suite services enabled.

The kernel-only profile proves infrastructure. It is not a reduced agent harness and does not need sessions, artifacts, context, skills, tools, callables, models, routing, orchestration, or workers.

## Kernel-only profile

The profile must provide:

- plugin discovery/registration and lifecycle;
- generic service and capability registration;
- authority/grant enforcement and attenuation;
- generic runtime task identity, cancellation, and worker-thread execution;
- IPC/local transport primitives;
- generic event delivery and subscriptions;
- durable namespace/schema registration;
- atomic transactions and migrations;
- one simple local persistence backend or equivalent baseline backend sufficient to exercise the persistence contract;
- immutable kernel policy/configuration snapshots;
- exact kernel-operation provenance.

These are mechanisms. They do not imply any agent-domain model.

## No intrinsic agent product

The kernel must not ship miniature implementations of Phenix userspace concepts to make itself appear product-complete.

Kernel-only mode therefore does not need to provide:

- a flat session service;
- an artifact store API with Phenix artifact semantics;
- context registration/injection semantics;
- skill registration or activation;
- tool or callable domain models;
- orchestration or worker semantics;
- model/provider catalogs or routing;
- frontend product protocols beyond generic plugin/IPC mechanisms.

Those belong to the Phenix Plugin Suite or an alternate userspace.

## Baseline persistence

The kernel may ship a simple local implementation of its generic persistence backend contract so the infrastructure can be tested and booted without an external storage provider.

That backend stores kernel-private infrastructure state and arbitrary valid plugin schemas. It does not define Phenix session, artifact, context, or other product tables.

A persistence provider may replace the baseline backend when explicitly configured and compatible.

## Acceptance

A kernel-only smoke profile must prove:

- the kernel boots with no Phenix Plugin Suite enabled;
- a mock plugin can register a service and capability;
- a mock plugin can register a durable namespace/schema and round-trip data;
- an unauthorized plugin operation is rejected;
- a parent authority cannot delegate more authority than it owns;
- a runtime task can be started and cancelled;
- an event can be emitted and delivered to a subscribed mock plugin;
- a multi-owner generic transaction commits atomically or rolls back completely;
- an alternate mock provider can replace a baseline capability through normal resolution.

No acceptance test should require an agent-domain concept.

## Invariants

- Kernel-only means infrastructure-only.
- Kernel mechanisms remain useful to multiple possible userspaces.
- A product feature does not become kernel code because the normal Harness requires it.
- The Phenix Plugin Suite uses only ordinary plugin contracts.
- Mock and third-party plugins can exercise the same service, persistence, event, and authority mechanisms as first-party plugins.