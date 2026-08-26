# Plugin system implementation

This PR implements the architecture specified by #398. The PR is incomplete until the runtime, first-party plugin suite, persistence boundary, product assembly, packaging, regressions, and final validation satisfy the acceptance criteria below.

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

Move current product semantics behind plugin contracts. A domain stays incomplete while the supported product reaches a superseded conductor-owned registry/state path.

- [x] sessions and session-tree behavior
- [x] artifacts, readers, read reuse, and invalidation
- [x] context and skills
- [x] tools, callables, execution, orchestration, and workers
- [x] planning/objectives/decisions/history behavior that exists in the current product
- [x] workspace and repository services
- [x] default CLI suite, including Git/GitHub/search/read/write/shell integration where applicable
- [x] model/provider/auth/routing services
- [x] language intelligence
- [x] frontend-facing services and projections
- [x] hooks and persistent jobs
- [x] debug/diagnostic services
- [x] repository-driven worker handoff behavior

Each first-party component uses the same contribution, resolution, authority, durable-data, event, and lifecycle contracts available to third-party plugins.

### Harness assembly

- [x] Add one product assembly path that constructs a Phenix Harness from the kernel plus the selected first-party plugin set.
- [x] Keep plugin selection/configuration explicit and inspectable.
- [x] Allow replacement or omission of first-party services without kernel changes.
- [x] Prove at least one first-party service can be replaced by an alternate implementation through the same kernel contract.
- [x] Preserve required frontend/backend product behavior through the supported Harness composition and ordinary plugin service contracts.

### Nix packaging

- [x] Expose a kernel package.
- [x] Expose the normal Harness package and keep `phenix` as the supported plugin-composed product package/alias.
- [x] Add `lib.mkPhenixPlugin` for external/resource plugin packaging.
- [x] Add `lib.mkPhenix` for declarative kernel + plugin composition.
- [x] Keep the normal wrapper path available for the final Harness package.
- [x] Add an explicit kernel-only profile.
- [x] Add packaging checks for embedded, external, resource-only, replacement, and kernel-only compositions.

### Cleanup

- [x] Remove agent-domain types from the kernel.
- [x] Remove duplicate conductor registries and compatibility lookup paths replaced by plugin contributions.
- [x] Keep first-party durable schemas in plugin-owned namespaces rather than kernel domain tables.
- [x] Remove obsolete conductor-only ownership assumptions from docs and tests.
- [x] Keep files focused by mechanism in the kernel and by semantic domain in the Plugin Suite.

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
- [x] Nix kernel-only, default Harness, and alternate-plugin compositions build
- [x] model routing reaches an alternate inference provider through the Harness kernel resolver
- [x] callable execution reaches a tool plugin through the Harness execution service
- [x] supported Harness process restart restores plugin-owned session, context, and planning state
- [x] the ACP product smoke crosses the process boundary with a real session create/get journey rather than a boot-only probe
- [x] retained backend adapter integration targets continue to validate ACP continuity and tool bridging independently of agent-domain ownership
- [x] required product/integration/system scenarios run through Harness assembly or their owning adapter/kernel boundary rather than a conductor runtime

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

The supported `phenix` product is `phenix-harness`, built from `phenix-kernel` plus selected plugins. The complete first-party Plugin Suite is available through ordinary kernel service contracts. Nix composition exercises default, selected-suite, external replacement, resource-only, omission, and kernel-only runtime behavior.

The superseded `phenix-conductor` workspace member and its domain registries, persistence schema, product runtime, and system-test owner are removed. Canonical system coverage now belongs to Harness tests, kernel tests, plugin-domain tests, and adapter integration tests according to responsibility.

The Harness parity suite now covers first-party service routing plus concrete model-provider and callable/tool journeys through ordinary kernel services. Process-roundtrip coverage proves plugin-owned durable state across Harness restart. The ACP product smoke sends a real session create/get JSONL journey through the supported Harness process instead of treating `--list-services` as protocol parity.

The remaining work is merge-candidate hygiene and final exact-head validation. #401 still owns final public package/name consolidation; this PR must not preempt that follow-up.

## Completion rule

Documentation or interface skeletons do not complete this PR. A checked item needs code and test evidence on the current tree. Final completion additionally requires one clean semantic commit on current `main`, full exact-head validation, and a repository-level check for duplicate ownership, stale claims, or transport residue.
