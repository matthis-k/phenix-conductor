# Worker results and verification

status: implemented

## Contract

Every completed worker task returns a typed result envelope bound to the worker task and execution. The envelope records structured output, exact evidence references, artifact references where used, and completion/failure provenance. Result schema validation happens before the task may enter completed state.

Verification policy is explicit per task/profile/workflow. Tasks that require verification remain incomplete until a verifier execution records a typed verification result against the exact worker result/evidence set. Verifiers are read-mostly by default and cannot silently repair implementation while claiming independent verification.

A failed worker may be routed to a failure-analyzer profile. Failure analysis produces diagnosis, evidence refs, and a proposed next action. It does not mutate plan/objective/task state directly. The parent/conductor chooses retry, successor task, plan invalidation/failure, continuation, or parent failure through canonical semantics.

Parent context receives compact structured results and exact refs rather than the worker transcript. Large outputs may use durable artifacts once that slice is available.

## Invariants

1. Task completion requires schema-valid structured result.
2. Verification status is distinct from worker self-report.
3. Required verification gates task completion/advancement.
4. Verifiers and failure analyzers do not gain write authority by role.
5. Failure analysis proposes; canonical conductor transitions decide.
6. Parent integration uses exact evidence/result references, not transcript copying.
