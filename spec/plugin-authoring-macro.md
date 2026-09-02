# Phenix plugin specification

status: specification-only

## Status

This document defines the canonical Phenix runtime plugin model and Rust authoring API.

It is the adoption gate for first-party static runtime plugins. The implementation PR must not migrate first-party plugins until every static capability described here is representable through the Rust-native authoring surface without hand-written manifests, registries, factories, dispatch ladders, resource-registration lists, or other parallel wiring.

The central rule is:

> Plugins provide capabilities. The kernel composes capabilities into the runtime.

A plugin provides data, functions, and declarations that describe their meaning. The kernel validates, resolves, authorizes, composes, wires, activates, dispatches, and reconciles those contributions.

```text
Plugin
  = identity
  + metadata
  + configuration/schema
  + data/resources
  + implementations
  + handlers
  + declared relationships

Kernel
  = inspect
  + validate
  + resolve
  + authorize
  + compose
  + wire
  + activate
  + dispatch
  + reconcile

Runtime
  = one resolved graph generation
```

A plugin does not register itself into a live runtime. It does not construct a private service registry, event bus, hook runtime, dependency resolver, or alternate execution graph.

## 1. Terms

### Plugin

An independently activatable runtime contribution.

A plugin is the package, identity, lifecycle, trust, hosting, and durable ownership boundary for one set of capabilities.

A plugin may contain no executable behavior. A resource-only plugin is still a plugin when activation contributes runtime-owned resources.

### Component

A runtime composition unit owned by a plugin.

Components import and export typed interfaces. A plugin may contain zero, one, or many components.

### Interface

A stable, versioned semantic contract between components.

Provider and consumer Rust types may differ. Compatibility is structural.

### Contribution

Data supplied by a plugin for kernel interpretation.

Examples include components, imports, exports, events, listeners, layers, resources, configuration schemas, lifecycle handlers, controllers, and runtime requirements.

### Concrete plugin dependency

A dependency on one specific plugin implementation.

This relationship selects and recursively includes that plugin implementation.

### Interface import

A dependency on a capability rather than an implementation.

The kernel resolves an import to a compatible provider during graph construction.

### Runtime provider

A plugin that can host plugins implemented for a non-Core execution runtime.

Core initially provides only `embedded`. WASM, TypeScript, Python, process, or other runtimes use the same plugin model through runtime-provider plugins.

### Application integration roles

Applications, protocols, adapters, client SDKs, bindings, and transports remain distinct from runtime plugins.

```text
Application
  -> Binding / Client SDK
  -> Protocol
  -> Adapter
  -> Phenix
```

Transport only carries protocol data.

## 2. What qualifies as a runtime plugin

A package belongs in the runtime plugin catalog when it exposes an independently activatable plugin.

Examples:

```text
phenix-plugin-sessions
phenix-plugin-memory
phenix-plugin-command-toolbelt
phenix-plugin-repository-workers
phenix-plugin-runtime-wasm
```

The following are not runtime plugins unless they also expose independently activatable behavior:

```text
phenix-sdk
proc-macro crates
contract libraries
ordinary helper libraries
presets
catalogs
assembly packages
client SDKs
language bindings
transports
applications
```

Importing `phenix-sdk` starts nothing and changes no runtime graph.

A plugin may mostly expose functions for other code to invoke. That does not make it a passive library. The distinction is activation, not whether the implementation initiates work itself.

## 3. Package hygiene

Shared semantic contracts live in neutral passive owners.

A consumer must not depend on a default provider implementation only to use its request types, response types, IDs, or schemas.

Preferred dependency shape:

```text
consumer -> phenix-sdk / neutral contract owner
provider -> phenix-sdk / neutral contract owner
```

A runtime-plugin to runtime-plugin crate dependency is valid only when it represents one of:

1. an intentional concrete plugin dependency;
2. deliberate implementation reuse.

Contract-only implementation dependencies are invalid.

The runtime plugin catalog contains runtime plugins only. Presets and catalogs describe composition; they do not become plugins merely by describing plugins.

No compatibility package, old plugin ID, old contract alias, or parallel legacy authoring API is retained during prerelease migration.

## 4. Plugin identity

Every plugin has:

```text
PluginId
plugin version
artifact revision
execution/runtime identity
```

`PluginId` is semantic identity.

Plugin version normally derives from package metadata. Artifact revision identifies the exact executable or resource artifact. A graph generation pins it.

Nested plugin-owned identities should derive where safe:

```text
plugin:     phenix.example
component:  phenix.example.api
resource:   phenix.example.state
```

Explicit IDs remain available where an identity:

- is externally visible;
- must survive a Rust field rename;
- already exists and must be preserved;
- participates in a stable ABI;
- cannot be derived without ambiguity.

Cross-plugin interface identity must not derive from a provider's local field or method name.

Identifier precedence is:

1. explicit ID when supplied;
2. a type-provided canonical ID when the type owns stable identity;
3. deterministic parent plus item-name derivation for plugin-owned nested identities.

Migration must preserve established runtime identities even when authoring syntax changes.

## 5. Plugin shape

A plugin may provide any combination of:

| Contribution | Meaning |
| --- | --- |
| metadata | identity, version, compatibility, execution requirements |
| configuration | typed schema, defaults, namespaced settings |
| concrete dependencies | recursively included plugin implementations |
| components | runtime composition units |
| imports | required or optional capability dependencies |
| exports | provided typed capabilities |
| services | terminal/provider participation |
| layers | ordered synchronous interposition |
| hooks | authoring shorthand over canonical interposition/event mechanisms |
| events | asynchronous facts |
| listeners | reactions to events |
| controllers | kernel-scheduled background convergence |
| resources | durable state, packaged data, skills, indexes, other owned resources |
| public callables | invocable operations projected into clients/bindings |
| public values | read-only values projected into clients/bindings |
| lifecycle functions | preparation, start, stop behavior |
| migrations | durable schema transitions |
| host requirements | explicit authority-bearing host capabilities |
| runtime requirements | embedded or bridge-provided execution |

Omitting a category means that category is empty.

There is no separate mandatory SDK declaration tree, hook registry, event registry, or equivalent container.

## 6. Components own runtime behavior

Plugins own components. Components are the normal runtime composition unit.

```text
Plugin
  component A
    imports
    exports
    listeners
    layers

  component B
    imports
    exports
```

A plugin with one simple component may use authoring sugar that creates a deterministic default component. The kernel must still see a real component. Sugar must not create alternate runtime semantics.

## 7. Concrete dependencies and interface imports

These relationships are distinct.

### Concrete dependency

```rust
#[phenix(dep)]
sessions: phenix_plugin_sessions::Plugin,
```

This means: compose this specific sessions implementation with me.

The dependency type supplies its plugin identity. The author does not repeat the dependency's string ID.

The kernel recursively expands concrete dependencies and deduplicates runtime nodes by `PluginId`.

Diamond dependencies are legal when all paths resolve to the same compatible plugin definition.

The kernel rejects:

- incompatible definitions for one `PluginId`;
- dependency cycles;
- impossible runtime bootstrap chains;
- incompatible execution requirements.

### Interface import

```rust
#[phenix(import)]
models: Required<Call<ModelsInference, ModelRequest, ModelNeeds>>,
```

This means: bind me to a compatible provider of this interface.

It does not select the provider implementation. Provider replacement must not require consumer source changes.

### Required and optional

Optionality belongs in the type:

```text
Required<T>
Optional<T>
```

A missing `Required<T>` fails graph construction.

A missing `Optional<T>` produces absence. Optional imports never trigger hidden provider fallback.

## 8. Interface contracts

Interfaces have stable semantic IDs.

A neutral marker may own the ID:

```rust
#[phenix_sdk::interface("phenix.models.inference@1")]
pub struct ModelsInference;
```

The marker owns identity, not provider-specific request and response Rust types.

A consumer may use:

```rust
Required<Call<ModelsInference, ConsumerRequest, ConsumerNeeds>>
```

A provider may implement the same interface with different native types.

The kernel validates structural compatibility. This keeps provider and consumer implementations independent.

Literal interface IDs remain available for external or local contracts that do not define a marker.

## 9. Dynamic values and typed boundaries

`PhenixValue` is the canonical dynamic Phenix representation.

The normal boundary is:

```text
foreign/native value
  -> PhenixValue
  -> resolve semantic target
  -> parse into consumer/provider local type
  -> typed business logic
```

Convert foreign values into `PhenixValue` as soon as they cross into Phenix.

Convert `PhenixValue` into invariant-bearing native types as soon as the semantic target is known.

`serde_json::Value` belongs only at boundaries where JSON itself is the format. `std::any::Any` is not a public Phenix ABI.

### Structural matching

The common policy is:

```text
T           projected structural matching
Project<T>  explicit projected matching
Exact<T>    exact structural matching
```

The same wrappers apply to calls, listener payloads, hook inputs, provider inputs, responses, and other typed structural boundaries.

Do not add parallel APIs such as `invoke_exact` when the type already expresses exactness.

## 10. Resolution

One canonical resolver constructs the runtime.

Inputs include:

```text
configuration contributions
plugin definitions
component metadata
resources
skills
environment/deployment bindings
authority policy
artifact identities
```

Output:

```text
ResolvedHarness
  = immutable graph generation
```

The resolver owns:

- contribution merge and precedence;
- plugin selection;
- concrete dependency closure;
- component selection;
- import/export compatibility;
- provider binding;
- interposition ordering;
- authority grants;
- resource dependencies;
- durable ownership;
- runtime-provider resolution;
- environment binding validation;
- semantic generation identity.

No configuration frontend owns runtime topology. No plugin owns runtime topology.

## 11. Provider selection

Imports are resolved before activation.

Provider selection is deterministic and independent of registration order.

Provider failure after invocation begins is an execution failure. It does not trigger provider search or fallback.

Replacing a provider creates a new graph generation.

## 12. Services and layers

A normal export supplies a capability.

Where an interface allows one terminal provider, the resolver selects that terminal before dispatch.

Layers are ordered interposition around the resolved terminal:

```text
caller
  -> layer
  -> layer
  -> terminal
```

A layer may:

- transform input;
- handle the request;
- delegate once;
- deny;
- fail.

Continuation handles are kernel-issued and invocation-scoped. They remain bound to invocation, interface, chain position, authority, and graph generation.

An export and a service contribution are distinct Core concepts even when authoring sugar places both on one method. Generated Core representation must preserve both meanings.

Priorities, service roles, and authority are semantic and must never be silently inferred when doing so could change routing or access.

## 13. Hooks

Hooks are authoring concepts, not a second execution system.

A hook lowers to the appropriate canonical mechanism.

Use interposition when the hook can transform, deny, or wrap an operation.

Use an event/listener when it only observes something that already happened.

There is no privileged Core hook registry or second hook runtime.

## 14. Events

Events represent facts.

A listener runs after the event exists. A listener cannot retroactively reject or transform the operation that emitted the event.

Events use normal typed structural boundaries.

Listener failures follow kernel event failure policy and do not corrupt unrelated listeners. Structural mismatch produces the canonical kernel diagnostic rather than a panic.

Events do not grant additional authority.

## 15. Inbound invocation events

Application, UI, language, or protocol input is normalized into kernel-owned invocation data.

Conceptually:

```text
Lua value / ACP message / UI action
  -> adapter or binding
  -> PhenixValue
  -> InvocationEvent
  -> kernel validation
  -> target resolution
  -> typed dispatch
```

`InvocationEvent` is input to kernel dispatch. It is distinct from a plugin broadcast event.

Bindings and adapters do not resolve providers or invoke plugin implementations directly.

## 16. Controllers

Long-running or reactive behavior belongs in kernel-scheduled controllers.

A plugin declares controller behavior. The kernel owns scheduling, cancellation, lifecycle, authority, and graph-generation pinning.

Plugins should not create unmanaged background runtimes merely to implement recurring behavior.

Controllers can react to events, durable state, timers, or other declared inputs through kernel mechanisms.

## 17. Configuration

Plugins own the schema and meaning of their configuration. Configuration frontends own user-facing syntax.

For example:

```text
Nix
Lua
TOML
IPC
project discovery
GUI
```

may all lower into the same canonical `ConfigContribution`.

A configuration frontend may expose an arbitrary ergonomic API. It may not:

- register live providers;
- mutate the active graph;
- grant authority;
- bypass plugin/component/interface resolution.

Plugin configuration should be typed before plugin business logic receives it.

Feature defaults belong to the feature or plugin that owns their meaning.

## 18. Resources and persistence

Plugins may own durable and packaged resources.

Durable resources declare:

```text
stable resource identity
schema version
required backend features
migrations
ownership
lifecycle requirements
```

Migrations belong to the resource owner.

The kernel coordinates registration and migration. Plugin authors do not manually register namespaces during startup when the resource declaration already contains enough information.

A resource-only plugin is valid. Resource-only plugins cannot declare embedded dispatch behavior.

## 19. Authority

Plugins request authority. The kernel grants authority. These are distinct values.

A plugin can never increase authority granted by its parent composition or deployment policy.

Host interaction uses explicit injected capability handles for operations such as:

```text
filesystem
process
network
clock
credentials
terminal
frontend callbacks
```

Ambient access is not the runtime contract.

Concrete dependency inclusion does not automatically grant the dependency all authority held by its parent.

Interface bindings carry attenuated authority.

Runtime bridge authority is separate from guest plugin authority. Build authority is separate from the authority granted to the built plugin.

All resolved authority is pinned to the graph generation.

## 20. Execution runtimes

Core initially implements only:

```text
embedded
```

Runtime identity is open, not a closed Core enum.

Future runtimes such as:

```text
wasm
typescript
python
process
remote
```

are provided by ordinary runtime-provider plugins.

A runtime provider maps another execution environment onto the same Phenix plugin contract.

Runtime choice does not change plugin identity, interface semantics, authority rules, persistence ownership, lifecycle semantics, graph resolution, or provenance requirements.

Runtime-provider bootstrap chains must terminate at `embedded`. The kernel rejects runtime-provider cycles.

## 21. Plugin management

Load, unload, and replacement are kernel operations.

Plugins may request these operations only through a kernel-owned management interface with appropriate authority. They do not mutate the live graph themselves.

### Replacement

Loading a new artifact with an existing `PluginId` means replacement. There is no separate reload lifecycle.

### Build requests

A load request may contain either an already built artifact or structured build steps that produce an artifact.

Build steps use executable plus argument vectors. They are not shell strings.

The kernel coordinates the transaction through an authorized sandboxed build executor. The resulting artifact is hashed and pinned.

## 22. Graph generations

Stable operation uses one immutable resolved graph generation.

Running work is pinned to the generation under which it started.

A new plugin, removed plugin, new artifact, provider change, authority change, resource change, or relevant configuration change produces a candidate generation.

Development mode uses the same resolver as stable mode.

```text
active generation N
  -> changed inputs
  -> candidate generation N+1
  -> complete resolution
  -> validation
  -> preparation
  -> start candidate
  -> commit
  -> generation N+1 active
```

If build, validation, resolution, preparation, migration, or candidate startup fails, generation N remains active.

After commit, old generations drain. Retired instances stop after pinned work no longer requires them.

No live graph enters a partially resolved state.

## 23. Lifecycle

Canonical lifecycle is:

```text
inspect metadata
build artifact if needed
resolve candidate
validate candidate
prepare
start
commit generation
active
drain
stop
```

Metadata needed for graph construction must be inspectable without activating arbitrary plugin behavior.

Omitted lifecycle callbacks use kernel defaults.

Lifecycle methods do not register services, listeners, resources, or dependencies. Those relationships already come from plugin declarations.

## 24. Public API projection

A plugin may make selected capabilities available to applications and language bindings.

There is no separate SDK object declared by the plugin.

The resolved public API is projected from ordinary plugin contributions.

The initial projection has two plugin-defined categories:

```text
callables
read-only values
```

### Public callable

A normal export may be marked public.

The resolver already knows the semantic interface ID, request schema, response schema, authority requirements, documentation, and selected provider. Client SDKs and bindings project that metadata into their language.

### Public value

A plugin may expose a read-only value.

A public value may inspect plugin state but must not mutate runtime state or obtain additional authority as part of the read.

### Categories

Public paths are hierarchical.

Example:

```text
bench.run
bench.capabilities
memory.search
memory.stats
```

An omitted category contributes nothing.

Bindings do not require plugins to build a parallel SDK declaration tree.

Where a canonical interface ID follows the Phenix namespace convention, the default public path may derive from it. An explicit public path remains available where needed.

## 25. Language and client bindings

Application-side bindings remain clients, not runtime plugins.

For example:

```text
phenix-binding-lua
  -> phenix-client-acp
  -> ACP
  -> phenix-adapter-acp
  -> kernel
```

Runtime language hosts are different.

A runtime Lua, WASM, or TypeScript bridge plugin hosts guest plugin implementations and maps their values and functions onto the same Phenix plugin API.

At any foreign runtime boundary:

```text
foreign value
  -> PhenixValue
```

happens immediately.

The bridge does not create a language-specific composition system.

## 26. Inspectability

The kernel must expose enough resolved metadata to explain the active runtime.

At minimum:

```text
graph generation and semantic identity
selected plugins and artifact revisions
components
concrete dependency edges
imports and selected providers
interposition chains
resources and durable owners
requested and granted authority
execution runtime/provider
configuration source revisions
public callable/value projection
candidate versus active changes
reconciliation rejection reasons
```

Invocation provenance records:

```text
graph generation
caller
target interface
selected provider
layers
authority
runtime provider
outcome
```

## 27. Rust authoring principles

Rust plugin authoring should read like normal Rust implementing domain behavior.

Authors write:

- state;
- concrete dependencies they intentionally select;
- components;
- resources;
- imports they need;
- methods implementing behavior;
- semantic policy that cannot be derived.

Authors do not write:

- `PluginManifest`;
- `ComponentManifest`;
- owner IDs;
- repeated dependency IDs;
- factories;
- registries;
- dispatch tables;
- decode/encode ladders;
- resource registration;
- lifecycle adapters;
- transitive plugin collection;
- self-registration;
- re-export glue;
- a parallel SDK tree.

Prefer, in order:

1. normal Rust types, fields, methods, and trait semantics;
2. kernel-generic behavior;
3. small proc-macro annotations;
4. explicit metadata only for real semantic choices.

Macro syntax complexity should scale with unusual behavior, not with runtime plumbing.

## 28. Macro placement

The public authoring API lives under:

```rust
phenix_sdk
```

`phenix-sdk` remains passive.

The procedural implementation may live in an implementation-detail proc-macro crate such as:

```text
phenix-sdk-macros
```

because Rust requires proc macros to be compiled separately.

Plugin authors should normally depend only on `phenix-sdk`.

The macro implementation is not a plugin.

## 29. Canonical root macro

The canonical root API is an attribute macro:

```rust
#[phenix_sdk::plugin("phenix.example")]
```

A function-like `phenix_plugin! { ... }` DSL is not the canonical plugin declaration.

Once migration is complete, static first-party plugins must not retain a second function-like declaration format.

### Stateful form

A stateful plugin uses an ordinary struct:

```rust
#[phenix_sdk::plugin("phenix.planner")]
pub struct Plugin {
    #[phenix(dep)]
    models: phenix_plugin_models::Plugin,

    #[phenix(component)]
    api: Api,

    #[phenix(resource)]
    plans: Durable<PlanStore>,

    #[phenix(config)]
    config: Config,

    cache: Cache,
}
```

The struct is the source of truth.

### Stateless form

A stateless plugin should not need an empty struct:

```rust
#[phenix_sdk::plugin("phenix.bench")]
pub mod plugin {
    #[phenix(export("phenix.bench.run@1"), public)]
    pub fn run(
        ctx: &CallContext,
        request: BenchRequest,
    ) -> Result<BenchResult, Error> {
        // behavior
    }

    #[phenix(value("phenix.bench.capabilities@1"), public)]
    pub fn capabilities(
        ctx: &ReadContext,
    ) -> BenchCapabilities {
        // read-only value
    }
}
```

The module form generates any zero-sized plugin/component types required by static Rust composition. The author does not write them.

This keeps plugin identity and contribution boundaries without empty-struct ceremony.

A derive-based spelling may exist only if it produces exactly the same model and does not become a second authoring language.

## 30. Component macro

Components use:

```rust
#[phenix_sdk::component]
```

It may apply to a component struct and its runtime-facing impl.

Example:

```rust
#[phenix_sdk::component]
pub struct Api {
    #[phenix(import)]
    models: Required<Call<ModelsInference, ModelRequest, ModelNeeds>>,

    #[phenix(host)]
    clock: Host<Clock>,

    #[phenix(event("phenix.planning.completed"))]
    completed: Emit<PlanningCompleted>,

    state: ApiState,
}
```

Normal unannotated fields remain private component state.

Component ownership derives from the enclosing plugin field. Authors must not repeat the owner manually.

## 31. Component behavior

Runtime-facing methods remain ordinary Rust methods:

```rust
#[phenix_sdk::component]
impl Api {
    #[phenix(export(Planning), public)]
    async fn plan(
        &mut self,
        ctx: &CallContext,
        request: PlanRequest,
    ) -> Result<PlanResponse, Error> {
        // domain logic
    }

    #[phenix(layer(ModelsInference, priority = PRIORITY))]
    async fn model_policy(
        &mut self,
        ctx: &LayerContext,
        request: ModelRequest,
    ) -> Result<LayerResult, Error> {
        // interposition
    }

    #[phenix(listen("phenix.sessions.created"))]
    async fn session_created(
        &mut self,
        event: SessionCreated,
    ) -> Result<(), Error> {
        // reaction
    }
}
```

The enclosing macro collects method annotations and generates or exposes the descriptors needed by Core.

No global linker registration is used.

## 32. Public projection syntax

Public API exposure is a modifier on an existing contribution:

```rust
#[phenix(export(BenchRun), public)]
fn run(...) -> ...;
```

and:

```rust
#[phenix(value("phenix.bench.capabilities@1"), public)]
fn capabilities(...) -> ...;
```

Do not require `sdk { ... }`, `bindings { ... }`, `api { ... }`, or similar blocks that restate the same methods.

Bindings derive from the resolved public projection.

## 33. Lifecycle authoring

Lifecycle methods attach to the plugin implementation:

```rust
#[phenix_sdk::plugin]
impl Plugin {
    #[phenix(start)]
    fn start(
        &mut self,
        ctx: &PluginContext,
    ) -> Result<(), Error> {
        Ok(())
    }

    #[phenix(stop)]
    fn stop(
        &mut self,
        ctx: &PluginContext,
    ) -> Result<(), Error> {
        Ok(())
    }
}
```

The plugin ID is not repeated.

No lifecycle impl is needed when defaults suffice.

A future `prepare` handler uses the same model when custom preparation is required.

## 34. Resource authoring

Resource declaration belongs on the field:

```rust
#[phenix(
    resource,
    schema = 3,
    features(Transactions, Migrations)
)]
plans: Durable<PlanStore>,
```

Migration functions should be associated with the resource type or resource declaration, not manually called from plugin startup.

For example:

```rust
#[phenix_sdk::resource(schema = 3)]
impl PlanStore {
    #[phenix(migrate(from = 2))]
    fn v2_to_v3(old: V2) -> Result<V3, MigrationError> {
        // ...
    }
}
```

The exact implementation syntax may vary. The invariant is that schema ownership is declared once and kernel startup performs the mechanical registration and migration work.

## 35. Configuration authoring

A plugin field may declare configuration:

```rust
#[phenix(config)]
config: Config,
```

Schema derives from the Rust type when possible.

Defaults use ordinary Rust `Default` semantics where those defaults are semantic plugin defaults.

External source parsing remains a configuration frontend concern.

## 36. Construction QoL

The author should not fill kernel-owned fields with dummy values.

The macro may generate a builder and construction adapters.

Default behavior:

- concrete dependency fields use their plugin definition/default when not customized;
- components use `Default` when available;
- kernel-managed resource/import/host handles are constructed by kernel plumbing;
- unannotated fields use `Default` when available;
- custom ordinary state may use an explicit initializer or normal author constructor.

A generated builder should expose semantic customization, for example:

```rust
Plugin::builder()
    .config(config)
    .models(custom_models)
    .cache(cache)
    .build()
```

It must not expose `owner_id(...)`, `register_component(...)`, `dependency_id(...)`, `dispatch_table(...)`, or equivalent wiring controls.

## 37. Derived metadata

Macros should derive mechanically knowable metadata.

Examples:

```text
package version
component owner
nested component ID
nested resource ID
dependency PluginId
request/response schemas
public callable schemas
listener payload schema
direct dependency list
factory availability
execution adapter
```

Semantic policy remains explicit.

Examples:

```text
root stable PluginId
authority request
public visibility
priority when ordering matters
stable cross-plugin interface identity
execution runtime when non-default
durable schema version
migration policy
```

## 38. Secure defaults

The default static plugin mode is:

```text
execution: embedded
authority: none unless requested
lifecycle: kernel default
public API: none unless marked public
concrete dependencies: none
imports: none
resources: none
controllers: none
```

Bare structural values use projected matching.

Optionality must be explicit through `Optional<T>`.

No default provider fallback exists. No ambient host authority exists.

## 39. Generated and kernel-generic plumbing

The authoring system may generate, or let the kernel derive generically:

- Core plugin descriptors;
- Core component descriptors;
- stable identifier helpers and safe derived IDs;
- direct plugin dependency descriptors;
- interface schemas;
- typed callable clients;
- event emitters and subscriptions;
- embedded factory and `PluginInstance` adaptation;
- lifecycle delegation;
- terminal, component, hook, and layer dispatch;
- request decoding and response encoding;
- durable schema registration and migration;
- public callable/value descriptors;
- direct dependency namespaces;
- builder code;
- compile-time diagnostics.

Prefer kernel-generic behavior when generation would only repeat the same implementation for every plugin.

Use proc macros where compile-time inspection or item rewriting materially removes author boilerplate.

Generated data uses Core types directly. There is no macro-owned duplicate manifest model.

## 40. Static discovery

Static plugin composition stays visible to Rust and rust-analyzer.

Do not use:

```text
inventory
linkme
linker sections
crate scanning
hidden global registries
constructor side effects
```

for normal static plugin discovery.

A root plugin, preset, catalog, or application explicitly supplies the static composition entry point.

The kernel recursively discovers typed concrete dependencies from there.

## 41. Direct dependency access

A macro may expose direct dependencies through a generated namespaced module when it improves Rust authoring:

```rust
plugin::dependencies::sessions
plugin::dependencies::models
```

Only direct dependencies are exposed.

Do not flatten transitive dependencies into one namespace.

## 42. Compile-time validation

Macros should reject statically knowable invalid states.

Examples:

- `#[phenix(dep)]` on a non-plugin type;
- `#[phenix(component)]` on a non-component type;
- invalid local duplicate IDs;
- invalid attribute combinations;
- resource-only plugin with embedded handlers;
- external plugin with an embedded-factory-only declaration;
- malformed lifecycle signatures;
- malformed listener/export signatures;
- missing schema traits;
- impossible local migration declarations;
- unsupported structural wrapper combinations;
- public value with mutable behavior;
- a cross-plugin import with neither a canonical marker nor explicit ID.

Compiler diagnostics should identify the plugin item and violated rule.

## 43. Kernel-time validation

The kernel owns validation that depends on assembled runtime state.

Examples:

- duplicate `PluginId` compatibility;
- dependency cycles;
- runtime-provider cycles;
- required import resolution;
- global schema compatibility;
- provider ambiguity;
- global public-path collisions;
- authority grants;
- durable namespace conflicts;
- artifact/runtime compatibility;
- migration compatibility;
- environment bindings;
- graph-generation transition safety.

The macro must not become a second resolver.

## 44. Dynamic plugin builders

A builder remains valid when plugin shape is genuinely runtime-derived.

Examples include:

- a plugin loaded from external metadata;
- a guest plugin discovered through a runtime bridge;
- generated plugin composition;
- runtime package inspection.

Dynamic builders produce the same Core plugin/component/contribution types as static macros.

Static first-party plugins whose shape is known at compile time must not hand-write equivalent builders or manifests.

Catalogs, presets, passive SDK crates, applications, and Core fixtures do not get fake plugin declarations merely to increase adoption.

## 45. Example: benchmark plugin

A benchmark implementation can be a normal plugin:

```rust
#[phenix_sdk::plugin("phenix.bench")]
pub struct Plugin {
    #[phenix(import)]
    models: Required<Call<ModelsInference, ModelRequest, ModelResponse>>,

    #[phenix(component)]
    api: Api,

    #[phenix(resource)]
    runs: Durable<BenchRuns>,

    #[phenix(config)]
    config: BenchConfig,
}
```

Its application-facing API is ordinary public contributions:

```rust
#[phenix_sdk::component]
impl Api {
    #[phenix(export("phenix.bench.run@1"), public)]
    async fn run(
        &mut self,
        ctx: &CallContext,
        request: BenchRequest,
    ) -> Result<BenchResult, Error> {
        // ...
    }

    #[phenix(value("phenix.bench.capabilities@1"), public)]
    fn capabilities(
        &self,
        ctx: &ReadContext,
    ) -> BenchCapabilities {
        // ...
    }
}
```

The resolved public projection becomes conceptually:

```text
bench.run(...)
bench.capabilities
```

A Lua binding can expose:

```lua
phenix.bench.run(request)
phenix.bench.capabilities()
```

without the benchmark plugin defining Lua-specific API code.

The Lua binding converts Lua values to `PhenixValue`, emits the kernel invocation, and converts the returned value back.

The kernel still owns target resolution, validation, authority, and execution.

## 46. Example: active behavior

A plugin that reacts to session creation may declare a listener:

```rust
#[phenix_sdk::component]
impl WorkerController {
    #[phenix(listen("phenix.sessions.created"))]
    async fn on_session_created(
        &mut self,
        ctx: &EventContext,
        event: SessionCreated,
    ) -> Result<(), Error> {
        // ...
    }
}
```

A plugin that must affect the originating operation uses a layer instead:

```rust
#[phenix_sdk::component]
impl Policy {
    #[phenix(layer(SessionCreate))]
    async fn create_session(
        &mut self,
        ctx: &LayerContext,
        request: CreateSession,
    ) -> Result<LayerResult, Error> {
        // transform, deny, handle, or delegate
    }
}
```

The author chooses behavior by meaning. The kernel chooses wiring.

## 47. Ergonomics acceptance test

A representative simple plugin should contain almost exclusively domain code.

Good:

```rust
#[phenix_sdk::plugin("acme.weather")]
pub mod plugin {
    #[phenix(export("acme.weather.current@1"), public)]
    pub async fn current(
        ctx: &CallContext,
        request: WeatherRequest,
    ) -> Result<Weather, Error> {
        // actual implementation
    }
}
```

A representative stateful plugin should add only the state and relationships it actually needs.

Migration is incomplete if authors still maintain any parallel:

```text
manifest
owner table
dependency ID table
factory table
registry
dispatch ladder
resource startup list
SDK declaration tree
```

## 48. Migration gate before adoption

Broad first-party adoption is forbidden until tests prove the complete goal-level authoring model.

Before #462 may migrate first-party runtime plugins, the implementation must prove all of the following through unit, compile-fail, graph, lifecycle, runtime, and repository-enforcement tests:

- stateful plugins are ordinary Rust structs;
- stateless plugins do not require user-written empty structs;
- the plugin declaration is the only static composition declaration;
- typed concrete dependency fields require no repeated IDs;
- recursive transitive collection works;
- diamond dependencies deduplicate by `PluginId`;
- incompatible duplicate IDs fail;
- dependency cycles fail with the concrete path;
- components and resources support derived and explicit stable IDs;
- required and optional typed interface imports work without implementation coupling;
- provider and consumer local Rust types may differ structurally;
- `T`, `Project<T>`, and `Exact<T>` behave consistently across typed boundaries;
- exports, services, terminal selection, layers, hooks, events, and listeners all use the canonical kernel mechanisms;
- public callables and read-only values project without a parallel SDK tree;
- inbound foreign values normalize through `PhenixValue` and kernel invocation dispatch;
- lifecycle defaults and explicit start/stop behavior work;
- controller scheduling remains kernel-owned;
- embedded execution works through generated or generic `PluginInstance` plumbing;
- external and resource-only execution modes are representable without fake embedded factories;
- arbitrary runtime IDs are representable through the runtime-provider boundary;
- runtime bootstrap cycles are rejected;
- resources declare durable schema, backend features, and migrations once;
- kernel startup performs mechanical resource registration/migration;
- requested versus granted authority remains distinct and attenuating;
- host capabilities are injected rather than ambient;
- graph generations pin plugin artifact/runtime/provider revisions;
- invalid candidate replacement leaves the prior generation active;
- static discovery uses no `inventory`, `linkme`, linker sections, scanning, or hidden global registration;
- dynamic builders produce the same Core model and remain limited to genuinely runtime-derived shapes;
- representative simple plugins contain mostly domain behavior rather than Phenix plumbing.

A partial macro that still requires hand-written manifests, dependency IDs, owner IDs, factories, dispatch ladders, resource registration, lifecycle adapters, transitive collection, registry/re-export plumbing, or an SDK declaration tree does not satisfy this gate.

Do not use a partial authoring implementation as a temporary migration format.

## 49. Repository enforcement after adoption

Once first-party migration begins, repository checks must reject:

- hand-written static first-party plugin manifests;
- static plugin registration through hidden global discovery;
- passive packages in the runtime plugin catalog;
- contract-only runtime-plugin dependency edges;
- parallel old and new authoring systems;
- reintroduction of a monolithic `phenix_plugin! { ... }` DSL for static first-party plugins;
- duplicated plugin/component/resource identity declarations where the authoring model derives them;
- runtime-specific plugin contract models;
- public `Any` at Phenix boundaries;
- JSON values outside explicit JSON boundaries;
- ambient host capability access where injected host handles are required.

Behavioral tests should cover semantics. Source checks should cover architecture rules that are mechanically knowable.

## 50. Adoption scope

After the gate passes, migrate every static first-party runtime plugin directly to the final model and delete superseded wiring rather than preserving an intermediate style.

Migration includes API, artifacts, basic context/model/skills/tools, command toolbelt, context, debug, execution, frontend-facing runtime contributions, hooks, jobs, language-host plugins, models, options, planning, repository workers, session tree, sessions, workspace, memory, and any other package still classified `runtime-plugin` at migration time.

Do not create fake plugin declarations for catalogs, presets, passive SDK crates, applications, adapters, clients, bindings, transports, Core fixtures, or ordinary libraries.

Truly runtime-derived plugin shapes may keep builders.

Migration must preserve exact existing stable IDs, versions, execution modes, concrete dependencies, interface imports, service roles/priorities, authority, optionality, resources, lifecycle, structural matching, durable ownership, public paths, and layer behavior unless a separate canonical spec intentionally changes them.

Delete manual manifests, factories, identifier parsers, dependency-ID registries, service dispatch ladders, decode/encode plumbing, namespace/schema startup, and equivalent static wiring when replaced.

Do not retain transitional aliases.

## 51. Completion criteria

The plugin system is complete when all of these are true:

- a plugin provides capabilities and never wires itself into a live runtime;
- the kernel is the only authority for validation, resolution, composition, activation, dispatch, and reconciliation;
- plugin, component, interface, resource, application, adapter, binding, transport, and runtime-provider roles stay distinct;
- static stateful Rust plugins use ordinary structs;
- stateless Rust plugins do not require user-written empty structs;
- concrete plugin dependencies are typed and deduplicated by `PluginId`;
- interface imports remain implementation-independent;
- all static Core plugin features have Rust-native authoring support;
- macros remove wiring rather than introducing another configuration language;
- public SDKs and bindings derive from categorized public callables and read-only values;
- dynamic values use `PhenixValue` and become typed immediately after target resolution;
- embedded is the only Core runtime implementation;
- additional execution runtimes are ordinary runtime-provider plugins;
- load, unload, replacement, and builds are kernel-managed transactions;
- invalid candidate generations never partially mutate the active runtime;
- runtime work pins exact graph and artifact generations;
- authority remains explicit and attenuating;
- durable ownership and migrations are declared once;
- hooks use ordinary interposition or events;
- controllers are kernel-scheduled;
- no hidden global static registration exists;
- passive SDKs and helpers activate nothing;
- first-party plugins contain domain behavior instead of Phenix plumbing.

## Architectural summary

The final mental model is:

```text
Plugin author writes
  data
  state
  typed relationships
  functions
  semantic annotations

Macros derive
  static descriptions
  schemas
  IDs where safe
  adapters
  boring Rust plumbing

Kernel performs
  loading
  validation
  dependency resolution
  provider selection
  authority resolution
  composition
  lifecycle
  dispatch
  persistence coordination
  graph reconciliation

Bindings project
  public callables
  public read-only values

Runtime is
  one coherent
  inspectable
  generation-pinned
  kernel-composed graph
```

The short rule remains:

> Plugins provide capabilities. The kernel composes capabilities into the runtime.
