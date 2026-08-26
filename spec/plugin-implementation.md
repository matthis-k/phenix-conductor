# Plugin system implementation

This PR implements the architecture specified by #398. The PR is incomplete until the runtime, first-party plugin suite, persistence boundary, product assembly, packaging, and regressions satisfy the acceptance criteria below.

Workers must continue this PR rather than split checklist items into replacement PRs. A worker may stop at a valid checkpoint; the next worker resumes from the repository and this checklist.

## Dependency

Requires #398. Treat the specs introduced there as normative.

## Target composition

```text
phenix-kernel
  generic mechanisms and trust boundaries

Phenix Plugin Suite
  replaceable first-party userspace services

phenix-harness
  kernel + selected plugins + product policy
```

Kernel-only mode must boot and pass its own tests without loading any agent-domain service.

## Implementation checklist

### Kernel boundary

- [ ] Introduce a kernel crate/module boundary that contains only generic mechanisms.
- [ ] Add stable plugin identity, manifest, lifecycle, dependency, and activation contracts.
- [ ] Add generic service/capability registration and deterministic provider resolution.
- [ ] Enforce plugin authority at registration and invocation boundaries.
- [ ] Add namespaced resources, events/subscriptions, and blocking task/cancellation mechanisms.
- [ ] Keep the first-party runtime synchronous. Long-running work uses blocking worker threads and typed channels/events.
- [ ] Support embedded Rust plugin factories without giving them privileged APIs.
- [ ] Support external executable plugins through the specified blocking local protocol and isolation boundary.
- [ ] Support resource-only plugins.
- [ ] Make kernel-only startup bootable and testable with no Phenix Plugin Suite loaded.

### Durable data and persistence

- [ ] Replace agent-domain kernel persistence with generic plugin-owned namespaces, schemas, migrations, and transactions.
- [ ] Keep persistence backend selection and transactional enforcement in the kernel.
- [ ] Make plugin durable state canonical. Plugin reload/restart must recover from persisted state without backend conversation state.
- [ ] Provide the baseline local persistence backend required by the specs.
- [ ] Add collision, incompatible-schema, migration, rollback, restart, and authority regressions.

### First-party Phenix Plugin Suite

Move current product semantics behind plugin contracts. No item is complete while the kernel still owns the corresponding domain model, registry, table, or policy.

- [ ] sessions and session-tree behavior
- [ ] artifacts, readers, read reuse, and invalidation
- [ ] context and skills
- [ ] tools, callables, execution, orchestration, and workers
- [ ] planning/objectives/decisions/history behavior that exists in the current product
- [ ] workspace and repository services
- [ ] default CLI suite, including Git/GitHub/search/read/write/shell integration where applicable
- [ ] model/provider/auth/routing services
- [ ] language intelligence
- [ ] frontend-facing services and projections
- [ ] hooks and persistent jobs
- [ ] debug/diagnostic services
- [ ] repository-driven worker handoff behavior

Each first-party component uses the same contribution, resolution, authority, durable-data, event, and lifecycle contracts available to third-party plugins.

### Harness assembly

- [ ] Add one product assembly path that constructs the normal Phenix Harness from the kernel plus the selected first-party plugin set.
- [ ] Keep plugin selection/configuration explicit and inspectable.
- [ ] Allow replacement or omission of first-party services without kernel changes.
- [ ] Prove at least one first-party service can be replaced by an alternate implementation through the same kernel contract.
- [ ] Preserve existing frontend/backend behavior through the Harness composition rather than compatibility registries in the kernel.

### Nix packaging

- [ ] Expose a kernel package.
- [ ] Expose a normal Harness package and keep `phenix` as the supported product package/alias.
- [ ] Add `lib.mkPhenixPlugin` for external/resource plugin packaging.
- [ ] Add `lib.mkPhenix` for declarative kernel + plugin composition.
- [ ] Keep the normal wrapper path working.
- [ ] Add an explicit kernel-only profile.
- [ ] Add packaging checks for embedded, external, resource-only, replacement, and kernel-only compositions.

### Cleanup

- [ ] Remove agent-domain types from the kernel.
- [ ] Remove duplicate registries and compatibility lookup paths replaced by plugin contributions.
- [ ] Remove kernel tables whose schemas belong to first-party plugins.
- [ ] Remove obsolete conductor-only ownership assumptions from docs and tests.
- [ ] Keep files focused. Split large modules by one responsibility rather than growing a monolithic plugin host.

## Required regressions

- [ ] kernel boots with zero first-party plugins
- [ ] Harness boots with the default Phenix Plugin Suite
- [ ] third-party/alternate provider resolves through the same service contract as a first-party provider
- [ ] deterministic provider priority and binding
- [ ] denied authority cannot be regained through plugin-to-plugin calls, retries, events, or persistence
- [ ] plugin restart restores canonical durable state
- [ ] incompatible durable schema fails deterministically
- [ ] external plugin crash/timeout does not corrupt kernel state
- [ ] blocking task cancellation is observable and leaves durable state consistent
- [ ] resource-only plugin cannot execute code
- [ ] replacement plugin can substitute a first-party service without kernel modification
- [ ] session behavior is absent in kernel-only mode and present when the session plugin is loaded
- [ ] artifact read reuse/invalidation works through the artifact plugin
- [ ] Nix kernel-only, default Harness, and alternate-plugin compositions build
- [ ] existing product/integration/system scenarios continue to pass through Harness assembly

## Validation

The PR is complete only when all applicable repository validation is green on the final rebased head.

- [ ] formatting/static source checks
- [ ] Clippy across the full Rust workspace
- [ ] all Rust unit tests in one run so failures are visible together
- [ ] doc tests
- [ ] integration/system suites
- [ ] product/startup scenarios
- [ ] Nix checks and packaging tests
- [ ] Maintenance checks
- [ ] Maintenance autofix with a clean resulting tree

## Completion rule

Documentation or interface skeletons do not complete this PR. A checked item needs code and test evidence on the current head. Do not mark the PR complete while agent-domain semantics remain kernel-owned or while the default Harness still bypasses plugin contracts.
