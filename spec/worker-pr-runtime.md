# Repository-driven worker PR enforcement

`spec/worker-pr-handoffs.md` remains normative for PR lifecycle and handoff semantics. This slice implements repository-driven work discovery and enforcement.

## Work sources

- Pull requests are the primary execution and handoff nodes.
- PR descriptions/spec files are the canonical semantic contract.
- PR comments, review comments, and unresolved review threads are findings/amendments/blockers. Substantive requirements are folded into the PR contract rather than remaining hidden in discussion history.
- Issues are backlog inputs. Related issues are aggregated into one coherent semantic PR with explicit `Addresses` / `Partially addresses` traceability before implementation.

## Runtime boundary

Repository adapters collect PR, review, issue, ancestry, and validation evidence. The conductor reconstructs worker state from that evidence with `RepositoryWorkerQueue`; it does not persist a second copy of GitHub state or infer progress from chat history. Semantic grouping is explicit through a stable `semantic_key`, so issue clustering and duplicate-PR suppression are deterministic and reviewable.

## Worker startup

Before changing code, reconstruct current base/head, actual diff, dependency ancestry, CI/Maintenance, review state, linked issues, normative specs, current PR claims, and remaining work. Repository evidence overrides stale PR text or prior worker summaries.

## Progress annotation

Worker-managed PRs use evidence-backed Markdown checkboxes for normative requirements, regressions, validation, and lifecycle progress. `[x]` means repository evidence proves completion; `[ ]` means incomplete or unverified. Workers reconcile checkboxes at startup and again before yielding.

## Selection priority

broken CI/blocking review -> stale or incomplete active PR -> missing regression/spec/invariant -> dependency blocker -> next unblocked PR -> aggregate related issues/comments into a semantic PR -> unrelated new work.

## Advancement

Only a green boundary unblocks dependent implementation: specification complete, regressions complete, dependency/base correct, blocking findings resolved, clean intended head, and full required validation green. Non-draft status alone is not sufficient.

Once a predecessor is merged at a verified green boundary, later advancement of `main` does not invalidate that dependency. Workers validate the merged predecessor and its ancestry rather than requiring its historical base SHA to remain the current tip.

## Invariants

1. Repository state is authoritative over chat history and worker memory.
2. PRs are durable worker checkpoints.
3. Substantive comment requirements are normalized into PR/spec state.
4. Related issues are aggregated by semantic responsibility before implementation.
5. Completion annotations are evidence-backed.
6. Workers repair incomplete prerequisites before creating downstream work.
7. Dependency claims must match Git reality.
