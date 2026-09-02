# Durable decisions and searchable history

status: implemented

## Contract

Decisions are workspace-owned durable semantic state. Drafts may change until recorded; recorded decisions are immutable. Changes use explicit successor/reversion relations. A decision records the question, chosen option, rationale, alternatives or why none were considered, evidence references, creator, objective references, dependencies, and any successor/reversion relation.

Decision dependencies form a DAG and cycles are rejected. Applicability is assessed separately from historical identity as applicable, questionable, or invalidated. Routine runtime failover remains an event rather than a semantic project decision.

History retrieval combines exact reference resolution, relational filtering, and SQLite FTS. Default scope is the current objective lineage and related plans/decisions; whole-workspace search is explicit. Search indexes are derived and rebuildable; relational durable state remains authoritative. Context discovery exposes decisions/history descriptor-first and loads full bodies through the canonical context path.

## Invariants

1. Recorded decisions are immutable.
2. Decision dependency graphs are acyclic.
3. Relational durable state is authoritative over search indexes.
4. Default history queries are objective-lineage scoped.
5. Exact references never resolve through moving aliases.
6. Runtime events and semantic project decisions remain distinct.

## Non-goals

Lifecycle hooks, persistent terminals/jobs, and embedding indexes as canonical state.
