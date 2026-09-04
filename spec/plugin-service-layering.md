# Layered service invocation

status: specification-only

## Purpose

Allow Plugins to extend or override an operation without replacing its complete implementation.

A resolved invocation chain contains zero or more ordered Layers and one Terminal Provider. A Layer may handle, delegate once, transform, deny, or fail. The kernel resolver constructs the complete chain before activation and pins it to one Graph Generation.

This document extends `spec/plugin-authoring-macro.md`, `spec/plugin-resolution.md`, and `spec/plugin-contributions.md`.

## Ownership boundary

`phenix-core` owns generic runtime mechanisms:

```text
Plugin and Component identity
Interface identity and structural compatibility
Graph resolution and Graph Generations
Terminal Provider selection
Layering and one-shot continuations
Effective Authority attenuation
Event transport
controller and task scheduling
cancellation
persistence namespaces and transactions
lifecycle and provenance
PhenixValue and PhenixSchema
```

Neutral passive contract owners such as `phenix-sdk` own shared semantic vocabulary that independent Providers and consumers need to name without coupling to one implementation:

```text
session contracts
model contracts
tool contracts
skill contracts
context contracts
workspace contracts
execution contracts
other shared product contracts
```

Runtime Plugins own implementations and non-trivial product behavior:

```text
model Providers and routing
tool suites and discovery
skill catalogs and activation policy
session trees and branching
context discovery and compaction
memory and indexing
orchestration and workers
planning and objectives
artifacts
Hooks and policy
diagnostics
```

A shared product contract does not become a Core mechanism merely because many Plugins use it. Core remains product-domain neutral. The neutral contract owner gives independent Providers and consumers one semantic identity without coupling them to a default implementation.

The ownership rule is:

> Core owns composition and execution mechanisms. Neutral contracts own shared semantic vocabulary. Plugins own implementations and product behavior.

## Hooks and events

Hooks are authoring concepts, not a privileged runtime.

A Hook that can affect an operation lowers to a Layer. A Hook that only observes a completed fact lowers to an Event and Listener. See `spec/plugin-events.md`.

Configurable Hook definitions, conditions, persistence, and user-facing management may live in `phenix-plugin-hooks`. That Plugin receives no private registration or dispatch path.

## Terms

**Terminal Provider.** The Provider implementation at the bottom of the resolved invocation chain.

**Layer.** An interposition contribution that runs before the Terminal Provider.

**Continuation.** A kernel-issued, invocation-scoped, one-shot capability that advances to the next participant in the already resolved chain.

**Declared Layer ordering constraint.** Intrinsic ordering metadata supplied by a Layer, such as a stable before/after relationship where semantics require it.

**Effective Layer Order.** The final Layer order produced by the kernel resolver from compatible Layers, declared constraints, and Product Composition Policy.

## Contribution roles

A service contribution has one executable role:

```text
terminal
layer
```

Terminal Provider selection and Effective Layer Order are separate decisions.

A Plugin contributes only Interfaces and versions it understands. Incompatible contributions are excluded before activation.

A Plugin cannot contribute the same Interface and version as both a Terminal Provider and a Layer in one Component contribution.

Resource-only Plugins have no executable Terminal Provider or Layer role.

## Layer ordering

Plugins do not assign themselves effective global priority.

A Layer may declare only ordering information intrinsic to its semantics. Product Composition Policy controls effective enablement, required status, binding policy, and priority. The kernel resolver combines these inputs deterministically.

Conceptually:

```text
Layer declaration
  identity
  compatible Interface/version
  optional intrinsic before/after constraints

Product Composition Policy
  enabled/disabled
  required/optional
  effective priority/order
  authority grant
  scope

kernel resolver
  -> Effective Layer Order pinned to Graph Generation
```

Source order, registration order, and activation order never define Effective Layer Order.

If declared constraints and Product Composition Policy cannot produce one valid deterministic order, candidate Graph construction fails.

## Chain resolution

During candidate Graph construction the kernel resolver:

1. finds compatible Layer contributions for the Interface;
2. checks Effective Authority and scope eligibility;
3. applies Product Composition Policy enablement and required status;
4. orders Layers from declared constraints and effective Product Composition Policy;
5. resolves one Terminal Provider through `spec/plugin-resolution.md`;
6. records the complete invocation chain in the candidate Graph Generation.

A valid executable chain has one Terminal Provider even when an earlier Layer may handle the request without delegation. This keeps Provider Availability decidable before Plugin code runs.

An explicit Terminal Provider binding policy chooses the Terminal Provider. It does not remove configured Layers.

## Layer behavior

A Layer receives the typed request plus a Continuation bound to the remaining resolved chain.

A Layer may:

```text
handle    return a result without delegation
delegate  invoke the Continuation once, then return or transform its result
deny      return an explicit denial without delegation
fail      return an execution failure
```

The Continuation is single-use. A second call fails before another participant runs.

The Continuation is bound to:

- originating invocation;
- Interface and version;
- Graph Generation;
- remaining chain position;
- Effective Authority.

It cannot be stored for later use.

Recursive invocation of the same Interface does not emulate Continuation. Only the kernel-issued Continuation advances the current chain.

## Failure semantics

Denial and failure stop the chain.

A Layer or Terminal Provider error never means "try the next Provider." Provider fallback follows the Graph Generation-pinned rules in `spec/plugin-resolution.md` and occurs only before Provider execution when the Resolved Provider Plan allows it.

Unsupported Interface/version behavior is represented by absence from the resolved chain, not by invoking a Plugin and interpreting an error as pass-through.

An Interface may define typed domain failures or explicit safe replay semantics. Those remain part of that Interface rather than generic Layer behavior.

## Authority

Each chain participant runs under the intersection of:

```text
caller authority
configured Plugin grant
participant maximum authority
Interface operation requirements
```

A Layer may attenuate Effective Authority before delegation. It cannot increase authority for the Continuation or later participants.

Retry, wrapping, handling, denial, and delegation cannot restore removed authority.

## State and transactions

Layering does not share Plugin state.

Each Plugin owns its own Plugin Resources and Durable State. Cross-Plugin state access requires an explicit Interface and Effective Authority.

When one logical mutation needs atomic changes across multiple namespaces, the owning Interface must use the kernel transaction mechanism. Layer order alone does not provide rollback for arbitrary side effects.

Irreversible external effects require Interface-defined semantics. The kernel does not retry a partially executed chain by default.

## Provenance

The kernel records both the resolved invocation chain and the executed path:

```text
Graph Generation
Interface/version
caller and Effective Authority
ordered Layers
selected Terminal Provider
participants entered
per-Layer outcome: handled, delegated, denied, failed
Terminal Provider outcome when reached
```

A delegating Layer remains part of provenance even when it leaves the result unchanged.

## Embedded and bridged plugin runtimes

Embedded Plugins and Plugins supplied through Runtime Provider bridges use the same Layer semantics.

A Runtime Provider bridge represents a Continuation as an opaque invocation-scoped capability. The kernel validates invocation identity, Interface, Graph Generation, chain position, and Effective Authority before advancing it.

A guest crash or Runtime Provider failure stops the chain. It does not select another Terminal Provider after dispatch begins.

## Example: session tree

The shared session contract belongs to a neutral passive contract owner. A session implementation and a session-tree extension are ordinary Plugins.

A session-tree Plugin may:

```text
Layer session creation when lineage must be recorded
provide a separate session-tree Interface
store lineage in its own Plugin Resource keyed by SessionId
```

It does not redefine `SessionId`, change the base session contract, or mutate the Terminal Provider's private Durable State.

If the session-tree Layer is optional and unavailable, the base session contract remains usable. If Product Composition Policy marks the Layer required, Graph construction fails when the Layer cannot participate.

## Product composition policy

Product Composition Policy owns effective composition:

```text
Terminal Provider binding policy and priority
Layer enabled/disabled state
Effective Layer Order or priority
required/optional Layer status
Layer authority grants
scope selectors
```

Plugins advertise capability, compatibility, and intrinsic ordering constraints. They do not grant themselves effective priority, required status, or authority.

The resolved invocation chain is part of Graph Generation identity.

## Invariants

- Core owns generic composition and execution mechanisms, not product-domain contracts.
- Shared product contracts live in neutral passive owners.
- Plugins own implementations and product behavior.
- Provider replacement and Layering are distinct mechanisms.
- Every executable chain has exactly one Terminal Provider.
- Layers advance only through a one-shot Continuation.
- Failure never means generic Provider fallback.
- Denial is explicit and stops the chain.
- Continuations are invocation-scoped and Graph Generation-pinned.
- Same-Interface recursion cannot bypass Continuation semantics.
- Layering cannot expand Effective Authority.
- Effective Layer Order comes from the kernel resolver and Product Composition Policy, not self-promotion or registration order.
- Plugin-owned Durable State stays isolated unless an explicit Interface permits sharing.
- Hooks lower to ordinary Layers or Events.
- Resolved and executed paths are inspectable.

## Required regressions

- no-Layer invocation behaves like direct Terminal Provider invocation;
- two Layers delegate in deterministic Effective Layer Order;
- a Layer may handle without entering lower participants;
- a Layer may deny and prevent lower participants from running;
- a Layer failure prevents lower participants from running;
- a Layer may delegate and transform the result;
- an incompatible Layer is excluded before activation;
- an optional missing Layer leaves the Terminal Provider usable;
- a required missing Layer fails Graph construction;
- explicit Terminal Provider binding policy preserves configured Layers;
- conflicting Layer ordering constraints fail Graph construction;
- a Continuation cannot be invoked twice or reused later;
- same-Interface recursive invocation is rejected while Continuation succeeds;
- delegated Effective Authority can only stay equal or shrink;
- a self-declared Layer priority cannot override Product Composition Policy;
- Provider failure after dispatch does not select another Terminal Provider;
- provenance records the resolved chain and actual path;
- embedded and bridged Layers obey the same semantics;
- session, model, tool, skill, context, workspace, and execution contracts can be consumed without importing default Provider crates;
- removing `phenix-plugin-hooks` removes configurable Hook behavior without removing generic Events or Layering;
- a replacement Hooks Plugin can use the same ordinary Core mechanisms;
- session-tree behavior can Layer session creation without changing the base session contract.
