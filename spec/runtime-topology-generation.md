# Runtime topology generations

Status: implementation contract.

## Purpose

Make one resolved graph generation the source of truth for executable runtime topology.

Plugins declare behavior and own state. They do not own routing. Components expose executable functions. Resolution turns those declarations into call nodes, service interposition, event entry points, resources, and lifecycle ownership. Loading or unloading a plugin produces a new graph generation and activates it atomically.

This extends the plugin authoring, service layering, event, reconciliation, and plugin host contracts.

## Model

A plugin contributes:

- executable component functions;
- service exports and terminal implementations;
- layers around service calls;
- listeners for completed events;
- resources and metadata;
- lifecycle and state ownership.

The resolved generation owns how those contributions connect.

```text
service call -> layer -> layer -> terminal function

event -> listener function
      -> listener function
```

Hooks are authoring syntax. An intercepting hook lowers to a layer. An observational hook lowers to a listener. There is no hook runtime.

Listeners are event-triggered entry points. They are not service-chain edges and cannot affect the operation that emitted the event.

## Resolved topology

A resolved generation contains enough information to activate the executable topology without asking plugin instances to discover or register routing semantics.

At minimum the resolved form owns:

- graph generation identity;
- component/function ownership;
- resolved service chains and layer order;
- terminal implementation selection;
- listener identities, event contracts, dependency ordering, failure policies, and owner identity;
- resource projections;
- lifecycle ownership and restart requirements.

Runtime handler objects may remain erased implementation bindings, but their topology and ownership come from the resolved generation.

## Plugin instances

`PluginInstance` is an execution, lifecycle, and state container. It is not a routing abstraction.

An instance may provide callable implementations and listener handlers for the nodes owned by its plugin. It must not mutate global routing, choose service chains, or create hidden subscriptions that are absent from the resolved generation.

Factories answer only how to construct an implementation for a generation. Factory registration is not topology.

## Activation

Candidate resolution finishes before live mutation.

Activation follows this order:

1. Resolve and validate the complete candidate generation.
2. Compute the transition from the active generation.
3. Stage new or restarted implementation instances and runtime bindings.
4. If staging fails, keep the active generation unchanged.
5. Atomically install the candidate config, component graph, service topology, listener topology, resources, and generation identity.
6. Retire removed or replaced bindings and instances.
7. Preserve unchanged instances when the transition permits it.

No caller may observe a mixture of old service topology and new listener topology, or the reverse.

## Load and unload

Loading a plugin means resolving a candidate generation that contains its contributions, then activating that generation.

Unloading a plugin means resolving a candidate generation without that plugin, then activating that generation.

Unload removes every executable binding owned only by the retired plugin, including service implementations, layers, listeners, and generation-local resources. No stale handler may remain reachable after the new generation becomes active.

Dependency closure and restart policy determine which other plugins must restart. Unchanged compatible implementations may remain active.

## Listener topology

Listener declarations are declarative metadata before activation.

A resolved listener entry contains at least:

```text
subscription id
owner plugin
owner component/function
source event type + version
dependency edges
failure policy
graph generation
runtime handler binding
```

The event bus executes the resolved entries. Plugin `start` and `stop` may manage plugin-local resources, but they are not the canonical source of listener registration or removal.

The same event transport, authority attenuation, dependency ordering, recursion policy, and delivery failure policy apply to generated and manually constructed Core fixtures.

## Service topology

Service chains are resolved before activation. Layers wrap the next resolved node through the canonical continuation mechanism. A plugin instance cannot insert an extra layer by mutating live routing during `start`.

Adding, removing, or reordering a layer therefore requires a new resolved generation.

## Reconciliation

Live reconciliation compares complete generations, not independent mutable registries.

The transition plan must account for changes to:

- plugin manifests;
- component/function ownership;
- service chains and layer order;
- listener entry points and listener dependency order;
- resources and configuration;
- runtime bindings and lifecycle policy.

A topology change that changes executable bindings must change the generation identity.

## Inspection

Inspection reports the active resolved generation and its executable topology. It must be possible to answer which plugin owns a callable node, which layers precede it, which listeners consume an event, and which generation installed each binding without inspecting plugin-local mutable state.

## Invariants

- One resolved generation is the source of truth for executable topology.
- Plugins provide behavior, state, lifecycle, and declarations. The kernel owns routing.
- Components/functions are executable nodes.
- Layers are service interposition in resolved call chains.
- Listeners are event-triggered entry points.
- Hooks lower to layers or listeners and have no separate runtime.
- `PluginInstance` does not own global topology.
- Factories construct implementations and do not register routing.
- Load and unload activate complete candidate generations.
- Generation replacement is atomic across service and listener topology.
- Failed staging leaves the active generation unchanged.
- Retired generations leave no reachable listener or service binding behind.
- Unchanged compatible instances may survive a generation transition.

## Required regressions

- adding a plugin activates its service nodes and listeners in one generation transition;
- removing a plugin removes its service nodes and listeners in one generation transition;
- a failed candidate listener or service binding leaves the previous generation fully active;
- unchanged plugin instances survive a topology change when restart policy permits it;
- changed layer order produces a new resolved chain without plugin-local registration;
- listener dependency changes produce a new resolved event topology;
- retired listener handlers cannot receive events after unload;
- retired layers cannot receive calls after unload;
- inspection attributes every executable binding to one active generation;
- no plugin lifecycle method can create routing that is absent from the resolved generation.