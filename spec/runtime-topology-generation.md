# Runtime topology generations

status: implemented
coverage:
  - rust/crates/phenix-core/src/runtime_topology_generation_regression.rs
  - rust/crates/phenix-sdk/tests/plugin_attribute_only_gate.rs

## Purpose

One resolved graph generation is the source of truth for executable runtime topology.

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

## Resolved Topology

A resolved generation contains enough information to activate the executable topology without asking plugin instances to discover or register routing semantics.

The resolved form owns:

- graph generation identity;
- component and function ownership;
- resolved service chains and layer order;
- terminal implementation selection;
- listener identities, event contracts, dependency ordering, failure policies, and owner identity;
- resource projections;
- lifecycle ownership and restart requirements.

Runtime handler objects may remain erased implementation bindings, but their topology and ownership come from the resolved generation.

## Plugin Instances

`PluginInstance` is an execution, lifecycle, and state container. It is not a routing abstraction.

An instance may provide callable implementations and listener handlers for the nodes owned by its plugin. It must not mutate global routing, choose service chains, or create hidden subscriptions that are absent from the resolved generation.

Factories answer only how to construct an implementation for a generation. Factory registration is not topology.

## Activation

Candidate resolution finishes before live mutation.

Activation follows this order:

1. Resolve and validate the complete candidate generation.
2. Compute the transition from the active generation.
3. Stage new or restarted implementation instances and runtime bindings.
4. Keep the active generation unchanged if staging fails.
5. Install candidate config, component graph, service topology, listener topology, resources, and generation identity as one transition.
6. Retire removed or replaced bindings and instances.
7. Preserve unchanged instances when the transition permits it.

No caller may observe a mixture of old service topology and new listener topology, or the reverse.

## Load And Unload

Loading a plugin means resolving a candidate generation that contains its contributions, then activating that generation.

Unloading a plugin means resolving a candidate generation without that plugin, then activating that generation.

Unload removes every executable binding owned only by the retired plugin, including service implementations, layers, listeners, and generation-local resources. No stale handler remains reachable after the new generation becomes active.

Dependency closure and restart policy determine which other plugins restart. Unchanged compatible implementations remain active.

## Listener Topology

Listener declarations are declarative metadata before activation.

A resolved listener entry contains:

```text
subscription id
owner plugin
owner component/function
source event type + version
payload schema + projection
dependency edges
failure policy
required and maximum authority
graph generation attribution
runtime handler binding
```

The event bus executes the resolved entries. Plugin `start` and `stop` may manage plugin-local resources, but they are not the canonical source of listener registration or removal.

Generated and manually constructed Core fixtures use the same event transport, authority attenuation, dependency ordering, recursion policy, and delivery failure policy.

## Service Topology

Service chains are resolved before activation. Layers wrap the next resolved node through the canonical continuation mechanism. A plugin instance cannot insert an extra layer by mutating live routing during `start`.

Adding, removing, or reordering a layer therefore requires a new resolved generation.

## Reconciliation

Live reconciliation compares complete generations, not independent mutable registries.

The transition plan accounts for changes to:

- plugin manifests;
- component and function ownership;
- service chains and layer order;
- listener entry points and listener dependency order;
- resources and configuration;
- runtime bindings and lifecycle policy.

A topology change that changes executable bindings changes the generation identity.

## Inspection

Inspection reports the active resolved generation and its executable topology. It is possible to determine which plugin owns a callable node, which layers precede it, which listeners consume an event, and which generation installed each binding without inspecting plugin-local mutable state.

## Invariants

- One resolved generation is the source of truth for executable topology.
- Plugins provide behavior, state, lifecycle, and declarations. The kernel owns routing.
- Components and functions are executable nodes.
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

## Validation

Regression coverage proves plugin addition and removal, service and listener replacement, failed candidate rollback, retained instances, retired handlers, resource-only plugins, listener DAG rejection, and generation attribution. Static SDK coverage proves generated listener declarations and handlers use the same resolved activation path.
