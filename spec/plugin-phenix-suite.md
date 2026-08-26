# Phenix Plugin Suite

Status: migration and product contract.

## Purpose

Define the first-party userspace that, together with the Phenix Kernel, forms the Phenix Harness.

```text
Phenix Kernel + Phenix Plugin Suite = Phenix Harness
```

The suite implements the agent-harness semantics for which the kernel exists. It is the canonical supported userspace for Phenix, but every component is replaceable and the kernel grants it no special privilege.

## Boundary

The kernel supplies generic hosting, authority, runtime, IPC, events, persistence, and provider-resolution mechanisms.

The Phenix Plugin Suite supplies product services including:

- sessions, lineage, navigation, and durable conversation state;
- artifacts, reads, reuse, dependency tracking, and exact recovery;
- context identity, discovery, projection, compaction, and provenance;
- skills and skill resources;
- tool/service catalogs and invocation semantics;
- callables, execution trees, delegation, orchestration, workers, retries, verification, and failure analysis;
- objectives, plans, decisions, and history search;
- workspace, filesystem, repository, shell, search, Git, and CLI integrations;
- model/provider/authentication/routing/backend services;
- language intelligence;
- frontend-facing services;
- hooks and lifecycle automation;
- persistent terminal/job semantics;
- debugging and export;
- repository-driven worker handoffs.

No item in this list becomes a kernel concept merely because the normal Harness enables it.

## Suite services

The suite should be split into coherent services/plugins rather than one monolith. Example boundaries include:

```text
phenix-sessions
phenix-session-tree
phenix-artifacts
phenix-context
phenix-skills
phenix-tools
phenix-orchestration
phenix-workers
phenix-workspace
phenix-models
phenix-language
phenix-frontends
phenix-jobs
phenix-debug
```

These names are illustrative. Repository/package granularity may differ from runtime service boundaries.

## Replaceability

Each suite service must expose explicit contracts sufficient for an alternate implementation to replace it.

Replacement may be:

- another first-party implementation;
- a third-party plugin;
- a custom product-specific service;
- no service at all when the Harness configuration does not require that feature.

A replacement must not require kernel changes when it implements the same contract.

The entire suite may be replaced by a different userspace.

## Suite composition

Harness policy selects a coherent service graph and declares required relationships, grants, provider bindings, priorities, and settings.

A normal zero-config Harness may select first-party providers by default. That selection is product policy, not kernel behavior.

A suite service may depend on another declared service contract. It should depend on the contract rather than a concrete implementation unless product assembly intentionally binds them.

## Hosting

Trusted first-party Rust services may be embedded in one process for efficiency. They remain userspace plugins architecturally.

External executable hosting is used for independent distribution or enforceable isolation. Resource-only plugins package static skills/templates/schemas/resources without fake executables.

Hosting mode never changes authority or semantic ownership.

## Durable state

Each service owns its canonical domain state in its own durable namespace or explicitly shared contract.

Examples:

```text
phenix-sessions       session records/events
phenix-session-tree   lineage/navigation metadata
phenix-artifacts      artifact records/content refs
phenix-context        context revisions/projections
phenix-workers        task/result/verification state
```

The kernel provides persistence mechanisms and atomic transactions without understanding these schemas.

## Product split

Inside the target `phenix-ai` repository:

- `phenix-kernel` owns kernel mechanisms and plugin contracts/host;
- first-party plugin crates/resources implement the Phenix Plugin Suite;
- `phenix-harness` owns the supported composition and product policy;
- product assembly links the selected embedded factory catalog into the normal binary.

The kernel crate must not depend on concrete suite crates.

## Migration order

Preferred order:

1. establish generic plugin hosting and product assembly;
2. establish kernel-only infrastructure smoke tests;
3. establish generic durable-data and persistence-provider contracts;
4. move all session semantics into suite plugins;
5. move all artifact semantics into suite plugins;
6. move all context and skill semantics into suite plugins;
7. move tool/callable/execution/orchestration/worker semantics into suite plugins;
8. move workspace/CLI/search/Git/read/write/shell services;
9. move model/provider/auth/routing integrations;
10. move language/frontend/debug/job/hook services;
11. remove remaining agent-domain types, registries, tables, and direct paths from the kernel.

## Invariants

- The suite is first-party but not privileged.
- Kernel-only mode is not expected to provide agent-harness behavior.
- Every suite service is replaceable through declared contracts.
- The entire suite is replaceable.
- Suite state uses plugin durable namespaces rather than kernel product tables.
- Embedded hosting does not make a suite service part of the kernel.
- Harness policy, not the kernel, selects the normal Phenix composition.

## Required regressions

- normal Harness boots with the expected Phenix Plugin Suite composition;
- kernel-only profile boots without suite services;
- one first-party service can be replaced by a mock alternate implementation without kernel changes;
- disabling a suite service removes its feature rather than revealing an intrinsic kernel fallback;
- suite durable data survives compatible disable/re-enable;
- first-party status never bypasses authority or provider resolution;
- a fully alternate mock userspace can register and operate using only kernel contracts.