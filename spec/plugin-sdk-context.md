# Plugin SDK context

Status: implementation contract.

## Purpose

Define the runtime boundary between a plugin, the kernel, the selected SDK, and plugin-owned data.

A plugin definition is static. A plugin instance and every invocation context are dynamic.

## Runtime context

Plugin callbacks receive one `PluginContext` value:

```text
PluginContext
  kernel
  sdk
  plugin
  call
```

The fields have distinct ownership:

- `kernel`: scoped access to generic kernel mechanisms. It is not a mutable `Kernel` reference.
- `sdk`: typed capabilities exported by other userspace plugins and resolved for this plugin.
- `plugin`: data owned by the current plugin instance, including settings, state, and private resources.
- `call`: data scoped to the current invocation, including call identity, effective authority, provenance, and cancellation.

`PluginContext` is a borrowed runtime view. It does not own kernel state, SDK providers, or plugin state.

## SDK access

The selected SDK is part of `PluginContext` because it belongs to the environment around the plugin.

The default Phenix SDK may expose typed handles such as:

```text
ctx.sdk.sessions
ctx.sdk.models
ctx.sdk.tools
ctx.sdk.skills
ctx.sdk.artifacts
ctx.sdk.orchestration
```

SDK access is also the normal dependency path between plugins. A plugin declares required SDK contracts statically. At runtime `ctx.sdk` contains only contracts resolved and granted for that plugin.

A plugin must not inspect another plugin instance directly.

## Cross-plugin data

Plugin-owned settings, state, internal resources, and implementation objects are private.

Another plugin may access data only through an explicitly exported typed contract. The provider owns the underlying state and decides which operations and values the contract exposes.

The normal form is a typed SDK handle:

```rust
let testing = ctx.sdk.require::<TestingSdk>()?;
let run = testing.runs().find("run-123")?;
let result = run.result()?;
```

`run` is a consumer-side object handle. It contains stable identity plus a scoped client back to the provider. It is not a shared reference to provider internals.

## SDK objects

SDK contracts may expose two kinds of objects.

### Value objects

Self-contained immutable data may cross the contract boundary by value.

Examples: session summaries, model information, artifact metadata, test results.

### Capability objects

Stateful or behavioral objects cross the boundary as typed handles.

Examples: sessions, artifacts, workers, terminals, test runs.

A capability object contains stable identity and a typed client. Method calls remain kernel-mediated so authority, provider resolution, provenance, cancellation, reload, and external-plugin compatibility remain intact.

Raw Rust references to another plugin's internal state are not part of an SDK contract.

## Recursive calls

SDK calls may invoke another plugin, including a provider that itself performs SDK calls.

Each hop receives a new `PluginContext` with a new call identity. Parentage and provenance are preserved, and effective authority may only stay equal or attenuate across the call chain.

The kernel retains recursion and cycle protection.

## Static and dynamic forms

The authoring side defines an immutable `PluginDefinition`:

```text
PluginDefinition
  identity
  provides
    sdk
    services
    resources
    events
  requires
    sdk
    capabilities
    resources
  settings schema/defaults
  state schema
  factory
```

Runtime resolution produces a `PluginInstance` with concrete bindings, settings, state namespace, resources, and implementation.

Callbacks receive a scoped `PluginContext` projected from the resolved runtime.

## Invariants

- Plugin definitions are static; plugin instances and invocation contexts are dynamic.
- `ctx.sdk` is the typed public dependency namespace available to a plugin.
- Plugin-owned state is private by default.
- Cross-plugin access requires an explicit exported contract and authority.
- SDK capability objects are handles, not shared provider-internal references.
- SDK method calls remain kernel-mediated.
- `ctx.kernel` is a scoped kernel handle, never unrestricted mutable kernel access.
- Recursive SDK calls preserve provenance and cannot expand authority.
