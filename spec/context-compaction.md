# Execution context budgeting and compaction

status: specification-only

## Purpose

Keep each execution within the resolved model's actual context capacity without sacrificing durable semantics or exact provenance.

## Order of operations

1. avoid unnecessary injection;
2. delegate bounded independent work when semantically appropriate;
3. deterministic pruning;
4. structured result collapse;
5. model-backed compaction only when still required.

## Contract

- Context budgeting is per execution, not session-global.
- Capacity is derived from the resolved model and accounts for tool schemas, output reserve, and safety margin.
- Category budgets are dynamic rather than fixed quotas.
- Category demand is derived from the canonical execution context projection/accounting; callers do not provide independent category demand or quota state.
- Model-backed compaction uses its own configurable model target and typed output.
- Compactor target and budget policy come from the immutable configuration revision pinned to the execution.
- The compactor has zero authority to mutate objectives, plans, decisions, execution authority/state, workspace observations, or artifacts.
- Compaction creates a durable `ContextCheckpoint` containing summary, exact covered history ranges, retained exact refs, and generation metadata.
- Later compaction may consume a prior checkpoint as an optimization, but provenance continues to resolve to raw durable history ranges rather than summary-only ancestry.
- Provider context-overflow errors trigger an emergency prune/compact/retry path.
- Context pressure may inform planning but never auto-spawns child executions solely due to a token threshold.

## Invariants

1. Context management is per execution.
2. Compaction cannot mutate canonical durable semantics.
3. Checkpoints retain exact provenance to raw history.
4. Deterministic pruning precedes model-backed compaction.
5. Model route/capacity changes recalculate budget from the resolved target.
6. Overflow recovery is explicit and inspectable.
7. Compaction policy and target selection use the execution's pinned configuration revision.

## Non-goals

- decisions/history retrieval implementation
- worker scheduling policy
- semantic plan mutation
