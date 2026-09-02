# Worker profile runtime implementation

temporary: true

`spec/workers.md` remains normative for worker identity and profile semantics. This slice implements those semantics on top of first-class execution context projection.

## Contract

- Worker profiles name a canonical agent by `CallableId`; they do not copy or replace `AgentDefinition`.
- Profiles are immutable configuration-revision semantics and participate in semantic fingerprinting.
- Child worker creation resolves the profile through the parent execution's pinned configuration revision.
- Effective worker authority is parent delegated authority intersected with the profile maximum and invocation restrictions.
- Read-mostly roles remain non-mutating unless explicit delegated authority permits mutation.
- Worker executions receive independent conductor-owned context projections.
- Restore and replay preserve the profile and configuration identity that governed the worker.

## Invariants

1. Role names never grant capability by themselves.
2. Worker profiles extend canonical agent and execution machinery.
3. Authority can only attenuate.
4. Worker context is an execution projection, not transcript inheritance.
5. Configuration pinning prevents profile semantics from changing under existing executions.
