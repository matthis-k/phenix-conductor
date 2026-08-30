# Plugin authoring macro

`phenix_plugin!` removes plugin declaration and ABI wiring boilerplate while keeping provider and consumer Rust types independent.

The implementation lives in `phenix-plugin-sdk`. `phenix-sdk` re-exports it as a convenience. Plugin ABI values remain stable identifiers plus `PhenixValue`.

## Declaration

A plugin declares only the sections it uses:

```rust
phenix_plugin! {
    "planner";

    uses {
        models: "phenix.models@1" => ModelRequest => ModelNeeds,
    }

    provides {
        planning: "phenix.planning@1" => PlanningRequest => PlanningResponse,
    }

    emits {
        completed: "phenix.planning.completed",
    }

    listens {
        session_created: "phenix.sessions.created" => SessionCreatedNeeds => on_session_created,
    }

    hooks {
        provides {
            before_plan: "phenix.planning.before@1" => BeforePlan => AfterPlan,
        }
        uses {
            model_request: "phenix.model.request@1" => ModelRequest => ModelResponse,
        }
    }
}
```

A plugin with no declarations is valid:

```rust
phenix_plugin! { "minimal"; }
```

Local names such as `models` and `completed` exist only in Rust. Quoted identifiers are the runtime ABI.

Request and response types belong to the plugin that declares them. The provider and consumer may use different Rust types for the same interface.

## Components

Each `uses` or `provides` declaration generates a local `ComponentInterface` marker. Its schema is derived from the local request and response types:

```text
InterfaceSchema::of::<Request, Response>()
```

The runtime can therefore check provider and consumer schema compatibility when it builds the component graph.

A generated dependency client is available directly on `ctx.sdk`:

```rust
let response = ctx.sdk.models.invoke(&request)?;
```

`invoke` projects the provider response into the consumer's local response type. Provider-only fields are accepted when the consumer does not need them.

`invoke_exact` requires an exact response shape:

```rust
let response = ctx.sdk.models.invoke_exact(&request)?;
```

`invoke_value` remains available when code intentionally wants the raw `PhenixValue` boundary.

A generated provider module has typed dispatch helpers:

```rust
phenix_plugin::provides::planning::dispatch(host, input, handle)
```

`dispatch` projects the incoming `PhenixValue` into the provider's local request type, calls the handler, then converts the local response back into `PhenixValue`.

`dispatch_exact` is the strict request variant.

## Events

`emits` creates typed emitter handles under `ctx.sdk.events`:

```rust
ctx.sdk.events.completed.emit(&PlanningCompleted { plan_id })?;
```

The emitter converts the local payload into `PhenixValue` and dispatches it through the existing kernel `EventBus`.

A `listens` entry owns its local required payload shape. The generated listener projects the event value before calling the handler.

```rust
fn on_session_created(event: SessionCreatedNeeds) -> Result<(), Error> {
    // business logic
}
```

Use `exact_listens` when extra producer fields must be rejected:

```rust
exact_listens {
    snapshot: "phenix.sessions.snapshot" => SessionSnapshot => on_snapshot,
}
```

The macro generates `event_subscriptions(...)` for runtime composition with the existing `EventBus`. Listener subscriptions use `EventFailurePolicy::Warn`.

This gives listener isolation. A failed listener records a warning and does not stop unrelated listeners. A structural payload mismatch also dispatches `kernel.structural_value_mismatch` with the source event, local listener name, and conversion error.

Listeners are passive. They observe an event after it exists. They do not transform or reject the operation that emitted it.

## Hooks

Hooks use the same structural component boundary as other plugin contracts.

A hook consumer declared under `hooks uses` gets a typed client under `ctx.sdk.hooks`:

```rust
let value = ctx.sdk.hooks.model_request.invoke(&request)?;
```

A hook provider declared under `hooks provides` gets a typed dispatch adapter:

```rust
phenix_plugin::hook_providers::before_plan::dispatch(host, input, handle)
```

The handler may return a transformed value or an error. The transformed value crosses the ABI as `PhenixValue`. An error propagates through the originating component or service invocation.

The macro does not define a second hook-chain runtime. Ordering and composition across multiple hook providers use the existing component and service routing mechanisms.

## Generated context

The macro generates a plugin-specific SDK and context alias. Generated dependencies are direct SDK fields. Event emitters and hook clients stay grouped by semantics:

```text
ctx.kernel
ctx.sdk.models
ctx.sdk.events.completed
ctx.sdk.hooks.model_request
ctx.plugin
ctx.call
```

The generated context still borrows `PluginHost`. Calls remain kernel-mediated and keep the existing authority attenuation, provider binding, provenance, and cycle checks.

## Type independence

Provider and consumer crates do not depend on each other or share payload definitions. The authoring machinery and ABI derives are exported by `phenix-plugin-sdk`:

```text
consumer -> phenix-plugin-sdk
provider -> phenix-plugin-sdk
```

The current `PhenixValue` derive expands through `::phenix_core`, so crates deriving local ABI types also keep `phenix-core` as a direct dependency. That derive-path constraint is separate from plugin-to-plugin type independence.

This is not required:

```text
consumer -> provider
```

For example, a provider may return:

```rust
struct ProviderResponse {
    value: String,
    tokens: u64,
}
```

The consumer may require only:

```rust
struct ConsumerNeeds {
    value: String,
}
```

Both derive their own `PhenixValue` schema. Startup compatibility checks compare those schemas. Runtime projected conversion then enforces the consumer's local view.

## Failure model

Structural mismatches are recoverable errors.

- Provider request mismatch returns a provider-side invocation error and reports `kernel.structural_value_mismatch`.
- Consumer response mismatch returns a consumer-side invocation error and reports `kernel.structural_value_mismatch`.
- Listener payload mismatch records a listener warning, reports `kernel.structural_value_mismatch`, and continues delivery to other listeners.
- Hook request or response mismatch follows the normal component invocation error path.
- A hook handler may reject an operation by returning an error.

These mismatch paths return errors instead of panicking.

## Generated code

`phenix_plugin!` generates mechanical wiring:

- plugin and component identifiers
- local import and export interface markers
- local interface schemas
- typed dependency clients
- typed hook clients and provider adapters
- plugin and component manifests
- plugin-specific SDK and context construction
- event emitter handles
- event subscription adapters
- provider request decoding and response encoding

Business logic and native payload types stay in normal Rust code.

## Validation

The authoring tests cover:

- independently defined provider and consumer request and response types
- startup component-graph compatibility for those independent types
- a live generated consumer client calling a generated provider dispatch adapter
- projected responses with provider-only fields
- exact response rejection
- generated manifests and interface schemas
- omitted declaration sections
- typed event emission through a live `Kernel`
- projected listener delivery
- exact listener mismatch isolation
- structural mismatch diagnostic emission
- typed hook transformation across independent local views
- hook rejection propagation

No shared payload crate or IDL is required. The cross-plugin ABI remains identifier plus `PhenixValue`.
