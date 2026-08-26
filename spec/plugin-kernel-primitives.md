# Kernel mechanism APIs

Status: normative architecture contract.

## Purpose

Define the mechanism boundary between the Phenix Kernel and replaceable userspace services.

Requires `spec/plugins.md` and `spec/plugin-kernel-baseline.md`.

## Core rule

The kernel owns only concepts required to host arbitrary userspace safely and durably.

```text
kernel
  mechanisms + trust boundaries

userspace plugins
  domain semantics + policy + product behavior
```

A concept is not a kernel primitive merely because every current Phenix feature uses it.

## Generic identity and resources

The kernel may define opaque identities for kernel-managed runtime resources such as plugin instances, host calls, tasks, transactions, configurations, and subscriptions.

Userspace services define their own domain identities. `SessionId`, `ArtifactId`, `SkillId`, `CallableId`, plan IDs, worker task IDs, and similar agent-domain identifiers belong to the plugins that define those services.

The kernel may persist and reference those values opaquely through registered schemas and service contracts.

## Authority

The kernel owns:

- authority dimensions and grants;
- attenuation/intersection rules;
- isolation boundaries;
- lease/resource checks required by kernel mechanisms;
- enforcement that plugins cannot increase their own authority.

Userspace defines when its domain operations require particular authority.

## Plugin host and lifecycle

The kernel owns plugin registration, startup, shutdown, health, generations, live-call scoping, failure normalization, and host isolation.

It does not own service-specific lifecycle semantics beyond the generic host contract.

## Services and capabilities

The kernel owns a generic versioned registry and resolver for service/capability providers.

It supports:

- contract identity/version;
- provider availability;
- permission eligibility;
- explicit binding;
- configured priority;
- deterministic selection;
- invocation provenance.

The kernel does not define what `artifact.read`, `session.open`, `tool.invoke`, or `model.complete` means. The owning userspace service defines those contracts.

## Runtime tasks

The kernel owns generic blocking task execution, cancellation, worker-thread scheduling, bounded resources, and host-call lifetime.

Agent execution trees, callables, orchestration DAGs, workers, retries, verification, model turns, and parent/child semantic meaning belong in userspace.

A userspace runtime service may map its semantic execution to one or more kernel tasks.

## Events

The kernel owns generic event transport:

- subscription identity;
- delivery ordering where promised;
- recursion protection;
- failure/veto mechanics where explicitly supported;
- dispatch provenance.

Event schemas and semantic meaning belong to their emitting services.

## IPC

The kernel owns generic local IPC and transport framing required to host external plugins or connect kernel-managed endpoints.

Frontend protocols, ACP semantics, model protocols, repository APIs, and other product transports belong to userspace services unless they are needed solely for plugin hosting.

## Durable data

The kernel owns:

- durable namespaces;
- schema registration/versioning;
- generic queries/mutations;
- transactions;
- migrations;
- recovery gating;
- backend abstraction;
- namespace isolation.

Plugins own their domain schemas and canonical product state.

The kernel does not need a `Session` table, `Artifact` table, context table, callable table, or other Phenix product aggregate.

## Configuration

The kernel owns only configuration needed to host plugins and pin kernel policy: enabled plugins, grants, bindings, provider priority, settings payload identity, and host/runtime configuration.

A plugin owns the semantic interpretation of its settings and may define its own immutable domain revisions through its durable/service contracts.

## Search and indexes

The kernel need not provide product search. Exact kernel-resource lookup may exist where required to operate kernel mechanisms.

Full-text search, semantic search, code graphs, rankings, history search, session search, and artifact indexes belong in plugins.

## Invariants

- Kernel concepts are generic enough to host a non-Phenix userspace.
- Agent-domain identity and semantics live in plugins.
- Kernel authority cannot be bypassed by embedded or first-party plugins.
- Generic persistence does not know plugin field meaning.
- Generic provider resolution does not privilege Phenix implementations.
- A statically linked plugin remains a plugin architecturally.
- Product convenience is never sufficient reason to add a kernel field or domain API.

## Required regressions

- mock third-party service registers without kernel code changes;
- alternate provider replaces a Phenix provider through the same resolver;
- plugin-owned domain IDs round-trip through plugin durable data without kernel interpretation;
- unauthorized provider is ineligible regardless of priority;
- embedded plugin cannot use a private privileged path unavailable to an external/mock implementation;
- kernel-only smoke tests require no agent-domain service.