# Plugin authoring macro

`phenix_plugin!` removes plugin registration and wiring boilerplate without creating Rust type dependencies between plugins.

The generated ABI uses stable Phenix identifiers and `PhenixValue`. Native request, response, hook, and event payload types remain local to each plugin.

## Authoring shape

The common form contains only sections the plugin uses:

```rust
phenix_plugin! {
    "planner";

    uses {
        models: "phenix.models@1",
        sessions: "phenix.sessions@1",
    }

    provides {
        planning: "phenix.planning@1",
    }

    emits {
        completed: "phenix.planning.completed",
    }

    listens {
        session_created: "phenix.sessions.created" => SessionCreatedNeeds => on_session_created,
    }

    hooks {
        uses {
            model_request: "phenix.model.request@1",
        }
    }
}
```

Empty sections are omitted. Projected listener conversion is the default. A listener that intentionally requires the complete payload goes in `exact_listens`:

```rust
exact_listens {
    session_snapshot: "phenix.sessions.snapshot" => SessionSnapshot => on_session_snapshot,
}
```

Names such as `models`, `completed`, and `session_created` are local Rust names. Quoted identifiers are the runtime ABI.

The macro generates direct dependency fields on the plugin SDK. Common calls stay shallow:

```rust
let value = ctx.sdk.models.invoke_value(request)?;
ctx.sdk.events.completed.emit(&PlanningCompleted { plan_id })?;
```

Hook clients remain grouped under `ctx.sdk.hooks` because they have different operation semantics from ordinary component dependencies.

## Components

A component dependency is an interface identifier plus a `PhenixValue -> Result<PhenixValue, _>` invocation boundary.

The macro generates local zero-sized interface markers and `SdkClient` fields. It does not reference provider request or response Rust types.

Provider code may keep strong local types:

```rust
let response = PlanningResponse { steps };
Ok(PhenixValue::from(&response))
```

Consumer code defines its own required shape:

```rust
let value = ctx.sdk.models.invoke_value(request)?;
let response = ModelNeeds::try_from(Project(&value))?;
```

`Exact` remains available when the consumer intentionally requires the complete shape.

## Events

Plugin events are distinct from kernel diagnostic events.

The kernel event ABI is:

```text
EventName + source plugin + PhenixValue
```

`EventName` is a validated string newtype. Core does not enumerate plugin event names or payload types.

Any plugin may introduce a new namespaced event without changing another plugin or `phenix-core`.

An emitted event may use a local typed payload. The generated emitter converts it into `PhenixValue`.

A listener owns its expected payload shape. A normal `listens` entry projects the producer value into the local type before calling the handler:

```rust
fn on_session_created(event: SessionCreatedNeeds) -> Result<(), Error> {
    // ...
}
```

An `exact_listens` entry uses exact conversion instead.

A conversion mismatch fails that listener invocation and emits a kernel diagnostic event. It does not panic, invalidate the original event, or prevent unrelated listeners from receiving it.

Listeners are passive. They cannot mutate or reject the operation that emitted the event.

## Hooks

Hooks cover participation in an operation. Their structural ABI is:

```text
HookName + PhenixValue -> Result<PhenixValue, HookError>
```

`HookName` is a validated string newtype.

A hook may inspect, transform, or reject the value. Hook failure propagates to the operation. Multiple hooks form the ordered hook chain.

Hook input and output types are local views converted from and into `PhenixValue`, using the same projected and exact rules as component calls and listeners.

Hooks and listeners cover operation interception and passive reactions. Add another extension mechanism only when a concrete case fits neither model.

## Generated code

The macro should generate only mechanical code:

- plugin and component manifest declarations
- dependency and export interface markers
- `SdkClient` construction
- plugin context wiring
- event emitter handles and listener registration
- hook registration and lookup
- provider dispatch from `PhenixValue`
- structural conversion error propagation
- diagnostic event reporting for payload mismatches

Business logic and native payload types stay in ordinary Rust code.

Generated code must use public SDK APIs. It must not require privileged access to kernel internals.

## Type independence

No generated dependency may require the provider plugin crate.

This must compile:

```text
consumer -> phenix-plugin-sdk
provider -> phenix-plugin-sdk
```

This must not be required:

```text
consumer -> provider
```

Providers and consumers may independently define structurally compatible Rust types with different names and different supersets of fields.

## Failure model

All structural mismatches are recoverable errors.

- component request mismatch: provider-side error plus diagnostic event
- component response mismatch: consumer-side error plus diagnostic event
- listener payload mismatch: listener failure plus diagnostic event
- hook input or output mismatch: hook failure plus diagnostic event

None of these paths may panic.

## Scope

This PR should add the authoring macro and the minimum SDK/runtime support needed for it. It should migrate representative first-party plugins to prove the API, then migrate the remaining plugin declarations once the generated shape is stable.

Tests must cover:

- a consumer and provider with no Rust dependency on each other
- projected component responses with provider-only extra fields
- exact component conversion rejection
- typed provider response to `PhenixValue`
- event emission and independently typed listener projection
- listener mismatch isolation and diagnostic event emission
- hook transformation and rejection
- macro-generated manifests matching runtime composition expectations
- omission of unused macro sections

Do not add a shared schema crate, generated shared Rust payload types, or an IDL requirement. The Phenix ABI remains identifier plus `PhenixValue`.
