# Adaptive worker task DAG

status: implemented

## Purpose

Phenix derives specialist worker executions from the active objective and enacted plan rather than maintaining a fixed team. This specification defines how the workflow agent and conductor turn bounded work into an explicit worker DAG.

## Task model

A worker task is a bounded unit of delegated work linked to exactly one primary objective and, when applicable, one enacted plan step.

Conceptually:

```text
WorkerTask {
  id
  primary_objective
  supports[]
  plan_step?
  description
  profile
  depends_on[]
  input_refs[]
  expected_result_schema
  delegated_authority
  state
}
```

Worker-task dependencies form a DAG. Cycles are invalid.

The DAG is semantic scheduling state. Executable orchestration remains the conductor's canonical callable and control-flow abstraction. A task may be enacted by a direct worker execution or by an orchestration node, but task state does not duplicate model selection, retry policy, timeout, or authority configuration.

## Creation

The workflow agent requests a worker through the conductor with a bounded task description, one primary objective, optional supporting objectives, an optional enacted plan step, a requested worker profile, exact input references, requested delegated authority, and an expected structured result schema.

The conductor then:

1. resolves the worker profile through the parent's pinned configuration revision;
2. validates objective and plan-step scope;
3. validates every exact input reference;
4. validates task dependencies and rejects cycles;
5. records the task before execution creation;
6. creates the child through the canonical worker-profile child path, which enforces delegation and authority attenuation;
7. binds the child to the plan step through the canonical plan operation when applicable;
8. builds the child's independent context projection;
9. records the exact task-to-execution binding.

Worker creation never passes an unbounded parent transcript as input.

## Scheduling

A task is runnable when it is pending, every dependency completed successfully, its primary and supporting objectives remain active, and its optional plan step remains runnable. Runnable task ordering is deterministic by task identity.

Independent runnable tasks may execute concurrently subject to existing workspace reader/writer leases and execution scheduling rules. A worker task never grants a lease itself.

Once a plan revision is enacted, materially different strategy uses a new worker task and the existing successor-plan semantics where required. It is not hidden as a retry.

## Failure and completion

Worker execution failure records the exact child execution and a durable failure cause. The parent may retry the same approach through existing retry semantics or create a materially different successor task.

Task completion requires the bound child execution to have completed and records typed exact result/evidence references. Detailed result-schema verification is owned by the worker-result verification slice; this slice durably carries the declared schema and exact references without creating a second validation system.

Completion of a task does not itself complete an objective or plan step. Those transitions remain evidence-backed conductor operations.

## Invariants

1. Worker tasks form a DAG, not a fixed roster.
2. Every worker task has one bounded description and one primary objective.
3. Worker profile lookup uses the parent's pinned immutable configuration revision.
4. Delegated authority can only attenuate through the canonical child path.
5. Task scheduling does not duplicate orchestration execution semantics.
6. Workspace lease and stale-write rules remain canonical.
7. Failed attempts and materially different successor approaches remain distinguishable.
8. Parent/worker exchange uses structured data and exact references rather than transcript sharing by default.
