# Plugin provider resolution

status: specification-only

## Purpose

Resolve replaceable Providers before activation and pin the resulting Provider Plan to one immutable Graph Generation.

Effective Authority determines Provider eligibility. Product composition policy determines preference among eligible Providers. Runtime dispatch follows the Resolved Provider Plan and does not search the live Plugin set again.

This document extends `spec/plugin-authoring-macro.md` and `spec/plugin-contributions.md`.

## Terms

**Provider Candidate.** A Plugin contribution that implements a compatible Interface.

**Provider Binding.** The resolved association between one Interface Import and its selected Provider in one Graph Generation.

**Resolved Provider Plan.** The Graph Generation-owned primary Provider Binding plus any explicitly allowed fallback Provider Bindings.

**Fallback Plan.** An optional ordered set of alternate Provider Bindings resolved with the Graph Generation. Fallback exists only when the Interface contract and product composition policy explicitly allow it.

**Provider Availability.** Runtime-local health for a Provider instance. Provider Availability can prevent dispatch to a resolved Provider. It does not silently change a Provider Binding or Graph Generation.

## Domain

The kernel may define generic Provider-resolution types such as:

```text
InterfaceId
InterfaceRequirement
ProviderId
ProviderBinding
ProviderPriority
ProviderAvailability
ResolvedProviderPlan
```

Interface identity is opaque to the kernel beyond version and structural compatibility. Session, model, tool, context, and other product meanings belong to neutral Interface contracts and Plugins.

## Product composition policy

Plugins advertise the Interfaces they provide. They do not choose their own effective global priority or authority.

Product composition policy may supply:

- enabled or disabled state;
- Plugin authority grant;
- effective Provider priority;
- explicit Provider binding policy;
- optional fallback policy where the Interface permits fallback;
- opaque scope selectors where the Interface requires scoping.

These policy inputs participate in Graph resolution and therefore in Graph Generation identity.

A Plugin may declare metadata intrinsic to its implementation. Product composition policy determines the effective Provider choice.

## Effective authority

A Provider is eligible only when the operation fits all authority bounds:

```text
caller authority
  ∩ configured Plugin grant
  ∩ Provider maximum authority
  ∩ Interface operation requirements
```

Priority, first-party status, package origin, bundling, or explicit Provider binding policy never expands Effective Authority.

## Resolution

During candidate Graph construction the kernel resolver:

1. matches the required Interface ID and compatible version;
2. checks structural request and response compatibility;
3. removes disabled Provider Candidates;
4. removes Provider Candidates that cannot receive the required authority;
5. applies declared scope requirements;
6. applies an eligible explicit Provider binding policy when configured;
7. otherwise selects by effective product composition priority;
8. breaks equal priority by stable Provider identity;
9. resolves any explicitly allowed Fallback Plan using the same eligibility rules;
10. records the complete Resolved Provider Plan in the candidate Graph Generation.

The kernel resolver never invokes Provider code while resolving Provider Bindings.

Registration order and activation order have no semantic effect.

## Dispatch

Dispatch uses the Resolved Provider Plan pinned to the invocation's Graph Generation.

The kernel does not perform an unbounded Provider search at request time.

If the primary Provider cannot accept the call, the result is normally a Provider Availability failure. A fallback Provider Binding may be used only when all of these are true:

- the Interface contract explicitly permits pre-dispatch fallback;
- product composition policy enabled fallback;
- the fallback Provider Binding was resolved and pinned in the same Graph Generation;
- the fallback remains within the invocation's Effective Authority.

The executed Provider and fallback reason are recorded in provenance.

## Failure after dispatch

Once Provider execution starts, Provider failure is an execution failure. It does not mean "try another Provider."

A userspace Interface may define explicit replay or retry semantics for safe idempotent operations. Such behavior belongs to that Interface or a Layer implementing it. It is not generic Provider search.

Mutating and ambiguity-sensitive operations default to no post-dispatch Provider switch.

## Provider availability

Provider Availability is runtime-local state, for example:

```text
starting
ready
degraded
unavailable
stopped
```

Provider Availability does not change a semantic Provider Binding inside an existing Graph Generation.

A durable Provider change, product composition policy change, authority change, or Provider replacement creates a candidate Graph Generation. Successful kernel reconciliation commits the new Graph Generation atomically.

## Provenance

The kernel records at least:

- Graph Generation;
- Interface ID and version;
- resolved primary Provider Binding;
- resolved Fallback Plan when present;
- Provider actually entered;
- Effective Authority bound;
- selection or fallback reason;
- Plugin Artifact Revision and Plugin Runtime generation where relevant;
- outcome.

The owning userspace service decides how this provenance relates to its domain records.

## Scope

The kernel supports opaque scope values where an Interface needs them. It does not define workspace, session, execution, model, repository, or other product scope semantics.

The Interface contract owns scope meaning and validation. Scope does not grant authority or Provider priority by itself.

## Invariants

- Provider Binding happens during Graph resolution.
- Resolved Provider Plans are pinned to immutable Graph Generations.
- Runtime dispatch does not search the live Plugin set again.
- Effective Authority determines Provider eligibility; product composition policy determines preference.
- First-party Providers receive no implicit priority.
- Explicit Provider binding policy never bypasses Interface compatibility or authority checks.
- Registration order does not affect Provider choice.
- Provider resolution never executes Provider code.
- Provider Availability does not silently mutate a Provider Binding.
- Pre-dispatch fallback exists only when explicitly allowed and Graph Generation-pinned.
- Post-dispatch failure never means generic Provider fallback.
- The kernel resolver understands no product-domain semantics.

## Required regressions

- a higher-priority unauthorized Provider is excluded before selection;
- explicit Provider binding policy wins only when compatible and authorized;
- equal-priority Providers resolve by stable identity;
- structural incompatibility excludes a Provider Candidate before activation;
- a Provider, product composition policy, or authority change creates a different Graph Generation identity;
- an unavailable primary Provider fails when no Fallback Plan exists;
- an explicitly configured Fallback Plan uses only a Provider Binding pinned in the same Graph Generation;
- runtime Provider Availability cannot cause an undeclared Provider to receive a call;
- Provider failure after dispatch does not select another Provider;
- alternate third-party Providers use the same kernel resolver as first-party Providers;
- provenance records the Resolved Provider Plan and actual executed Provider;
- the kernel resolver requires no session, execution, model, workspace, or repository-specific type.
