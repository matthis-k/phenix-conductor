---
status: specification-only
---

# Model routing

## Purpose

Define one model-selection contract for direct model targets and named routing profiles.

Routing profiles stay strictly typed inside the routing plugin. Plugins and clients exchange them as `PhenixValue` so any type-compliant producer can contribute a profile without linking against the routing plugin's Rust implementation.

The design must support static routing now and later add model capability metadata, online estimates, local observations, learned performance estimates, budgets, and exploration without changing the model-selection contract.

## Canonical model selection

The public model-selection type has two modes:

```rust
enum ModelSelection {
    Routing(RoutingProfileId),
    Concrete(ModelTarget),
}
```

`Concrete` identifies one deployable target. It bypasses profile ranking but still passes runtime validation such as availability and required model capabilities.

`Routing` names a profile. The routing plugin resolves that profile against the invocation context and available model deployments.

The current domain `ExecutionTarget::{Fixed, Routed}` already represents this distinction. Implementation must converge on one canonical semantic type. Do not keep parallel `ExecutionTarget` and `ModelSelection` enums with equivalent states.

Task kind, estimated difficulty, required capabilities, provider constraints, and later budgets are request context. They do not become variants or fields of `ModelSelection`.

## Routing profile publication

A routing profile is published at:

```text
phenix.routing.profiles.<profile>
```

Examples:

```text
phenix.routing.profiles.default
phenix.routing.profiles.fast
phenix.routing.profiles.best
```

The path segment `<profile>` is the profile identity. The published value must not carry a second independently editable profile id.

Each entry is a `PhenixValue` that must parse as the routing plugin's strict `RoutingProfile` type. The routing plugin owns the profile schema and parsing rules. The dynamic representation is an interchange boundary, not the plugin's internal representation.

Conceptually:

```text
plugin/client value
    |
    v
PhenixValue at phenix.routing.profiles.<profile>
    |
    | schema parse
    v
RoutingProfile
    |
    v
routing implementation
```

A producer may be:

- product configuration;
- another Phenix plugin;
- a protocol client;
- the Neovim bridge or another frontend bridge.

A producer does not need the routing plugin's Rust type as long as its `PhenixValue` satisfies the published schema.

## Contribution ownership

Every profile contribution has an owner and generation or equivalent lifetime identity.

Registration rules:

- registration parses the value before making it visible;
- an invalid profile fails at the publication boundary;
- two live owners cannot silently publish the same profile name;
- duplicate publication returns a structured conflict;
- an owner may replace or remove its own contribution through an explicit operation;
- unloading or disconnecting an owner removes its live contributions;
- profile lookup never depends on registration order.

Last-writer-wins behavior is not part of the contract.

The routing plugin remains the semantic owner of routing. Contributors provide profile values. They do not replace routing validation or selection logic by writing arbitrary internal state.

## Profile schema

`RoutingProfile` is a strict plugin-owned type derived from the dynamic schema.

The initial implementation may retain the existing default-target and callable-specific routing behavior, but profile identity comes from the publication path rather than an `id` field inside the value.

Future profile fields may define policy such as:

```text
provider allow/deny rules
minimum capability requirements
quality/cost/latency preferences
difficulty policy
exploration policy
budget policy
```

Adding such policy must not change `ModelSelection`.

Profile schema evolution uses the normal Phenix contract/versioning rules. A consumer must not guess the meaning of unknown fields.

## Route request

Routing consumes a request context separate from model selection:

```rust
struct RouteRequest {
    selection: ModelSelection,
    task: TaskKind,
    difficulty: Difficulty,
    required_capabilities: CapabilitySet,
    constraints: RouteConstraints,
}
```

Not every field must land in the first implementation. The separation is contractual.

`ModelSelection` answers which selection mode the caller requested. `RouteRequest` carries facts and constraints needed to resolve that request.

## Resolution pipeline

Routing uses ordered stages rather than a multidimensional lookup table:

```text
ModelSelection + request context
    |
    v
resolve profile when routed
    |
    v
hard eligibility filtering
    |
    v
candidate estimation
    |
    v
profile policy ranking
    |
    v
selected concrete target
```

Hard filtering handles facts such as required capabilities, provider availability, context requirements, and hard budget limits.

Ranking handles preferences such as expected quality, reliability, latency, cost, and profile-specific priorities.

A routing profile is policy. Model metadata is fact. Learned estimates are evidence-derived beliefs. Keep those roles separate.

## Model indexing

The model catalog indexes objective model and deployment properties. It does not materialize every combination of profile, task, difficulty, provider, and capability.

Useful indexes include:

```text
models by provider
models by required capability
models by context capacity
models by availability
models by coarse strength threshold
```

The router intersects eligible sets, then ranks the survivors.

Task kind and difficulty influence requirements and ranking. They are not primary model identities.

Provider identity and model identity should remain separable so the same logical model can later have more than one deployment.

## Adaptive estimates

Adaptive routing is an extension of ranking, not a second router.

Use four concepts:

```text
Evidence -> Estimate -> Policy -> Decision
```

Evidence records observations. Estimate predicts model performance for request context. Policy expresses profile preferences and constraints. Decision records the selected target and the inputs used to select it.

The evidence log is authoritative. Estimator state is derived and replaceable.

Possible evidence sources include:

- curated or online benchmark data;
- provider telemetry;
- deterministic local checks;
- verifier results;
- retries and fallback outcomes;
- explicit user feedback.

Local observations must preserve individual attempts. If one model fails and a fallback succeeds, both outcomes remain evidence.

An estimator may later learn a factorized model such as model baseline, task effect, difficulty response, and deployment effect. The routing API must not depend on one estimator algorithm.

Exploration, if enabled, happens only after hard filtering and stays within profile policy. A future contextual bandit or Thompson-sampling implementation must not bypass capability, authority, provider, or budget constraints.

## Client and plugin boundary

Clients and plugins operate on the same dynamic contract:

```text
phenix.routing.profiles.<profile> : RoutingProfile schema
```

The Neovim bridge should expose ordinary Lua ergonomics for creating or removing entries, then lower those values through the generic Phenix value/client boundary. It must not add an Neovim-specific routing protocol.

Other clients can publish the same type-compliant value and get identical routing behavior.

The generated or runtime SDK should expose enough schema information for dynamic clients to construct and validate a profile without importing private plugin implementation details.

## Errors

The boundary preserves structured failures for at least:

```text
invalid_profile
profile_conflict
unknown_profile
unknown_provider
unsupported_capability
unavailable_target
owner_mismatch
```

A concrete selection that cannot satisfy hard requirements fails. It does not silently fall back to a routed profile.

A routed selection with no eligible candidates fails with enough structured information to explain which hard constraints removed the candidates.

## Context negotiation and bounded recovery

Context supplies model-independent demand before routing: mandatory material,
optional material, required content/tool capabilities, and requested output.
Coarse demand uses bytes/content kinds and estimate provenance, not a token count
silently shared across tokenizers. Mandatory demand is a hard eligibility input;
optional full-history size is not an irreducible minimum.

Routing returns a concrete target and a pinned effective capability/limit snapshot
as defined in `model-turn-protocol.md`. Context then materializes a target-specific
projection, including schema overhead, output reserve, and safety margin.

If the projection cannot fit after permitted reduction, routed selection may try
the next eligible target from the same pinned policy. Default: at most two distinct
targets per logical turn, each attempted once for admission. Concrete selection
never changes target. If all attempts fail, return typed context exhaustion with
the required floor and available capacity.

Use adapter/model limit metadata or an explicitly configured conservative limit.
If capacity is unknown, expose that fact and use bounded best-effort admission only
where the request permits it. A strict fit guarantee requires known/configured
limits. Price or usage unknown cannot be treated as free when enforcing a hard
monetary budget; require a conservative cost bound or reject that candidate.

Persist the route decision, profile/configuration generation, capability snapshot,
and projection revision together for each dispatch attempt. Recheck availability
and authority at dispatch. A changed capability snapshot requires rematerialization.
Active executions retain their pinned profile after its contributor disconnects;
removal affects new resolutions. Revoked authority takes effect immediately.

## Helper-call defaults

The normal Harness publishes one default profile through the same typed contribution
path as custom profiles. It uses the product's selected deployment. No second model,
provider credential, local inference server, or learned estimator is required.

Classifier, summarizer, verifier, and worker requests carry distinct task kinds.
An explicit callable target takes precedence; otherwise helpers inherit the selected
deployment when compatible. A concrete root selection also stays concrete for its
helpers unless the user configured a separate helper policy.

Helpers use isolated requests with minimal context, fixed budgets, and no tools
unless their task requires authorized tools. They cannot run root recovery.
Classifier and summarizer calls cannot recursively compact or delegate themselves.
Nested worker delegation requires explicit ordinary orchestration policy and stays
within the same root budget and depth limits. Strict parsing works
without native structured-output support; invalid output follows the caller's
bounded failure path. A backend unable to isolate a helper cannot run that helper.

Unavailable optional helpers leave deterministic execution available. An incompatible
required verifier or worker returns a typed failure. Optional estimator failure uses
the deterministic ranker; it does not change hard eligibility rules.

## Required regressions

- concrete selection reaches the requested target without profile lookup;
- concrete selection still rejects unavailable or capability-incompatible targets;
- routed selection resolves `phenix.routing.profiles.<profile>`;
- profile identity is derived from the publication path;
- a valid dynamic `PhenixValue` parses to the strict `RoutingProfile` type;
- an invalid dynamic value fails before publication;
- duplicate live owners for one profile name receive a conflict;
- an owner can explicitly replace its own profile;
- owner disconnect or unload removes its live profiles;
- profile lookup is independent of registration order;
- runtime configuration and a client-published profile use the same routing path;
- the application/client boundary round-trips the same profile schema and value;
- the Neovim bridge can publish a profile through the generic client value path once the frontend integration lands;
- routing explanation can identify the selected profile and final concrete target without provider-specific parsing.

## Implementation order

1. Reuse or rename the existing `ExecutionTarget::{Fixed, Routed}` representation so there is one canonical `ModelSelection` concept.
2. Make profile identity derive from `phenix.routing.profiles.<profile>` and remove duplicated profile-id state from the profile value.
3. Expose the strict routing profile schema through the generic Phenix value/SDK boundary.
4. Add owned profile contribution registration, replacement, removal, and lifetime cleanup.
5. Route product configuration through the same contribution path.
6. Route protocol-client contributions through the same path.
7. Add Neovim/Lua profile publication as a frontend follow-up using the generic client bridge.
8. Keep the current deterministic routing behavior as the first estimator/ranker implementation.
9. Add model metadata, evidence sources, learned estimates, budgets, and exploration as independent follow-up plugins or routing components.

## Non-goals

- Make `PhenixValue` the routing plugin's internal state representation.
- Let profile values bypass strict parsing.
- Encode task kind, difficulty, capabilities, provider, and budget as one giant lookup-table key.
- Add a Neovim-only routing protocol.
- Add adaptive learning before deterministic routing and contribution ownership are correct.
- Silently replace a concrete model request with a routed choice.
