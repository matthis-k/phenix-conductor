# Layered service invocation

status: specification-only

Status: normative architecture and implementation contract.

## Purpose

Allow plugins to extend or override an operation without replacing its complete implementation.

A resolved call contains zero or more ordered layers and one terminal provider. A layer may handle, delegate once, transform, deny, or fail. The kernel resolves the complete chain before activation and pins it to one graph generation.

This document extends `spec/plugin-authoring-macro.md`, `spec/plugin-resolution.md`, and `spec/plugin-contributions.md`.

## Ownership boundary

`phenix-core` owns generic runtime mechanisms:

```text
plugin and component identity
interface identity and structural compatibility
graph resolution and generations
terminal provider selection
service layering and one-shot continuations
authority attenuation
event transport
controller and task scheduling
cancellation
persistence namespaces and transactions
lifecycle and provenance
PhenixValue and PhenixSchema
```

Neutral passive contract owners such as `phenix-sdk` own shared semantic vocabulary that plugins and clients need to name independently of one implementation:

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

Runtime plugins own implementations and non-trivial product behavior:

```text
model providers and routing
tool suites and discovery
skill catalogs and activation policy
session trees and branching
context discovery and compaction
memory and indexing
orchestration and workers
planning and objectives
artifacts
hooks and policy
diagnostics
```

A shared product contract does not become a Core mechanism merely because many plugins use it. Core remains domain-neutral. The neutral contract owner gives independent providers and consumers one semantic identity without coupling them to a default implementation.

The rule is:

> Core owns composition and execution mechanisms. Neutral contracts own shared semantic vocabulary. Plugins own implementations and product behavior.

## Hooks and events

Hooks are authoring concepts, not a privileged runtime.

A hook that can affect an operation uses layering. A hook that only observes a completed fact uses events. See `spec/plugin-events.md`.

Configurable hook definitions, conditions, persistence, and user-facing management may live in `phenix-plugin-hooks`. That plugin receives no private registration or dispatch path.

## Terms

**Terminal provider.** The implementation at the bottom of the resolved chain.

**Layer.** An interposition contribution that runs before the terminal provider.

**Continuation.** A kernel-issued, invocation-scoped, one-shot capability that advances to the next participant in the already resolved chain.

**Declared ordering constraint.** Intrinsic ordering metadata supplied by a layer, such as a stable before/after relationship where semantics require it.

**Effective layer order.** The final order produced by the resolver from compatible layers, declared constraints, and composition policy.

## Contribution roles

A service contribution has one executable role:

```text
terminal
layer
```

Provider selection and layer ordering are separate decisions.

A plugin contributes only interfaces and versions it understands. Incompatible contributions are excluded before activation.

A plugin cannot contribute the same interface and version as both a terminal and a layer in one component contribution.

Resource-only plugins have no executable terminal or layer role.

## Layer ordering

Plugins do not assign themselves effective global priority.

A layer may declare only ordering information intrinsic to its semantics. Composition policy controls effective enablement, required status, binding, and priority. The resolver combines these inputs deterministically.

Conceptually:

```text
layer declaration
  identity
  compatible interface/version
  optional intrinsic before/after constraints

composition policy
  enabled/disabled
  required/optional
  effective priority/order
  authority grant
  scope

resolver
  -> effective layer order pinned to graph generation
```

Source order, registration order, and activation order never define layer order.

If declared constraints and composition policy cannot produce one valid deterministic order, graph construction fails.

## Chain resolution

During graph construction the kernel:

1. finds compatible layer contributions for the interface;
2. checks authority and scope eligibility;
3. applies composition enablement and required status;
4. orders layers from declared constraints and effective composition policy;
5. resolves one terminal provider through `spec/plugin-resolution.md`;
6. records the complete chain in the candidate graph generation.

A valid executable chain has one terminal provider even when an earlier layer may handle the request without delegation. This keeps availability decidable before plugin code runs.

An explicit terminal binding chooses the terminal provider. It does not remove configured layers.

## Layer behavior

A layer receives the typed request plus a continuation bound to the remaining chain.

A layer may:

```text
handle    return a result without delegation
delegate  invoke the continuation once, then return or transform its result
deny      return an explicit denial without delegation
fail      return an execution failure
```

The continuation is single-use. A second call fails before another participant runs.

The continuation is bound to:

- originating invocation;
- interface and version;
- graph generation;
- remaining chain position;
- effective authority.

It cannot be stored for later use.

Recursive invocation of the same interface does not emulate continuation. Only the kernel-issued continuation advances the current chain.

## Failure semantics

Denial and failure stop the chain.

A layer or terminal error never means "try the next provider." Provider fallback follows the generation-pinned rules in `spec/plugin-resolution.md` and occurs only before provider execution when the resolved provider plan allows it.

Unsupported interface/version behavior is represented by absence from the resolved chain, not by invoking a plugin and interpreting an error as pass-through.

An interface may define typed domain failures or explicit safe replay semantics. Those remain part of that interface rather than generic layer behavior.

## Authority

Each participant runs under the intersection of:

```text
caller authority
configured plugin grant
participant maximum authority
interface operation requirements
```

A layer may attenuate authority before delegation. It cannot increase authority for the continuation or later participants.

Retry, wrapping, handling, denial, and delegation cannot restore removed authority.

## State and transactions

Layering does not share plugin state.

Each plugin owns its own durable resources. Cross-plugin state access requires an explicit interface and authority.

When one logical mutation needs atomic changes across multiple namespaces, the owning interface must use the kernel transaction mechanism. Layer order alone does not provide rollback for arbitrary side effects.

Irreversible external effects require interface-defined semantics. The kernel does not retry a partially executed chain by default.

## Provenance

The kernel records both the resolved chain and the executed path:

```text
graph generation
interface/version
caller and effective authority
ordered layers
selected terminal provider
participants entered
per-layer outcome: handled, delegated, denied, failed
terminal outcome when reached
```

A delegating layer remains part of provenance even when it leaves the result unchanged.

## External and bridged plugins

Embedded and bridged plugins use the same layering semantics.

A runtime bridge represents continuation as an opaque invocation-scoped capability. The kernel validates invocation identity, interface, graph generation, chain position, and authority before advancing it.

A guest crash or bridge failure stops the chain. It does not select another terminal provider after dispatch begins.

## Example: session tree

The shared session contract belongs to a neutral passive contract owner. A session implementation and a session-tree extension are ordinary plugins.

A session-tree plugin may:

```text
layer session creation when lineage must be recorded
provide a separate session-tree interface
store lineage in its own durable resource keyed by SessionId
```

It does not redefine `SessionId`, change the base session contract, or mutate the terminal session provider's private storage.

If the session-tree layer is optional and unavailable, the base session contract remains usable. If composition policy marks the layer required, graph construction fails when the layer cannot participate.

## Configuration

Composition policy owns effective composition:

```text
terminal binding and priority
layer enabled/disabled state
layer effective order or priority
required/optional layer status
layer authority grants
scope selectors
```

Plugins advertise capability, compatibility, and intrinsic ordering constraints. They do not grant themselves effective priority, required status, or authority.

The resolved chain is part of graph identity.

## Invariants

- Core owns generic composition and execution mechanisms, not product-domain contracts.
- Shared product contracts live in neutral passive owners.
- Plugins own implementations and product behavior.
- Provider replacement and layering are distinct mechanisms.
- Every executable chain has exactly one terminal provider.
- Layers advance only through a one-shot continuation.
- Failure never means generic fallback.
- Denial is explicit and stops the chain.
- Continuations are invocation-scoped and generation-pinned.
- Same-interface recursion cannot bypass continuation semantics.
- Layering cannot expand authority.
- Effective layer order comes from the resolver and composition policy, not self-promotion or registration order.
- Plugin-owned state stays isolated unless an explicit contract permits sharing.
- Hooks use ordinary layers or events.
- Planned and executed paths are inspectable.

## Required regressions

- no-layer invocation behaves like direct terminal invocation;
- two layers delegate in deterministic resolved order;
- a layer may handle without entering lower participants;
- a layer may deny and prevent lower participants from running;
- a layer failure prevents lower participants from running;
- a layer may delegate and transform the result;
- an incompatible layer is excluded before activation;
- an optional missing layer leaves the terminal usable;
- a required missing layer fails graph construction;
- explicit terminal binding preserves configured layers;
- conflicting layer ordering constraints fail graph construction;
- a continuation cannot be invoked twice or reused later;
- same-interface recursive invocation is rejected while continuation succeeds;
- delegated authority can only stay equal or shrink;
- a self-declared layer priority cannot override composition policy;
- provider failure after dispatch does not select another terminal provider;
- provenance records the resolved chain and actual path;
- embedded and bridged layers obey the same semantics;
- session, model, tool, skill, context, workspace, and execution contracts can be consumed without importing default provider crates;
- removing `phenix-plugin-hooks` removes configurable hook behavior without removing generic events or layering;
- a replacement hooks plugin can use the same ordinary Core mechanisms;
- session-tree behavior can layer session creation without changing the base session contract.
