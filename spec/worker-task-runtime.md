# Worker task DAG runtime implementation

temporary: true

`spec/worker-task-dag.md` is normative for task semantics. This slice implements durable worker tasks, deterministic runnable state, child execution creation, exact result references, replay, and relational persistence.

## Contract

- Worker tasks have durable identity, one primary objective, optional supporting objectives, optional enacted plan-step link, dependencies, requested profile, exact input references, expected result schema, requested delegated authority, and lifecycle state.
- Dependencies form a DAG and cycles are rejected during runtime creation and journal replay.
- Runnable state requires successful dependencies and still-valid objective/plan state.
- Worker creation resolves the requested profile through the parent's pinned configuration, validates scope and exact references, then uses the canonical worker-profile child path for delegation and authority attenuation.
- Child context is an independent conductor-owned execution projection.
- Independent read-only work may run concurrently; writers remain governed by canonical workspace leases and stale-write protection. Worker-task state never allocates, widens, or replaces a workspace lease.
- Retrying the same approach remains an execution retry of the same task. A materially different strategy uses a new worker-task identity and the canonical successor-plan semantics when the enacted plan changes.
- Task completion records typed exact result/evidence references. Detailed schema verification belongs to the next worker-result slice.

## Invariants

1. Task scheduling does not duplicate orchestration execution semantics.
2. Every task has one primary objective.
3. Dependency graph is acyclic.
4. Authority can only attenuate.
5. Workspace consistency, reader/writer leases, and stale-write rules remain authoritative outside the task DAG.
6. Parent receives structured results and exact references, not worker transcript by default.
7. Retry identity and materially different successor-task identity remain distinct durable concepts.
