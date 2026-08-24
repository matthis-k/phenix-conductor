# Phenix workers

## Purpose

Phenix exposes one stable workflow/frontend agent and creates specialist workers only when the active objective, plan, or orchestration needs them. Workers are executions owned by the conductor, not a second agent runtime.

## Core model

The workflow agent is the stable user-facing coordinator. It interprets user intent, maintains objective/plan state, delegates bounded work, and integrates worker results. It does not edit tracked files directly.

Workers are created adaptively from the current task/plan DAG. Phenix must not require a fixed always-present team. A worker is an ordinary child execution with an explicit role profile, objective/plan-step linkage, effective authority, callable ceiling, context projection, and expected result schema.

Initial role profiles are semantic defaults, not hard-coded scheduling slots:

- `planner`: read-mostly; decomposes objectives and proposes/revises prospective plans.
- `architect`: read-mostly; evaluates architecture and cross-boundary implications.
- `implementer`: may edit within delegated filesystem/repository authority and run normal checks.
- `verifier`: read-mostly; validates criteria, evidence, tests, and implementation claims.
- `failure_analyzer`: read-mostly by default; analyzes failed attempts and proposes a materially different approach.
- `uiux_designer`: read-only advisory role, instantiated only for user-facing UI/UX work.

Additional specialist profiles may be defined by immutable configuration revisions. Profiles describe constraints and defaults; they do not bypass normal callable or execution authority.

## Authority

Planner, architect, verifier, failure-analyzer, and UI/UX advisory profiles are read-mostly by default. Implementers may receive write authority needed for the delegated step.

Commit, push, destructive repository operations, secret access, outbound network access, IPC, and child delegation remain separately governed by normal conductor authority and policy. A role name never grants capability by itself.

Child authority is still:

```text
parent delegated authority
∩ worker profile maximum
∩ invocation restrictions
```

A worker cannot regain authority removed by its parent or configuration.

## Configuration

Worker profiles compile into the immutable `ConfigRevision` alongside agent definitions and policies. Sessions and executions therefore keep the worker semantics they were pinned to.

A profile should resolve to a canonical `AgentDefinition` plus worker metadata rather than creating a parallel agent registry.

Conceptually:

```text
WorkerProfile {
  id
  description
  agent_definition
  role_kind
  default_result_schema
  delegation_policy
}
```

## Context

Workers receive independent per-execution context projections. They do not inherit the parent transcript wholesale.

Mandatory worker context includes:

- delegated objective and relevant success criteria;
- enacted plan step or explicit bounded task;
- effective authority and policy constraints;
- exact input/evidence references required for the task;
- applicable scoped instructions.

Optional context is loaded through the canonical context catalog. Worker results return exact references to durable evidence rather than copying large private transcripts into the parent.

## Ownership invariants

1. The conductor owns worker identity, lifecycle, authority, objective/plan links, and persistence.
2. The workflow/frontend agent coordinates but does not own durable worker state.
3. Worker profiles reuse canonical `AgentDefinition`, authority, configuration revision, context, and execution machinery.
4. No fixed team is required; workers exist only when delegated work exists.
5. Read-mostly roles cannot mutate tracked files unless explicit authority says otherwise.
6. Role selection never bypasses callable delegation or sandbox policy.
7. Worker creation and completion are durable causal transitions.

## Non-goals

This slice does not define task-DAG scheduling, worker handoff/result envelopes, automatic verification gates, or retry/failure routing. Those are layered on top of this worker identity and authority contract.