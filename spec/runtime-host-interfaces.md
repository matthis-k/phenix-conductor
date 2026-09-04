# Stable runtime and host interfaces

status: specification-only

## Goal

Give Core and Plugin code a small fixed typed API for common runtime behavior while keeping replaceable behavior implemented by ordinary Components and Plugins.

Core may own stable Interface contracts and ergonomic typed Interface Handles. Core does not thereby own the selected Provider implementation. Provider selection follows `spec/plugin-resolution.md`.

```text
Plugin code
    |
    | typed Interface Handle
    v
stable Interface
    |
    | Provider Binding
    v
Provider Component
```

For example:

```rust
ctx.context.compact(request).await?
```

is an ergonomic call through the resolved `phenix.context.compact@1` Interface. It is not a hard-coded compaction implementation and it is not a live service lookup by string.

## Vocabulary

Use these terms consistently:

- **Runtime Interface**: stable typed callable Interface used by Plugin Runtime code.
- **Domain Interface**: Runtime Interface for product behavior Phenix performs.
- **Host Interface**: Runtime Interface for controlled interaction with the environment outside the Resolved Graph.
- **Provider Binding**: Graph Generation-pinned association between an Interface Import and selected Provider, as defined by `spec/plugin-resolution.md`.
- **Interface Handle**: typed reference to one resolved Provider Binding.
- **Provider**: Component that Exports an Interface implementation.
- **Layer**: ordered interposition around a resolved Terminal Provider.
- **Host Capability**: authority-bearing kernel/environment handle supplied through `PluginHost`, as defined by `spec/plugin-host.md`.

A Host Interface and a Host Capability are not synonyms. A Host Interface is an ordinary resolved Interface contract. Its Provider may internally use one or more Host Capabilities to perform authorized environment operations.

Do not call these executable contracts `options`. Options are configuration values.

## Core boundary

Core may define broadly useful Interface types and typed accessors when they provide stable vocabulary across many Components.

Examples of Domain Interfaces may include:

```text
phenix.context.compact@1
phenix.models.infer@1
phenix.tools.execute@1
phenix.skills.resolve@1
phenix.sessions.create@1
phenix.sessions.persist@1
phenix.orchestration.invoke@1
```

Examples of Host Interfaces may include:

```text
phenix.host.filesystem@1
phenix.host.process@1
phenix.host.network@1
phenix.host.clock@1
phenix.host.credentials@1
phenix.host.frontend@1
phenix.host.terminal@1
```

This list does not require every Interface immediately. Add a Core-owned Interface only when multiple runtime Components need the same stable semantic boundary.

Core must not grow a default Provider implementation merely because it owns the Interface type.

## Typed runtime context

Normal embedded Rust Plugin code uses typed Interface Handles exposed through `PluginContext` or the equivalent resolved Plugin Runtime context.

Desired shape:

```rust
ctx.context.compact(request).await?;
ctx.models.infer(request).await?;
ctx.host.filesystem.read(path).await?;
ctx.host.process.spawn(command).await?;
ctx.host.network.request(request).await?;
```

The exact Rust layout may differ, but these invariants are required:

- callers do not resolve Providers by string at invocation time;
- required Provider Bindings are validated before activation;
- typed request/response contracts use `PhenixValue` and `PhenixSchema` at structural Plugin API boundaries;
- normal embedded calls remain statically typed;
- an Interface Handle cannot grant more Effective Authority than the resolved Provider Binding permits;
- no hidden Provider fallback occurs after invocation failure.

Generic structural dispatch remains explicit ABI, inspection, Adapter, or Runtime Provider plumbing. It is not the normal Rust authoring API.

## Domain interfaces

Domain Interfaces answer:

> How does Phenix perform this product behavior?

`phenix.context.compact@1` is the motivating example.

The neutral contract owner defines request, response, error, and typed Interface Handle vocabulary. A selected Plugin Provider implements the behavior.

Possible Providers may implement deterministic truncation, summarization, semantic compaction, model-assisted compaction, or a custom third-party policy.

Callers do not depend on Provider identity. Product Composition Policy may replace the Provider declaratively. Replacement creates a new Graph Generation rather than mutating an invocation in flight.

## Host interfaces

Host Interfaces answer:

> Through which resolved Interface may a Plugin request this environment-facing behavior?

Plugin code should prefer Host Interface Handles over ambient operating-system access after bootstrap when the architecture supplies that Interface.

Examples include filesystem, process, network, clock, credential, frontend, and terminal Interfaces.

A Host Interface Provider may be local, sandboxed, delegated through an Adapter, virtualized for tests, or a deny Provider.

A Host Interface must not silently turn an unavailable Provider Binding into ambient OS access.

The Provider behind a Host Interface may itself require Host Capabilities from `PluginHost`. Those Host Capabilities remain separately authority-bounded; resolving a Host Interface does not create ambient authority.

## Authority

Host Interface Handles are authority-sensitive references to resolved Provider Bindings. Host Capabilities are authority-bearing kernel/environment handles. Neither expands authority.

A Provider may request authority in metadata, but the kernel resolver computes the Effective Authority from Product Composition Policy, caller authority, Provider limits, and Interface requirements.

Authority cannot expand through delegation, Layer interposition, retry, reconnect, Provider replacement, or conversion between an Interface Handle and a Host Capability.

Security-sensitive environment operations fail explicitly when no compatible authorized Provider Binding and required Host Capability path exist.

## Resolution

Runtime Interfaces use the existing Component Graph and kernel resolver. This specification does not define a second resolver.

A Component declares Interface Imports. Provider Components declare compatible Exports. The kernel resolver selects and validates Providers while constructing the candidate Graph Generation.

Required unresolved Imports fail before activation. Optional Imports are explicitly optional. Equivalent Provider Candidates resolve deterministically according to `spec/plugin-resolution.md`.

Provider failure after dispatch is an execution failure. It does not trigger live Provider search. Changing a Provider requires a new valid Graph Generation.

## Interface cardinality

Do not force every extension mechanism through one invocation model.

- **Single Provider Binding:** ordinary replaceable behavior such as context compaction or clock access.
- **Provider Binding plus Layers:** ordered interposition around one Terminal Provider.
- **Events and Listeners:** fan-out observation and reactions, not Provider selection.
- **Controllers:** background convergence, not fake synchronous Interfaces.
- **Plugin Resources:** Durable State, not callable Interfaces.

## No service locator

This architecture must not introduce a generic live service locator.

Bad:

```rust
ctx.call("phenix.context.compact", value).await?
```

Normal embedded code should use a typed Interface Handle:

```rust
ctx.context.compact(request).await?
```

A generic dynamic Interface lookup may exist only at explicit ABI, inspection, configuration, Adapter, or Runtime Provider boundaries where static Rust types cannot cross directly.

## Defaults

A default Harness may select ordinary first-party Providers for common Domain and Host Interfaces.

Defaults remain explicit Product Composition Policy. Omitting or replacing a default Provider must not reveal a hidden Core implementation.

For a required Interface, omission without a replacement is a resolver error. For an optional Interface, the typed API represents absence explicitly.

## Adapters

Protocol Adapters may provide or consume Host Interfaces without becoming privileged runtime paths. Application integration terminology belongs to `spec/application-integration-terminology.md`.

For example, `phenix-adapter-acp` may translate Application-side ACP callback capabilities into an ordinary `phenix.host.frontend@1` Provider. Plugin Runtime code still invokes the Host Interface rather than ACP-specific functions.

Another Adapter can support the same Plugin Runtime when it supplies a compatible Host Interface Provider.

## Testing

Host Interfaces are preferred seams for deterministic tests of environment interaction.

Tests may bind in-memory or fake Providers for filesystem, process, network, clock, credentials, terminal, or frontend behavior without giving the tested Component ambient authority.

A fake Provider must satisfy the same Interface and Effective Authority semantics as a production Provider.

## Required regressions

Implementation must prove:

- a call such as context compaction dispatches through a pre-resolved typed Interface Handle;
- interchangeable Providers can be selected declaratively without caller changes;
- missing required Providers fail Graph resolution before activation;
- invocation failure does not trigger Provider fallback;
- Host Interfaces can be supplied by replaceable Providers;
- denied Host Capability authority cannot be recovered through an ambient path;
- Interface Handles preserve Effective Authority attenuation;
- Layers wrap one preselected Terminal Provider in deterministic order;
- Events remain fan-out rather than callable Provider Interfaces;
- fake Providers can replace environment access without changing Component code;
- embedded Rust callers do not need string-based service lookup;
- Runtime Providers can use the same Interface semantics through structural dispatch;
- changing a Provider produces a new Graph Generation and does not mutate pinned invocations.

## Completion

- [ ] Core defines a small stable typed Runtime Interface mechanism;
- [ ] Domain Interfaces and Host Interfaces are distinct roles over the same Provider Binding machinery;
- [ ] `PluginContext` exposes ergonomic typed Interface Handles;
- [ ] Host Interface and Host Capability remain explicitly distinct concepts;
- [ ] `phenix.context.compact@1` is replaceable rather than hard-coded behavior;
- [ ] environment interaction can be routed through typed Host Interfaces and authority-bounded Host Capabilities;
- [ ] required Provider Bindings are resolved before activation;
- [ ] no hidden fallback Provider exists;
- [ ] no generic service locator becomes the normal embedded authoring API;
- [ ] Layers, Events, controllers, and Plugin Resources remain separate mechanisms;
- [ ] default implementations remain ordinary selectable first-party Providers;
- [ ] exact-head Source, Rust, Product, and Maintenance validation passes.
