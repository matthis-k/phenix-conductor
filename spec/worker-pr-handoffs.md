# Worker PR handoffs

## Purpose

A Phenix worker PR is a self-contained semantic work unit and the durable handoff object between workers. A worker must be able to reconstruct the intended contract, dependency position, repository state, validation state, and remaining work from the PR and repository without relying on chat history.

## Required PR contract

Every worker-managed PR carries these sections:

```text
## Specification

Implements: <slice / issue / architecture section>

Normative requirements:
- ...

## Dependencies

Requires:
- #...

Builds on:
- #...

Addresses:
- #...

Unblocks:
- #...

## Scope

Included:
- ...

Excluded:
- ...

## Invariants

- ...

## Implementation

Completed:
- ...

Still required:
- ...

## Regression coverage

Required:
- ...

Implemented:
- ...

## Validation

- Source: ...
- Rust: ...
- Product: ...
- Integration: ...
- Maintenance: ...

## Current boundary

Head: <sha>

Last verified state:
- ...

Next action:
- ...

## Remaining work

- ...
```

Sections may state `none` when not applicable, but must not be silently omitted when the category is relevant.

## Repository state is authoritative

Before changing a worker PR, a worker reconstructs:

1. normative specification;
2. dependency graph and current base/head;
3. actual diff against the intended base;
4. current CI and Maintenance results;
5. review comments and unresolved threads;
6. claimed implementation and regression state;
7. remaining work recorded by the previous worker.

The worker then compares the PR description against repository evidence. When they disagree, repository state wins and the PR description is repaired before new semantic work proceeds.

Chat history, worker memory, and previous summaries are hints only. They are never authoritative handoff state.

## Semantic size

One PR owns one semantic responsibility. File count and line count do not define scope quality.

A broad file diff is valid when all changes implement one invariant. A small diff is invalid as a work unit when it mixes unrelated contracts.

A worker must not opportunistically absorb another semantic slice merely because nearby code is convenient to change.

## Dependency graph

PR dependencies are first-class rather than inferred from numbering or creation time.

The PR records:

- `Requires`: hard prerequisites whose contract must already exist;
- `Builds on`: immediate stack/base relationship;
- `Addresses`: issues or architecture gaps directly covered;
- `Partially addresses`: optional explicit partial issue coverage;
- `Unblocks`: downstream semantic slices made implementable by this PR.

The actual Git base must agree with the claimed stack dependency. A mismatch is a stale PR contract and must be repaired.

## Lifecycle

Worker PRs have four semantic states:

### Draft / specification

The normative contract and dependencies exist, but substantive implementation is incomplete or has not started.

### Draft / implementation

A worker is actively implementing the contract. The PR may have partial tests and partial validation but is not eligible to unblock dependent implementation.

### Ready for review

The worker believes the semantic contract and required regressions are complete. Full validation must run on the intended clean head.

### Green boundary

The PR satisfies all of the following:

```text
spec complete
+ required regressions complete
+ dependency/base relationship correct
+ unresolved blocking review findings closed
+ clean intended head
+ full CI green
```

Only a green boundary may unblock the next dependent PR's implementation. Draft state or GitHub `ready` status alone is insufficient.

## Worker selection rule

Every worker chooses the next action in this strict priority order:

```text
broken CI
-> stale/incomplete current PR
-> missing regression/spec requirement
-> dependency-blocking PR
-> next ready PR
-> new work
```

Workers must not create or advance new dependent branches while an earlier dependency remains semantically incomplete, except to create specification-only placeholder PRs whose implementation is explicitly blocked on the dependency.

## Startup procedure

When a worker receives a PR:

1. fetch the PR metadata and current base/head;
2. read its normative spec and relevant architecture sections;
3. inspect the actual diff;
4. inspect CI/Maintenance and review state;
5. validate dependency claims against actual Git ancestry/base;
6. compare implemented behavior and tests with normative requirements;
7. repair stale PR description fields;
8. select the highest-priority action from the worker selection rule;
9. only then modify code.

## Checkpoint procedure

Before yielding the PR to another worker, update the handoff fields to record:

- exact current head;
- last state actually verified;
- which validation is complete or still running;
- concrete next action;
- exact remaining semantic and regression work;
- blockers or unresolved review findings.

Do not claim a test or invariant is complete merely because implementation appears plausible.

## Advancement gate

A dependent PR may move from specification-only into implementation only when every hard prerequisite is a green boundary.

A worker may not advance merely because:

- the predecessor is non-draft;
- a subset of CI is green;
- the code compiles;
- Maintenance changed nothing;
- the previous worker said it was complete.

The gate is repository-evidenced semantic completion.

## Invariants

1. The PR is the durable worker checkpoint and handoff object.
2. Repository state overrides stale PR claims and chat history.
3. Workers repair stale handoff metadata before adding new work.
4. One PR owns one semantic responsibility.
5. Dependency relationships are explicit and must match Git reality.
6. New semantic work never outranks broken or incomplete prerequisite work.
7. Only a green boundary unblocks dependent implementation.
8. Validation claims record observed results, not expected results.

## Non-goals

This slice does not define GitHub-specific automation implementation, code-review assignment, worker scheduling transport, or automatic merge policy. It defines the semantic contract those mechanisms must enforce.