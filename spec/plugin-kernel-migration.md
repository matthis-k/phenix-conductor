# Kernel/userspace migration

Status: migration contract.

## Purpose

Shrink the existing Conductor into a mechanism-only Phenix Kernel and move agent-harness semantics into the replaceable Phenix Plugin Suite.

Requires `spec/plugins.md`, `spec/plugin-kernel-baseline.md`, `spec/plugin-kernel-primitives.md`, `spec/plugin-durable-data.md`, and the plugin host/resolution contracts.

## Migration target

```text
phenix-kernel
  plugin host/lifecycle
  authority/isolation
  generic services/capabilities
  runtime tasks/cancellation/threading
  IPC/events
  generic durable data/transactions/backends
  kernel policy/configuration

Phenix Plugin Suite
  sessions/artifacts/context/skills/tools
  callables/orchestration/workers
  objectives/plans/decisions/history
  workspace/repository/CLI services
  models/providers/auth/routing
  language/frontend/hooks/jobs/debug

phenix-harness
  kernel + selected suite + product policy
```

## Audit test

For every existing module/type/field/table/API, ask:

> Would an unrelated userspace built on the Phenix Kernel need to understand this concept?

If no, it belongs outside the kernel.

A second test is replaceability:

> Could a third-party implementation replace this Phenix behavior without modifying the kernel?

If not, the boundary is incomplete unless the behavior is a genuine trust/infrastructure mechanism.

## Kernel candidates

Retain or extract only generic mechanisms such as:

- plugin host/lifecycle/health;
- authority and isolation;
- generic provider resolution;
- blocking task scheduling and cancellation;
- generic IPC;
- generic event transport;
- durable namespaces/schemas/transactions/migrations;
- persistence backend abstraction;
- immutable kernel policy snapshots;
- kernel-operation provenance.

## Userspace extraction

Move all agent-domain semantics, including currently minimal forms:

- flat sessions and session trees;
- basic artifacts and artifact readers;
- explicit context and context discovery;
- skill identity/activation/content;
- tool/callable identities and invocation semantics;
- execution trees, delegation, orchestration, workers, retries, verification;
- objectives, plans, decisions, history;
- workspace/repository semantics beyond raw authority mechanisms;
- provider/model/auth/routing behavior;
- language/frontend/job/hook/debug services.

Do not retain miniature kernel implementations as fallbacks after extraction.

## Persistence

Retain generic durable schemas, transactions, migrations, recovery gating, backend abstraction, and a narrow baseline local backend.

Move every agent-product table/index/aggregate into the owning plugin schema. Refactor SQLite-specific logic so only the backend implementation knows SQL/SQLite.

## Hosting test

A service may be embedded when it is trusted first-party Rust code and direct calls are useful. It remains userspace architecturally.

Use external hosting for independent distribution or enforceable process isolation. Use resource-only packaging for static content.

Hosting form is not an ownership test.

## Migration shape

Each migration PR should:

1. identify existing product semantics mixed into kernel code;
2. identify the generic mechanism, if any, that must remain;
3. define a userspace service contract;
4. move domain types/state/registries/tables into the owning plugin;
5. add only generic kernel APIs needed by more than one possible userspace;
6. route the normal Harness through the plugin contract;
7. add a mock/alternate provider conformance test;
8. remove the old direct kernel path;
9. verify authority and persistence boundaries;
10. update architecture ownership documentation.

## Dependency order

Preferred order:

1. establish blocking/threaded host and embedded factory/product-assembly contracts;
2. establish kernel-only infrastructure baseline;
3. establish generic durable schema/persistence contracts;
4. move session semantics entirely into suite plugins;
5. move artifact semantics entirely into suite plugins;
6. move context and skill semantics entirely into suite plugins;
7. move tool/callable/execution/orchestration/worker semantics;
8. move CLI/search/Git/workspace/read/write/shell services;
9. move model/provider/auth/routing services;
10. move language/debug/frontend/job/hook services;
11. land wrapper composition for kernel + suite + external/resource packages;
12. perform final dependency/module/schema audit and remove compatibility leftovers.

## Completion criteria

Migration is complete when:

- the kernel can boot without the Phenix Plugin Suite;
- kernel-only tests exercise only infrastructure mechanisms;
- normal Phenix behavior is provided by suite plugins;
- one or more first-party services can be replaced by mock alternatives without kernel changes;
- kernel source contains no Phenix agent-domain registries or product persistence tables;
- kernel crate has no dependency on concrete suite crates;
- embedded suite plugins have no privileged kernel path;
- the Harness composition is explicit and reproducible;
- obsolete direct/compatibility paths are removed.