# Plugin SDK context

Status: implementation contract.

## Purpose

Define the author-facing runtime boundary between a plugin instance, the kernel, and the selected SDK.

The core runtime stays generic. It passes `PluginHost` to a plugin instance. The SDK authoring layer adapts that callback into the `PluginContext` exposed to plugin code.

## Runtime context

`PhenixPlugin` start and invocation callbacks receive one `PluginContext` value:

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

The fields have distinct ownership:

- `kernel`: scoped access to generic kernel mechanisms. It is not a mutable `Kernel` reference.
- `sdk`: typed userspace clients bound to the component handling the callback.
- `plugin`: the current plugin identity plus borrowed instance settings and mutable instance state.
- `call`: metadata scoped to the current kernel-mediated callback.

`PluginContext` is a borrowed view. It does not own kernel state, SDK providers, settings, or plugin state.

`PhenixPluginAdapter` is the dynamic core `PluginInstance`. It owns one settings value and one mutable state value. For each host-backed callback it borrows those values into a fresh `PluginContext` and passes that context to the static `PhenixPlugin` definition.

The core `stop` callback has no host. `PhenixPlugin::stop` receives borrowed settings and state directly.

This keeps plugin state in one place. A plugin does not have hidden mutable implementation state alongside `context.plugin.state`.

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

Examples: session summaries, model information, artifact metadata, and test results.

### Capability objects

Stateful or behavioral objects cross the boundary as typed handles.

Examples: sessions, artifacts, workers, terminals, and test runs.

A capability object carries stable identity and a typed client. Operations remain kernel-mediated, so provider resolution and authority checks still apply. Handles are callback-scoped because their clients borrow the current runtime host. Persistent plugin state should store stable object identity and reacquire a client on the next callback.

Raw Rust references to another plugin's internal state are not part of an SDK contract.

## Recursive calls

An SDK call may invoke another plugin, and that provider may perform further SDK calls.

Each provider using the authoring adapter receives a fresh `PluginContext` for its host-backed callback. The kernel remains responsible for authority attenuation, component bindings, provenance, and cycle protection across the call chain.

## Static and dynamic forms

The author-facing split is:

```text
PhenixPlugin                 static behavior contract
  Settings                   immutable instance configuration
  State                      mutable private instance state

PhenixPluginAdapter          dynamic core PluginInstance
  default_component
  settings
  state

PluginContext                borrowed callback projection
  kernel
  sdk
  plugin
  call
```

A plugin may have multiple components. Component callbacks bind the SDK to the actual component supplied by the kernel. Legacy service callbacks use the adapter's declared default component.

## Invariants

- Core owns generic plugin execution; the SDK layer owns the author-facing context adapter.
- Plugin settings and state have one dynamic owner: `PhenixPluginAdapter`.
- `PluginContext` borrows settings and mutable state; it does not copy or own them.
- `ctx.sdk` is the typed public dependency namespace available from the current component.
- Cross-plugin access requires an explicit typed import and remains kernel-mediated.
- Plugin-owned state is private by default.
- Stateful SDK objects are handles, not shared provider-internal references.
- `ctx.kernel` is scoped kernel access, never unrestricted mutable kernel access.
- Recursive SDK calls cannot expand effective authority.
