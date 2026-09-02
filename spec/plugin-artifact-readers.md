# Artifact service, readers, and reuse

status: implemented

## Purpose

Define artifact semantics entirely in userspace. The kernel provides generic persistence, authority, service resolution, events, and provenance mechanisms. It does not define `ArtifactId` or a built-in artifact store.

The Phenix Plugin Suite supplies the normal artifact service because exact durable artifacts, provenance, repeated-read reuse, and invalidation are part of the Phenix product design.

## Artifact service contract

The first-party service may expose versioned contracts such as:

```text
artifact.store@1
artifact.get@1
artifact.read@1
artifact.dependencies@1
artifact.revalidate@1
```

The owning artifact service defines:

- `ArtifactId` and content identity;
- immutability rules;
- producer/provenance semantics;
- persistence and exact recovery;
- descriptors/references;
- read-result identity;
- dependency and invalidation records.

The kernel treats these as plugin-owned contracts and durable records.

## Phenix artifact implementation

The Phenix Plugin Suite should provide conservative exact behavior:

- immutable content-addressed artifacts;
- exact recovery references;
- normalized resource reads;
- repeated-read reuse;
- direct observation dependencies;
- conservative invalidation on relevant source changes;
- provenance for producer, provider/config identity, and source observations.

This service is first-party but replaceable.

## Read result identity

A Phenix read result should record enough information to decide reuse:

```text
ReadResultId
normalized request identity
provider identity + contract version + implementation/config identity
result artifact/content identity
direct observation dependencies
optional structured dependency evidence
producing invocation provenance
```

Equivalent semantic requests should converge on one normalized request identity. Presentation-only prompt wording must not create duplicate reads.

## Conservative reuse

A result may be reused when:

- the normalized request matches;
- the producing provider/config identity remains compatible;
- the referenced artifact still exists;
- every required direct observation remains valid.

A relevant source change invalidates the result. An unrelated workspace change does not.

Reuse points to the prior artifact/result. It does not duplicate content.

## Semantic revalidation

Reuse across changed dependencies is opt-in through an explicit provider contract.

A revalidator returns `still_valid`, `invalid`, or `unknown` with exact provenance. `invalid` or `unknown` causes a fresh read.

The kernel never implements artifact semantic equivalence. It only mediates the provider call and enforces authority.

## Replacement

Another plugin may replace the full artifact service or selected read/revalidation capabilities when the Harness contract permits composition.

Provider changes do not silently reinterpret historical records. Compatible implementations must declare how existing durable schemas/identities are consumed or migrated.

## Durable state

Artifact records, read indexes, dependency graphs, normalized request mappings, reuse metadata, and content references are plugin-owned durable data.

The kernel persistence backend stores them without artifact-specific knowledge.

## Permissions

Readers use caller authority bounded by plugin grants and capability requirements. Provider priority cannot make an otherwise unauthorized reader eligible.

Artifact existence or retrieval never grants filesystem, repository, network, IPC, or secret access.

## Invariants

- Kernel contains no artifact domain model or intrinsic artifact fallback.
- The Phenix artifact service implements the normal exact/provenance/reuse guarantees.
- The whole artifact implementation is replaceable.
- Repeated equivalent reads reuse one valid result rather than duplicate work/content.
- Source invalidation is dependency-scoped, not global.
- Semantic reuse across changed dependencies requires explicit revalidation.
- Artifact durable data lives in plugin-owned schemas.
- Provider priority never bypasses authority.

## Required regressions

- kernel-only profile has no artifact service;
- Phenix artifact plugin stores, restores, and retrieves exact immutable content;
- two equivalent reads reuse one result/artifact identity;
- superficial prompt wording does not create a distinct read identity;
- semantic request parameter changes create a distinct identity;
- unrelated workspace changes preserve reusable results;
- changed dependencies invalidate conservative reuse;
- semantic provider can return `still_valid` with exact provenance;
- `unknown` performs a fresh read;
- alternate artifact provider can replace the first-party provider without kernel changes;
- kernel persistence code contains no artifact-specific schema or logic.