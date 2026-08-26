# Plugin system implementation

This PR implements the architecture specified by #398. The PR is incomplete until the runtime, first-party plugin suite, persistence boundary, product assembly, packaging, and regressions satisfy the acceptance criteria below.

Workers continue this PR rather than split checklist items into replacement PRs. A worker may stop at a valid checkpoint; the next worker resumes from the repository and this checklist.

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

- [x] Introduce a kernel crate/module boundary that contains only generic mechanisms.
- [x] Add stable plugin identity, manifest, lifecycle, dependency, and activation contracts.
- [x] Add generic service/capability registration and deterministic provider resolution.
- [x] Enforce plugin authority at registration and invocation boundaries.
- [x] Add namespaced resources, events/subscriptions, and blocking task/cancellation mechanisms.
- [x] Keep the first-party runtime synchronous. Long-running work uses blocking worker threads and typed channels/events.
- [x] Support embedded Rust plugin factories without giving them privileged APIs.
- [x] Support external executable plugins through the specified blocking local protocol and isolation boundary.
- [x] Support resource-only plugins.
- [x] Make kernel-only startup bootable and testable with no Phenix Plugin Suite loaded.

### Durable data and persistence

- [x] Replace agent-domain kernel persistence with generic plugin-owned namespaces, schemas, migrations, and transactions.
- [x] Keep persistence backend selection and transactional enforcement in the kernel.
- [x] Make plugin durable state canonical. Plugin reload/restart recovers from persisted state without backend conversation state.
- [x] Provide the baseline local persistence backend required by the specs.
- [x] Add collision, incompatible-schema, migration, rollback, restart, and authority regressions.

### First-party Phenix Plugin Suite

Move current product semantics behind plugin contracts. A domain stays incomplete while the supported product still reaches its conductor-owned registry/state path.

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
- [x] repository-driven worker handoff behavior

Each first-party component uses the same contribution, resolution, authority, durable-data, event, and lifecycle contracts available to third-party plugins.

### Harness assembly

- [x] Add one product assembly path that constructs a Phenix Harness from the kernel plus the selected first-party plugin set.
- [x] Keep plugin selection/configuration explicit and inspectable.
- [x] Allow replacement or omission of first-party services without kernel changes.
- [x] Prove at least one first-party service can be replaced by an alternate implementation through the same kernel contract.
- [ ] Preserve existing frontend/backend behavior through the supported Harness composition rather than compatibility registries in the kernel.

### Nix packaging

- [x] Expose a kernel package.
- [ ] Expose the normal Harness package and keep `phenix` as the supported plugin-composed product package/alias.
- [x] Add `lib.mkPhenixPlugin` for external/resource plugin packaging.
- [x] Add `lib.mkPhenix` for declarative kernel + plugin composition.
- [x] Keep the normal wrapper path available for the final Harness package.
- [x] Add an explicit kernel-only profile.
- [ ] Add packaging checks for embedded, external, resource-only, replacement, and kernel-only compositions.

### Cleanup

- [x] Remove agent-domain types from the kernel.
- [ ] Remove duplicate registries and compatibility lookup paths replaced by plugin contributions.
- [x] Keep first-party durable schemas in plugin-owned namespaces rather than kernel domain tables.
- [ ] Remove obsolete conductor-only ownership assumptions from docs and tests.
- [ ] Keep files focused. Split large modules by one responsibility rather than growing a monolithic plugin host.

## Required regressions

- [x] kernel boots with zero first-party plugins
- [x] Harness boots with the default Phenix Plugin Suite
- [x] third-party/alternate provider resolves through the same service contract as a first-party provider
- [x] deterministic provider priority and binding
- [x] denied authority cannot be regained through plugin-to-plugin calls, retries, events, or persistence
- [x] plugin restart restores canonical durable state for migrated stateful plugins
- [x] incompatible durable schema fails deterministically
- [x] external plugin crash/timeout does not corrupt kernel state
- [x] blocking task cancellation is observable and leaves durable state consistent
- [x] resource-only plugin cannot execute code
- [x] replacement plugin can substitute a first-party service without kernel modification
- [x] session behavior is absent in kernel-only mode and present when the session plugin is loaded
- [x] artifact read reuse/invalidation works through the artifact plugin
- [x] CLI discovery/version/auth probes use the ordinary workspace service, reject arbitrary targets, and do not self-grant shell authority
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

## Current boundary

The first-party Harness assembly now registers repository worker, sessions, artifacts, CLI probes, context, execution, language, planning, workspace, model routing, jobs, frontend services, hooks, and debug services through ordinary kernel manifests and factories. Focused Plugin Suite and Harness unit runs are green after repairing frontend test authority and the durable hook immutability assertion.

The supported `phenix`/`phenix-harness` Nix package still maps to the legacy conductor binary, so these userspace services are not yet the canonical product path. The remaining migration must switch the product/runtime boundary before the domain checklist can be promoted.

## Completion rule

Documentation or interface skeletons do not complete this PR. A checked item needs code and test evidence on the current head. Do not mark the PR complete while agent-domain semantics remain conductor-owned or while the supported Harness package bypasses plugin contracts.
