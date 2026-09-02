# Memory freshness and revalidation

status: partial

This requirement extends `spec/plugin-memory.md`. It was added after the original PR scope. The PR is not complete until the memory implementation satisfies both specifications.

## Purpose

Derived memory must not stay current after its basis becomes uncertain or stale. Phenix must detect when a memory may no longer hold, revalidate it incrementally, and keep authoritative source state separate from the derived memory record.

## Core rule

Freshness comes from evidence, not a fixed global TTL.

Temporal validity is one signal. Source revisions, supersession, conflicting evidence, and canonical resource changes are also freshness signals.

A stale or unresolved memory must not be silently recalled as current truth.

## Freshness lifecycle

A derived memory has an explicit lifecycle state. The implementation may choose the concrete representation, but it must distinguish at least:

```text
current
needs_validation
historical
```

Superseded or expired records remain addressable as historical evidence when their source provenance remains valid.

A record moves out of `current` when deterministic evidence shows that its basis may have changed. Triggers include:

- `valid_until` expiry or another configured temporal boundary;
- revision change in a referenced canonical resource;
- supersession by a newer memory;
- new durable evidence that conflicts with or materially overlaps the same subject and scope;
- recall that requires a record whose current validity cannot be established from existing metadata.

Freshness state is derived memory state. It never changes the source service's canonical state.

## Revalidation

Revalidation is bounded and incremental. A source change should identify affected memories through provenance and dependency links instead of rescanning all memory.

The default order is:

1. check provenance, source availability, revisions, temporal validity, and supersession;
2. resolve deterministic outcomes without a model call;
3. use `memory.validate` only when semantic judgment is required;
4. use `memory.resolve` only for ambiguous conflicts that remain after validation.

`memory.validate` should normally route to a cheap or local model. Product presets may route it independently from other memory callables. The memory plugin must not contain a provider-specific model name.

A revalidation operation produces one of these semantic outcomes:

```text
keep current
needs validation
supersede
expire
retain as historical evidence
```

The exact wire representation belongs in the neutral SDK contract.

## Recall

Recall filters validity before presenting derived content as current memory.

When a matching record is `needs_validation`, recall may:

- revalidate it synchronously when the answer depends on it and the policy permits the cost;
- omit it from current-memory results;
- return it only with an explicit non-current status when historical evidence is requested.

Recall must never upgrade unresolved memory to current merely because lexical or semantic ranking scores it highly.

## Canonical decisions

A `decision` memory is an index over a decision, not a second decision store.

When another service owns a canonical decision resource, the memory record must reference that resource through ordinary service contracts and provenance. It may cache a derived explanation for recall. Revalidation follows the canonical resource revision and source evidence.

When no canonical decision resource exists, the memory record remains derived memory with exact source provenance. It does not acquire canonical decision authority.

## Events and maintenance

Freshness maintenance consumes ordinary durable userspace events or source revision information after canonical state changes.

Background revalidation should batch related work and use cheap deterministic checks first. Failure leaves affected records non-current or explicitly unresolved. It must not corrupt authoritative source history or silently keep an uncertain record current.

## Model routing

The memory callable set now includes:

```text
memory.summarize
memory.extract
memory.consolidate
memory.validate
memory.resolve
```

Suggested routing:

```text
memory.validate    deterministic first; cheap/local model for semantic checks
memory.resolve     stronger fallback for ambiguous conflicts
```

This is a new requirement for PR #459. Existing implementations and tests in the PR must be updated rather than treating it as follow-up work outside the PR.

## Required regressions

- deterministic freshness checks work without a model call;
- temporal expiry can move a derived record out of current state;
- a referenced canonical resource revision can move affected memory to `needs_validation`;
- supersession can move the older record out of current state while preserving its provenance;
- conflicting new evidence triggers bounded revalidation of affected memory rather than a full-memory rescan;
- semantic freshness checks use the `memory.validate` routed callable;
- ambiguous conflicts may escalate from `memory.validate` to `memory.resolve`;
- stale or unresolved memory is not silently returned as current truth;
- historical recall can still resolve superseded or expired evidence when its source remains available;
- decision memory references canonical decision state when one exists and cannot become a parallel canonical decision;
- revalidation failure leaves authoritative source state unchanged.

## Completion criterion

PR #459 is not merge-ready until the SDK contracts, first-party memory implementation, suite wiring, and conformance tests cover this freshness lifecycle together with the requirements in `spec/plugin-memory.md`.