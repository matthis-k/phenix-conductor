# Adaptive worker task DAG

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
  role_requirements
  depends_on[]
  input_refs[]
  expected_result_schema
  state
}
```

Worker-task dependencies form a DAG. Cycles are invalid.

The DAG is semantic scheduling state. Executable orchestration remains the conductor's canonical callable/control-flow abstraction. A task may be enacted by a direct worker execution or by an orchestration node, but task state does not duplicate model selection, retry policy, timeout, or authority configuration.

## Adaptive role selection

The workflow agent may propose the role profile needed for each task. The conductor validates that the selected profile exists in the execution's pinned configuration revision and that the parent may delegate to it.

Role selection is conditional on task type and risk. Phenix must not instantiate planner, architect, implementer, verifier, failure-analyzer, or UI/UX workers merely because those profiles exist.

Typical routing:

- decomposition or prospective strategy -> planner;
- architecture/cross-boundary analysis -> architect;
- tracked-file implementation -> implementer;
- acceptance/evidence validation -> verifier;
- failed-attempt diagnosis -> failure-analyzer;
- user-facing visual/interaction review -> UI/UX designer.

These are defaults, not exclusive mappings.

## Creation

The workflow agent requests a worker through the conductor with:

- bounded task description;
- primary objective and optional supporting objectives;
- plan-step reference when enacting a plan;
- requested worker profile;
- exact input/context references;
- requested delegated authority;
- expected structured result schema.

The conductor then:

1. resolves the worker profile through the parent's pinned `ConfigRevision`;
2. validates objective and plan-step scope;
3. checks the parent's callable/delegation ceiling;
4. attenuates requested authority against the parent and profile maximum;
5. records the worker task and causal creation event;
6. creates the child execution;
7. builds an independent context projection for that execution.

Worker creation never passes an unbounded parent transcript as input.

## Scheduling

A task is runnable when all required dependencies are terminal-successful and its objective/plan state still permits execution.

Independent runnable tasks may execute concurrently subject to existing workspace reader/writer leases and execution scheduling rules.

The workflow agent may add future tasks while the plan remains prospective. Once a plan revision is enacted, changes to its strategy require the existing successor-plan semantics; worker scheduling cannot silently rewrite the enacted plan.

## Workspace writes

Implementer workers use existing workspace leases and stale-write protection. Parallel read-only workers may proceed concurrently. Parallel writers are constrained by the canonical workspace consistency model.

A worker task does not grant a writer lease by itself.

## Failure

Worker execution failure updates the task with an exact execution/attempt cause. The parent may then:

- retry within existing durable retry semantics when the approach is materially the same;
- invoke a failure-analyzer worker;
- create a different successor task;
- invalidate/fail the plan step;
- continue when the task was optional;
- fail the parent objective/execution.

A materially different strategy must not be hidden as a retry. It becomes a new task and, where it changes enacted strategy, a successor plan/decision as required by the existing semantic model.

## Completion

A worker task completes only when its child execution has produced a result conforming to the declared result schema. Completion records the exact result/evidence references used by the parent.

Completion of a task does not by itself complete an objective or plan step; those transitions remain evidence-backed conductor operations.

## Invariants

1. Worker tasks form a DAG, not a fixed roster.
2. Every worker has one bounded task and one primary objective.
3. Worker role/profile comes from the pinned immutable configuration revision.
4. Delegated authority can only attenuate.
5. Task scheduling does not duplicate orchestration execution semantics.
6. Independent workers may run concurrently only through existing workspace lease/consistency rules.
7. Failed attempts and materially different successor approaches remain distinguishable.
8. The parent receives structured results and exact references, not the worker transcript by default.

## Non-goals

This slice does not define the detailed handoff/result envelope, verification gates, or parent context reintegration policy. Those are specified by the worker handoff slice.