# Stable runtime and host interfaces

status: specification-only

## Goal

Give core and plugin code a small fixed typed API for common runtime behavior while keeping every replaceable behavior implemented by ordinary components/plugins.

Core owns the stable interface contracts and ergonomic typed handles. It does not own rich default implementations.

The canonical resolver binds each required interface before activation.

```text
runtime code
    |
    | typed handle
    v
stable runtime interface
    |
    | resolved binding
    v
plugin/component implementation
```

For example:

```rust
ctx.context.compact(request).await?
```

is an ergonomic call through the resolved `phenix.context.compact@1` interface. It is not a hard-coded compaction implementation and it is not a runtime lookup by string.

## Vocabulary

Use these terms consistently:

- **Runtime interface**: stable typed callable contract owned by core.
- **Domain interface**: runtime interface for behavior Phenix performs.
- **Host interface**: runtime interface for controlled interaction with the environment outside the resolved runtime.
- **Binding**: resolver-selected implementation of an interface.
- **Handle**: typed capability-bearing reference to one resolved binding.
- **Provider**: component exporting an implementation of an interface.
- **Interposition layer**: ordered wrapper around a resolved binding.

Do not call these contracts `options`. Options are configuration values. These interfaces are executable contracts.

## Core boundary

Core may define broadly useful interface types and typed accessors when they provide a stable vocabulary across many components.

Examples of domain interfaces may include:

```text
phenix.context.compact@1
phenix.models.infer@1
phenix.tools.execute@1
phenix.skills.resolve@1
phenix.sessions.create@1
phenix.sessions.persist@1
phenix.orchestration.invoke@1
```

Examples of host interfaces may include:

```text
phenix.host.filesystem@1
phenix.host.process@1
phenix.host.network@1
phenix.host.clock@1
phenix.host.credentials@1
phenix.host.frontend@1
phenix.host.terminal@1
```

This list is not a requirement to create every interface immediately. Add a core-owned interface only when multiple runtime components need the same stable semantic boundary.

Core must not grow a default implementation merely because it owns the interface type.

## Typed runtime context

Normal embedded Rust code uses typed handles exposed through `PluginContext` or the equivalent resolved runtime context.

Desired shape:

```rust
ctx.context.compact(request).await?;
ctx.models.infer(request).await?;
ctx.host.filesystem.read(path).await?;
ctx.host.process.spawn(command).await?;
ctx.host.network.request(request).await?;
```

The exact Rust layout may differ, but these invariants are required:

- callers do not resolve providers by string at invocation time;
- required bindings are validated before activation;
- typed request/response contracts use the existing structural Phenix value/schema machinery where ABI boundaries require it;
- normal embedded calls remain statically typed;
- a handle cannot grant more authority than the resolver assigned to that binding;
- no hidden provider fallback occurs after invocation failure.

Generic byte/value dispatch remains ABI and external-host plumbing. It is not the normal authoring API.

## Domain interfaces

Domain interfaces answer:

> How does Phenix perform this behavior?

`phenix.context.compact@1` is the motivating example.

Core defines the request, response, error contract, and typed handle. A selected plugin provides the behavior.

Possible providers may implement:

- deterministic truncation;
- summarization;
- semantic compaction;
- model-assisted compaction;
- a custom third-party policy.

Callers do not depend on the provider identity.

A Harness may replace the provider declaratively. The replacement creates a new resolved graph generation rather than mutating an invocation in flight.

## Host interfaces

Host interfaces answer:

> How may Phenix interact with the environment outside the resolved runtime?

Runtime and plugin code should prefer host handles over direct ambient access to operating-system facilities after bootstrap.

Examples:

```text
ctx.host.filesystem
  controlled workspace/file access

ctx.host.process
  process execution

ctx.host.network
  outbound network access

ctx.host.clock
  time source

ctx.host.credentials
  secret/credential access

ctx.host.frontend
  user/application callbacks

ctx.host.terminal
  terminal operations when explicitly available
```

A host provider may be local, sandboxed, delegated through an adapter, virtualized for tests, or denied entirely.

For example:

```text
phenix.host.frontend@1
        |
        +--> ACP client callbacks
        +--> headless deny provider
        +--> test provider
```

or:

```text
phenix.host.network@1
        |
        +--> restricted HTTP provider
        +--> sandbox proxy provider
        +--> deny provider
```

Host interfaces must not silently turn unavailable capabilities into ambient OS access.

## Authority

Host handles are authority-bearing capabilities.

A provider may request broad authority in metadata, but the resolver computes the actual grant from Harness policy and the caller/provider relationship.

Example:

```text
requested:
  filesystem.read
  filesystem.write
  process.spawn

resolved grant:
  filesystem.read(workspace=/repo)
  process.spawn(commands=[git, rg])
```

The resulting handles expose only the granted operations/scope.

Authority cannot expand through delegation, interposition, retry, reconnect, or provider replacement.

Security-sensitive host operations must fail explicitly when no compatible authorized provider is bound.

## Resolution

Runtime interfaces use the existing typed component graph and canonical resolver.

A component declares imports such as:

```text
imports:
  phenix.context.compact@1
  phenix.host.filesystem@1
```

Providers declare exports:

```text
exports:
  phenix.context.compact@1
```

The resolver selects and validates providers while constructing `ResolvedHarness`.

Required unresolved imports fail before activation. Optional imports are explicitly optional.

Equivalent candidates are selected deterministically by policy and configuration, not registration order.

Provider failure during invocation is an error. It does not trigger provider search or fallback.

Changing a provider requires a newly resolved valid graph generation.

## Interface cardinality

Do not force every extension mechanism through one invocation model.

### Single binding

Use one selected provider for ordinary replaceable behavior:

```text
context.compact
sessions.persist
host.filesystem
host.clock
```

### Binding plus interposition

Use existing ordered interposition when behavior needs wrappers:

```text
caller
  -> audit layer
  -> rate-limit layer
  -> selected host.network provider
```

Interposition never changes the selected terminal provider after a call begins.

### Events and subscribers

Use events for fan-out observation and asynchronous reactions:

```text
session.created
execution.completed
tool.finished
```

Do not model event subscribers as callable runtime-interface providers.

### Controllers

Use controller/reconciler components for durable background convergence. Do not turn them into fake synchronous interfaces.

## No service locator

This architecture must not introduce a generic runtime service locator.

Bad:

```rust
ctx.call("phenix.context.compact", value).await?
```

Normal embedded code should look like:

```rust
ctx.context.compact(request).await?
```

The typed accessor may internally hold a resolved capability handle. Provider resolution is complete before the call.

A generic dynamic interface lookup may exist only at explicit ABI, inspection, configuration, or external component-host boundaries where static Rust types cannot cross directly.

## Defaults

A default Harness may select ordinary first-party providers for common runtime and host interfaces.

Defaults remain explicit composition policy.

Omitting or replacing a default provider must not reveal a hidden core implementation.

For a required interface, omission without a replacement is a resolver error. For an optional interface, the typed API represents absence explicitly.

## External adapters

Protocol adapters may provide or consume host interfaces without becoming privileged runtime paths.

Example:

```text
Application
   | ACP
   v
phenix.adapter.acp
   |
   +--> provides phenix.host.frontend@1 callbacks
   |
   v
resolved runtime
```

The ACP adapter translates external client capabilities into ordinary scoped host-interface providers. Runtime code still calls `ctx.host.frontend`, not ACP-specific functions.

The same runtime can therefore work with another adapter when it provides compatible host interfaces.

## Testing

Host interfaces are the preferred seam for deterministic tests of external interaction.

Tests may bind in-memory or fake providers for filesystem, process, network, clock, credentials, terminal, or frontend behavior without giving the tested component ambient authority.

A fake provider must satisfy the same typed contract and authority semantics as a production provider.

## Regression coverage

Implementation must prove:

- a runtime call such as context compaction dispatches through a pre-resolved typed handle;
- two interchangeable compaction providers can be selected declaratively without caller changes;
- missing required providers fail graph resolution before activation;
- invocation failure does not trigger provider fallback;
- host filesystem/process/network access can be supplied by replaceable providers;
- denied host authority cannot be recovered through another ambient code path;
- handles preserve authority attenuation;
- interposition wraps one preselected terminal provider in deterministic order;
- events remain fan-out rather than single-provider callable interfaces;
- test providers can replace host access without changing component code;
- embedded Rust callers do not need string-based service lookup;
- external component hosts can use the same interface semantics through ABI dispatch;
- changing a provider produces a new graph generation and does not mutate pinned executions.

## Completion

- [ ] core defines a small stable typed runtime-interface mechanism;
- [ ] domain and host interfaces are distinct roles over the same resolver/binding machinery;
- [ ] `PluginContext` exposes ergonomic typed capability handles;
- [ ] `phenix.context.compact@1` is implemented as a replaceable resolved interface rather than hard-coded behavior;
- [ ] external environment interaction can be routed through typed host interfaces;
- [ ] authority is carried and attenuated by host handles;
- [ ] required bindings are resolved before activation;
- [ ] no hidden fallback provider exists;
- [ ] no generic service locator becomes the normal embedded authoring API;
- [ ] interposition, events, controllers, and resources remain separate mechanisms;
- [ ] default implementations remain ordinary selectable first-party components/plugins;
- [ ] exact-head Source, Rust, Product, and Maintenance validation passes.
