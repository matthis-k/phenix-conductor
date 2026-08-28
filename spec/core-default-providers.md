# Basic agent components

Status: normative architecture and implementation contract.

## Purpose

Keep `phenix-core` limited to broadly shared contracts and mechanisms while making a minimal usable Phenix composition easy to assemble from ordinary replaceable components.

Core must not own hidden product behavior simply because a basic implementation is useful in most configurations.

## Canonical shape

```text
consumer component
  -> typed core interface
  -> resolved component import handle
  -> selected basic or replacement component
```

The Harness chooses the implementation. Core defines the contract, linking, authority, execution, persistence, events, tasks, isolation, and provenance needed to run it.

Byte-oriented service dispatch may implement an ABI or transport boundary. It is not the canonical embedded dependency model.

## Core ownership

`phenix-core` owns fundamental types and mechanisms, including:

```text
agent and invocation primitives
model request/response and cancellation contracts
tool identity/schema/invocation/result contracts
skill identity/content contracts
flat session identity/lifecycle/input/output contracts
context attachment contracts
component/plugin/interface identities
resolved component handles and authority attenuation
persistence namespaces and transactions
events, tasks, cancellation, isolation, and provenance
```

Core does not instantiate session, model, tool, skill, or context providers by itself.

The flat core session contract contains no parent, child, branching, or tree operation. Session lineage belongs to a separate plugin component and interface.

## Basic implementation ownership

A minimal product may select deliberately small first-party components such as:

```text
phenixPlugins.basic-sessions
phenixPlugins.basic-model
phenixPlugins.basic-tools
phenixPlugins.basic-skills
phenixPlugins.basic-context
```

Names may differ while the package split is being completed. The invariant is that each implementation is an ordinary plugin-owned component selected through the same public component model available to third parties.

These components should remain policy-light. Rich routing, discovery, ranking, compaction, trees, history, orchestration, hooks, jobs, and diagnostics stay in focused plugins.

## Required invariants

- Core can be constructed and used as a mechanisms/contracts library with zero behavior plugins.
- Omitting a basic component means that implementation is absent. Core does not silently replace it.
- A basic first-party component and a third-party replacement use the same typed interface and Harness binding mechanism.
- Consumer code depends on the interface, not a concrete implementation.
- First-party status grants no binding priority or extra authority.
- Configured interposition behaves the same over a basic, replacement, embedded, or external terminal component.
- Trust-boundary invariants remain kernel guarantees rather than plugin policy.
- Session-tree semantics never leak into the flat core session contract.

## Minimal product composition

The supported minimal product composition should prove that ordinary components can provide the basic agent journey without privileged core behavior:

1. select a flat session component;
2. accept input;
3. bind required model, tool, skill, and context interfaces;
4. perform one deterministic model interaction;
5. persist and restore the minimum state owned by the selected components;
6. omit richer management plugins without causing core-owned fallback behavior.

## Replacement proofs

For each basic replaceable interface, coverage should show:

```text
basic component selected through the component graph
replacement component satisfies the same import
consumer request shape remains unchanged
omitting both leaves the import unresolved rather than invoking a hidden fallback
binding authority is attenuated by Harness, caller, plugin, and component bounds
```

At least one proof must use an external plugin component to prevent embedded-only privileges.

## Completion rule

This contract is complete when `phenix-core` contains contracts and mechanisms only, the minimal product is assembled from ordinary plugin-owned components, and no core-owned default provider or session-tree operation remains as a hidden fallback.
