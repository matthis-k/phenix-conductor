# Plugin provider resolution

status: specification-only

Status: implementation contract.

## Purpose

Resolve replaceable providers before activation and pin the result to one immutable graph generation.

Authority determines eligibility. Composition policy determines preference among eligible providers. Runtime dispatch follows the resolved plan and does not search the live plugin set again.

This document extends `spec/plugin-authoring-macro.md` and `spec/plugin-contributions.md`.

## Terms

**Provider candidate.** A plugin contribution that implements a compatible interface.

**Resolved provider.** The provider selected for an import in one graph generation.

**Fallback plan.** An optional ordered set of alternate providers resolved with the graph. Fallback is available only when the interface contract and composition policy explicitly allow it.

**Availability.** Process-local runtime health for a provider instance. Availability can prevent dispatch to a resolved provider. It does not silently change the graph.

## Domain

The kernel may define generic provider-resolution types such as:

```text
InterfaceId
InterfaceRequirement
ProviderId
ProviderBinding
ProviderPriority
ProviderAvailability
ResolvedProviderPlan
```

Interface identity is opaque to the kernel beyond version and structural compatibility. Session, model, tool, context, and other product meanings belong to neutral contracts and plugins.

## Composition policy

Plugins advertise what they provide. They do not choose their own effective global priority or authority.

Composition policy may supply:

- enabled or disabled state;
- authority grant;
- effective provider priority;
- explicit provider binding;
- optional fallback policy where the interface permits fallback;
- opaque scope selectors where the interface requires scoping.

These inputs are part of graph resolution and therefore part of graph identity.

A plugin may declare metadata that is intrinsic to its implementation. Composition policy decides the effective provider choice.

## Effective authority

A provider is eligible only when the operation fits all authority bounds:

```text
caller authority
  ∩ configured plugin grant
  ∩ provider maximum authority
  ∩ interface operation requirements
```

Priority, first-party status, package origin, bundling, or explicit binding never expands authority.

## Resolution

During graph construction the kernel:

1. matches the required interface ID and compatible version;
2. checks structural request and response compatibility;
3. removes disabled candidates;
4. removes candidates that cannot receive the required authority;
5. applies declared scope requirements;
6. applies an eligible explicit binding when configured;
7. otherwise selects by effective composition priority;
8. breaks equal priority by stable provider identity;
9. resolves any explicitly allowed fallback plan using the same eligibility rules;
10. records the complete result in the candidate graph generation.

The resolver never invokes provider code while choosing providers.

Registration order and activation order have no semantic effect.

## Dispatch

Dispatch uses the provider plan pinned to the invocation's graph generation.

The kernel does not perform an unbounded provider search at request time.

If the selected provider cannot accept the call, the result is normally an availability failure. A fallback may be used only when all of these are true:

- the interface contract explicitly permits pre-dispatch fallback;
- composition policy enabled fallback;
- the fallback provider was resolved and pinned in the same graph generation;
- the fallback remains within the invocation's authority bound.

The executed provider and fallback reason are recorded in provenance.

## Failure after dispatch

Once provider execution starts, provider failure is an execution failure. It does not mean "try another provider."

A userspace interface may define explicit replay or retry semantics for safe idempotent operations. Such behavior is part of that interface or a layer implementing it. It is not generic provider search.

Mutating and ambiguity-sensitive operations default to no post-dispatch provider switch.

## Availability

Availability is process-local runtime state, for example:

```text
starting
ready
degraded
unavailable
stopped
```

Availability does not change semantic provider binding inside an existing generation.

A durable provider change, policy change, authority change, or provider replacement creates a candidate graph generation. Successful reconciliation commits the new generation atomically.

## Provenance

The kernel records at least:

- graph generation;
- interface ID and version;
- resolved primary provider;
- resolved fallback plan when present;
- provider actually entered;
- authority bound;
- selection or fallback reason;
- provider artifact and runtime generation where relevant;
- outcome.

The owning userspace service decides how this provenance relates to its domain records.

## Scope

The kernel supports opaque scope values where an interface needs them. It does not define workspace, session, execution, model, repository, or other product scope semantics.

The interface contract owns scope meaning and validation. Scope does not grant authority or priority by itself.

## Invariants

- Provider selection happens during graph resolution.
- Provider plans are pinned to immutable graph generations.
- Runtime dispatch does not search the live plugin set again.
- Authority determines eligibility; composition policy determines preference.
- First-party providers receive no implicit priority.
- Explicit binding never bypasses compatibility or authority checks.
- Registration order does not affect provider choice.
- Provider resolution never executes provider code.
- Availability does not silently mutate semantic binding.
- Pre-dispatch fallback exists only when explicitly allowed and generation-pinned.
- Post-dispatch failure never means generic provider fallback.
- The resolver understands no product-domain semantics.

## Required regressions

- a higher-priority unauthorized provider is excluded before selection;
- explicit binding wins only when compatible and authorized;
- equal-priority providers resolve by stable identity;
- structural incompatibility excludes a candidate before activation;
- a provider, policy, or authority change creates a different graph identity;
- an unavailable primary provider fails when no fallback plan exists;
- an explicitly configured fallback uses only a provider pinned in the same generation;
- runtime availability cannot cause an undeclared provider to receive a call;
- provider failure after dispatch does not select another provider;
- alternate third-party providers use the same resolver as first-party providers;
- provenance records the resolved plan and actual executed provider;
- the kernel resolver requires no session, execution, model, workspace, or repository-specific type.
