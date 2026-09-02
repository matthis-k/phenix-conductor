# Durable artifacts and reversible pruning

status: specification-only

## Purpose

Prevent execution context growth by preserving large/reusable outputs durably and replacing old context bytes with compact recoverable references.

## Contract

- Large, reusable, binary, or otherwise expensive outputs may become immutable durable artifacts at production time.
- Model context may contain a compact artifact view plus an exact artifact reference.
- Deterministic pruning happens before model-backed compaction.
- Eligible pruning includes old tool output, large build/test logs, repeated reads, superseded diagnostics, and completed child chatter.
- A projection may lose bytes only when the original remains durably recoverable from immutable history or an artifact.
- Small results may remain inline; artifact promotion is selective rather than universal.
- Pruning is per execution and updates only model-context projection state, never underlying durable semantics.
- Pruned content remains inspectable with reason and recovery reference.

## Invariants

1. Context may lose bytes but never reachability to exact source material.
2. Artifact identity/content is immutable once referenced.
3. Pruning cannot mutate objectives, plans, decisions, authority, observations, or execution history.
4. Backend conversation state is not the preservation mechanism.
5. Deterministic pruning precedes model-backed compaction.

## Non-goals

- model-backed compaction/checkpoints
- semantic decisions/history search
- automatic child spawning due to token pressure
