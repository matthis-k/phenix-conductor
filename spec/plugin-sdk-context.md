# Plugin SDK context

status: implemented

## Purpose

Define the runtime boundary between a plugin instance, the kernel, and the selected SDK.

The core `PluginInstance` ABI receives `PluginHost`. A plugin callback immediately projects that host and its instance data into `PluginContext`. Business logic receives the context instead of `PluginHost`.

Generic context types live in `phenix-core`. The default `PhenixSdk` remains in `phenix-plugin-sdk`.

## Runtime context

Plugin business logic receives one `PluginContext` value:

```text
PluginContext
  kernel
  sdk
  plugin
    id
    settings
    state
  call
    authority
    graph_generation
```

The fields have distinct roles:

- `kernel`: scoped access to generic kernel mechanisms. It is not a mutable `Kernel` reference.
- `sdk`: typed userspace clients bound to the component handling the callback.
- `plugin`: the current plugin identity plus the settings and state view selected by the plugin instance.
- `call`: metadata scoped to the current kernel-mediated callback.

`PluginContext` always borrows the live `PluginHost`. Its `Settings` and `State` slots are generic. A stateful plugin normally supplies `&Settings` and `&mut State`. A stateless plugin may supply `()`. A plugin with process-local resources may supply another callback-scoped handle.

The context does not create a second owner for plugin state. The dynamic `PluginInstance` remains responsible for its live state and constructs a fresh context for each host-backed callback.

The core `stop` callback has no host, so it cannot construct a runtime context. Stop logic works directly with data owned by the plugin instance.

## ABI adapter

`PluginInstance` methods are the runtime ABI adapter. They should parse the request, construct the appropriate context, call business logic, and serialize the response.

Business logic should not receive `PluginHost`. This keeps host access grouped into four explicit surfaces:

```text
ctx.kernel  generic kernel mechanisms
ctx.sdk     typed component imports
ctx.plugin  current plugin data
ctx.call    current call metadata
```

A plugin may define a local context alias and an SDK dependency struct for its declared imports. This makes unavailable dependencies absent from the business-logic type.

The `phenix_context` helper constructs a context with the default `PhenixSdk`. The caller still chooses which settings and state view to place in `ctx.plugin`.

## SDK access

The selected SDK belongs to the environment around the plugin, so it is part of `PluginContext`.

The default Phenix SDK exposes typed clients such as:

```text
ctx.sdk.sessions
ctx.sdk.models
ctx.sdk.tools
ctx.sdk.skills
ctx.sdk.context
ctx.sdk.options
ctx.sdk.config
```

A client is scoped to the component handling the callback. Constructing the SDK does not bypass dependency checks. Each invocation still goes through that component's declared import, resolved provider, and effective authority.

Plugins can add typed SDK contracts and access them through:

```rust
let testing = ctx.sdk.require::<TestingSdk>();
```

The returned client is only useful if the caller component declared and resolved the matching import. The kernel rejects undeclared or unbound calls.

## Cross-plugin data

Plugin-owned settings, state, internal resources, and implementation objects are private.

Another plugin may access data only through an explicitly exported typed contract. The provider owns the underlying state and decides which operations and values the contract exposes.

The normal form for stateful data is a typed SDK handle:

```rust
let testing = ctx.sdk.require::<TestingSdk>();
let run = testing.runs().find("run-123")?;
let result = run.result()?;
```

`run` is a consumer-side capability object. It contains stable identity plus a scoped client back to the provider. It is not a shared reference to provider internals.

## SDK objects

SDK contracts may expose two kinds of objects.

### Value objects

Self-contained immutable data may cross the contract boundary by value.

Examples include session summaries, model information, artifact metadata, and test results.

### Capability objects

Stateful or behavioral objects cross the boundary as typed handles.

Examples include sessions, artifacts, workers, terminals, and test runs.

A capability object carries stable identity and a typed client. Operations remain kernel-mediated, so provider resolution and authority checks still apply. Handles are callback-scoped because their clients borrow the current runtime host. Persistent plugin state should store stable object identity and reacquire a client on the next callback.

Raw Rust references to another plugin's internal state are not part of an SDK contract.

## Recursive calls

An SDK call may invoke another plugin, and that provider may perform further SDK calls.

Each provider constructs a fresh `PluginContext` for its host-backed callback. The kernel remains responsible for authority attenuation, component bindings, provenance, and cycle protection across the call chain.

## Ownership split

The runtime split is:

```text
phenix-core
  PluginContext
  KernelAccess
  SdkClient
  SdkContract
  SdkObject

phenix-plugin-sdk
  PhenixSdk
  phenix_context

plugin crate
  PluginInstance ABI adapter
  instance settings and state
  typed business logic
```

A plugin may have multiple components. Component callbacks bind SDK clients to the component supplied by the kernel. A legacy service callback must select the component that declares its imports.

## Invariants

- Core owns generic plugin execution and generic context mechanisms.
- The SDK crate owns the default Phenix userspace namespace and authoring helper.
- A plugin instance remains the single owner of its mutable instance state.
- A context carries an explicit settings and state view. It does not invent or copy hidden state.
- Plugin business logic receives `PluginContext`, not `PluginHost`.
- Typed component imports are available through `ctx.sdk`.
- Cross-plugin access requires an explicit typed import and remains kernel-mediated.
- Plugin-owned state is private by default.
- Stateful SDK objects are handles, not shared provider-internal references.
- Generic kernel mechanisms are available through `ctx.kernel`, never an unrestricted mutable kernel reference.
- Call authority and graph generation are available through `ctx.call`.
- Recursive SDK calls cannot expand effective authority.
