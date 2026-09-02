# Execution context projection

status: implemented

## Purpose

Make model context a first-class conductor-owned projection over durable workspace semantics. Backend conversation state is disposable and never canonical.

## Contract

- Every execution has its own `ExecutionContextProjection` produced by a conductor-owned context manager.
- The projection is derived deterministically from durable execution/session/objective/plan state, mandatory scoped instructions, exact context injections, tool/callable schemas, and other explicitly selected resources.
- The backend receives the resolved projection; it does not own semantic context state.
- Child executions receive independent projections. They do not inherit parent transcripts wholesale.
- Parent/child exchange uses typed results and exact durable references.
- Projection inspection must explain why each resource is present, requester, exact revision/content identity, lifetime, and recovery reference.
- Restoring the same durable state and configuration revision must recreate the same logical projection.
- Context additions continue to use the canonical exact-revision context catalog/injection path.
- Authority and callable delegation remain unchanged by projection construction.

## Operations

The canonical context manager should support the equivalent of:

- catalog
- load
- project_execution
- inspect
- resolve_reference
- account_tokens

Prune and compact operations belong to later slices and must not be implemented as hidden policy here.

## Invariants

1. Durable semantics are canonical; model context is a projection.
2. Projection identity is per execution, not session-global.
3. Optional context enters through exact durable references/injections.
4. Backend/model state cannot silently add durable semantic context.
5. Restored executions preserve logical projection semantics.
6. Child context is explicitly constructed, never implicit transcript inheritance.
7. Projection construction cannot expand execution authority.

## Non-goals

- durable artifact promotion
- deterministic pruning
- model-backed compaction
- context checkpoints
- decisions/history search
- worker-profile implementation
