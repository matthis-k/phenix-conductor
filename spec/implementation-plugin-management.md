---
temporary: true
---

# Desired-state plugin management

## Goal

Implement kernel-owned plugin load, unload, replacement, and reconciliation through one desired-state transaction model.

## Required changes

- Add typed `PluginManagementRequest`, `PluginLoadRequest`, `PluginUnloadRequest`, and desired plugin-set reconciliation contracts.
- Treat loading an active `PluginId` with a different artifact as replacement. Do not add a separate reload subsystem.
- Support `expected_active_revision` compare-and-swap semantics.
- Resolve runtime providers, contribution compatibility, authority, resources, and dependencies before activation.
- Prepare and start candidate instances before graph commit.
- Commit one immutable graph generation atomically.
- Keep the previous generation active on every pre-commit failure.
- Reject unload when required imports become unsatisfied.
- Reconcile runtime-provider dependencies and reject cycles or unavailable runtimes before commit.
- Pin exact artifact revision and runtime provider in the committed generation.
- Keep old invocations pinned to their original generation until drain completes.

## Tests

Cover load, unload, replacement, stale revision, failed start rollback, required-import rejection, unknown runtime, runtime-provider cycle, provider removal with dependents, and old/new invocation generation pinning.

## Completion

- `plugin-runtime-bridges.md` accurately reflects the implemented management subset;
- no parallel load/reload path exists;
- Source, Rust, Product, and Maintenance pass on the exact head;
- delete this temporary implementation slice before merge.
