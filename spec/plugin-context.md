# Context service plugins

Status: implementation contract.

## Purpose

Define context as userspace service semantics. The kernel provides generic services, authority, persistence, events, runtime tasks, and provenance. It does not own `ContextResourceId`, context injection, context projection, skill activation, or repository discovery.

The Phenix Plugin Suite supplies the normal exact context system and richer project discovery.

## Phenix context service

The first-party context service should own:

- context resource and immutable revision identity;
- source provenance and observations;
- scope, lifetime, requester, and descriptor semantics;
- registration and exact load;
- injection/projection into model or agent execution;
- historical resolution;
- execution-pinned context provenance;
- context-budget projection and compaction contracts where applicable;
- durable context records.

Repository discovery and richer processing may be separate plugins behind contracts such as:

```text
context.discover@1
context.dependencies@1
context.rank@1
context.expand@1
context.compact@1
```

These contracts are userspace contracts even when mediated by the kernel provider resolver.

## Repository context

The normal Phenix suite may provide repository discovery for scoped project instructions, configured project documents, and other exact resources.

Language-aware expansion, code graphs, embeddings, QML-specific context, Git/GitHub context, and remote knowledge should remain focused replaceable providers rather than one monolithic context service.

## Canonicalization

Plugin output becomes canonical only according to the selected context service's contract.

The context service, not the kernel, defines resource identity, immutable revision/content identity, scope, lifetime, observations, and injection semantics.

The kernel enforces the authority used by the service and persists its declared durable state without understanding context meaning.

## Mandatory context

If the Phenix Harness requires a context class for a configured execution policy, that requirement belongs to Harness/suite policy, not to the kernel.

An alternate userspace may define different requirements.

## Composition and replacement

Context roles declare whether providers are additive, singular, or ordered processors. Harness policy chooses composition explicitly.

An alternate context implementation may replace the first-party service through the same declared service contract. The kernel must not contain a privileged path for the Phenix implementation.

## Durable state

Context resources, revisions, injection history, discovery indexes, rankings, and domain caches use plugin-owned schemas.

Derived caches may be rebuildable. Canonical context history is defined by the owning context service.

## Permissions

Context services receive only the authority required for their sources and operations. Local discovery may require filesystem read. Remote discovery may require network or secrets.

Context or skill activation must not implicitly increase unrelated tool/write/network/repository/IPC/secret authority.

## Invariants

- Kernel contains no intrinsic context model or context injection fallback.
- The Phenix Plugin Suite implements normal exact context semantics.
- Context implementations are replaceable.
- Repository discovery is userspace behavior.
- Domain-specific context requires no kernel code changes.
- Context durable data uses plugin namespaces.
- Provider selection cannot expand authority.
- Harness policy, not the kernel, decides required context behavior.

## Required regressions

- kernel-only profile has no context service;
- Phenix context plugin registers, persists, restores, loads, and projects exact context;
- normal repository discovery reproduces current scoped project-file behavior;
- changed direct source invalidates relevant discovery state while unrelated changes do not;
- alternate context provider can replace the first-party implementation without kernel changes;
- QML/mock provider contributes context through ordinary service contracts;
- context/skill activation does not increase execution authority;
- kernel persistence and runtime modules contain no context-specific schema or semantic registry.