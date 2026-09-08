# Value-capability SDK and cross-language ABI

## Status

This is the authoritative architecture contract for the value/capability SDK work spanning #503, #505, #504, and #506.

It supersedes earlier operation-specific or callback-specific implementation guidance where the two conflict. Existing product semantics remain valid unless this document explicitly changes the underlying ABI.

The intended implementation stack is:

```text
#503  value/capability SDK ABI + Lua projection + observables
  -> #505  client-provided tool admission using callable values
  -> #504  canonical Neovim client over the value SDK
  -> #506  integrations and nvim.* tools through lifted Lua callables
```

A weaker implementation model should be able to implement each PR by following this document plus that PR's product-specific spec without inventing another transport or callback abstraction.

## Goal

Make `PhenixValue` the semantic cross-language ABI, including behavior.

The SDK visible to a client is a typed value graph:

```text
SdkRoot {
  schema: PhenixSchema,
  value: PhenixValue,
}
```

Structural data is represented by normal `PhenixValue` variants. Behavior is represented by first-class callable capability references:

```text
Type::Callable {
  contract,
  input,
  output,
}

PhenixValue::Callable(CallableRef)
```

Opaque stateful capabilities may use:

```text
Type::Object { contract }
PhenixValue::Object(ObjectRef)
```

but this stack must not depend on an undefined object-method protocol. For SDK resources that need methods now, prefer a structural table containing callable leaves. `ObjectRef` support should be preserved/projection-capable for future use.

The same callable mechanism must work in both directions:

```text
plugin/runtime callable -> CallableRef -> Lua callable proxy
Lua function -> client-owned CallableRef -> plugin/runtime invocation
```

This one mechanism should underpin observable listeners, client-provided tools, frontend handlers, and future language-hosted extensions.

## Existing foundation

The core already has the correct value/type primitives:

```text
PhenixSchema / Type
  Callable { contract, input, output }
  Object { contract }

PhenixValue
  Callable(CallableRef)
  Object(ObjectRef)
```

`Callable` compatibility is already function-like: inputs are checked contravariantly and outputs covariantly.

The existing `PhenixValue::schema()` is not sufficient to describe a callable value by itself because a raw `CallableRef` only carries its contract identity and `PhenixValue::schema()` necessarily falls back to broad input/output types. Therefore every SDK value graph crossing a language/transport boundary is projected with its authoritative `PhenixSchema`.

Do not attempt to reconstruct callable input/output schemas from `PhenixValue::Callable` alone.

## Core semantic model

### Structural values

The following remain ordinary copied/owned structural values:

```text
Unit
Bool
I64
U64
F64
String
Bytes
Option
List / Array
Map
Table
Variant
```

The ABI converts these recursively according to the authoritative schema.

### Callable capabilities

A callable is not a serialized closure. It is a typed capability reference to behavior owned by a provider.

Conceptually:

```text
CallableCapability {
  reference: CallableRef
  schema: Type::Callable {
    contract,
    input,
    output,
  }
}
```

Required invariants:

1. the reference contract must equal the callable schema contract;
2. the provider that minted the reference owns invocation dispatch for it;
3. a reference is valid only for its provider generation/lifetime;
4. invocation input is validated before provider code executes;
5. invocation output is validated before it crosses back to the caller;
6. stale/dead references fail structurally;
7. no language binding receives direct kernel/plugin host pointers.

### Capability provider identity

The current capability reference shape is plugin-specific: it stores `PluginId` and `GraphGenerationId`. That is insufficient once Lua/Neovim or another connected client may own a callable.

Generalize capability ownership to a transport-neutral identity.

The exact Rust names may differ, but the semantics must be equivalent to:

```text
CapabilityOwnerId =
  Plugin(PluginId)
  Client(ClientConnectionId)
  Runtime(RuntimeId)         # only if runtime-owned refs need it

CapabilityGenerationId(u64 or equivalent stable typed id)

CallableRef {
  contract: ContractId
  owner: CapabilityOwnerId
  generation: CapabilityGenerationId
  id: ReferenceId
}

ObjectRef {
  contract: ContractId
  owner: CapabilityOwnerId
  generation: CapabilityGenerationId
  id: ReferenceId
}
```

Migration requirements:

- plugin-created refs preserve current plugin identity semantics;
- a plugin graph generation maps deterministically into the generic capability generation;
- client refs use host-assigned connection identity/generation;
- users cannot forge another connection's owner identity through public application input;
- reconnect creates a new generation;
- stale refs never silently resolve to a new provider generation.

Do not encode client ownership as magic `PluginId` strings such as `client:...` merely to avoid this generalization.

### Object capabilities

`ObjectRef` should remain generically projectable as opaque userdata/proxy with identity, contract, generation, and stale-reference behavior.

This stack does not need to invent object method dispatch. Until an object-interface contract exists, SDK resources should use tables of callable values, for example:

```text
sessions.state = Table {
  get = Callable(...)
  listen = Callable(...)
}
```

Do not block this stack on richer `ObjectRef` semantics.

## SDK as `(schema, value)`

### Canonical resolved SDK

The resolved SDK exposed to a client is one paired schema/value tree:

```text
ResolvedSdkValue {
  schema: PhenixSchema
  value: PhenixValue
}
```

It may be represented internally as namespace entries before composition, but the language binding must see a deterministic root graph.

Example:

```text
schema = Table {
  sessions = Table {
    create = Callable<CreateSessionContract>(CreateSessionInput -> Session)
    state = Table {
      get = Callable<StateGetContract>(StateGetInput -> StateSnapshot)
      listen = Callable<StateListenContract>(ListenOptionsWithCallback -> StopCallable)
    }
  }
  models = Table {
    list = Callable<...>
    select = Callable<...>
  }
}

value = Table {
  sessions = Table {
    create = Callable(ref-create)
    state = Table {
      get = Callable(ref-get)
      listen = Callable(ref-listen)
    }
  }
  models = ...
}
```

### SDK contribution resolution

Current `SdkContribution` metadata remains useful for provider selection, namespaces, required interfaces, resources, and observables. Revise it so its final output can materialize a schema/value namespace rather than language-specific generated helpers.

The implementation may choose one of these equivalent internal shapes:

```text
SdkContribution {
  provider
  namespace
  ...existing dependency/resource metadata...
  bindings: binding declarations
}
```

that are materialized after graph resolution, or:

```text
SdkContribution {
  provider
  namespace
  schema
  value
  ...validation metadata...
}
```

if live refs are already available at contribution time.

The required external semantics are fixed:

1. each namespace resolves to exactly one selected provider, as today;
2. each namespace produces one authoritative `(schema, value)` pair;
3. `schema.parse(value)` must succeed before publication;
4. binding paths are deterministic and contain non-empty segments;
5. two contributions may not publish conflicting leaves at the same effective path;
6. table merges are structural and deterministic;
7. provider capability refs embedded in the value must belong to that provider/current generation unless explicitly client-owned through a client registration path;
8. the client does not reconstruct the SDK by scraping application operations.

### Application access

Expose one descriptor-driven application operation for retrieving the resolved SDK value, conceptually:

```text
phenix.application.sdk-get@1
  input: Unit or optional namespace filter
  output: SdkValue {
    schema: PhenixSchema
    value: PhenixValue
  }
```

A namespace-filtered form is allowed for size/performance, but clients must be able to obtain a logically equivalent deterministic root.

Keep existing fixed application operations for compatibility and transport internals where useful. The SDK value is now the primary language-binding semantic surface.

Do not require generated Lua code for every operation in order to expose a callable leaf.

## Generic callable invocation

### Canonical invocation contract

Define one generic capability invocation contract used by all callable proxies.

Conceptually:

```text
CapabilityInvokeInput {
  callable: CallableRef
  input: PhenixValue
}

CapabilityInvokeResult {
  output: PhenixValue
}
```

The invocation path also knows the authoritative callable schema. It must not trust input/output types inferred from the raw ref.

Server-side algorithm:

1. resolve `CallableRef.owner` and generation;
2. reject stale/unknown generation structurally;
3. resolve callable contract/schema registration;
4. verify reference contract matches expected contract;
5. validate input with the callable input schema;
6. invoke provider behavior without holding unrelated observable/kernel mutation locks;
7. validate successful output against callable output schema;
8. return structural domain/application failure unchanged by string matching.

### Runtime/plugin-owned callables

A runtime/plugin SDK callable proxy received by Lua follows:

```text
Lua call
  -> Lua values converted using callable input schema
  -> CapabilityInvokeInput(ref, PhenixValue)
  -> transport/runtime resolves plugin-owned ref
  -> plugin/service behavior
  -> validated PhenixValue output
  -> paired-schema Lua projection
```

Do not route this through model-facing `InvokeCallable { callable_id }`; model callable ids and capability refs are different identities.

### Client-owned callables

A Lua function passed where the expected schema is `Type::Callable` is lifted:

```text
Lua function
  -> local host callable registry entry
  -> host mints client-owned CallableRef
  -> PhenixValue::Callable(ref)
```

The local registry stores:

```text
LocalCallableEntry {
  ref identity
  authoritative Type::Callable schema
  Lua registry function handle
  owning client connection generation
  optional session/lifetime metadata
}
```

The native worker thread must never dereference or execute the Lua function.

When the runtime later invokes the client-owned ref:

```text
runtime generic capability invocation
  -> transport request to owning client connection
  -> bounded native host queue
  -> Neovim/Lua host thread dispatch
  -> validate input
  -> Lua function
  -> convert/validate output
  -> response to runtime
```

### Bidirectional transport

ACP remains transport, not semantic API.

The adapter may use two wire directions for the same semantic contract:

- client -> runtime: application extension request for generic capability invocation;
- runtime -> client: ACP extension callback/request for generic capability invocation of a client-owned ref.

Both directions must use the same `CapabilityInvokeInput/Result` schemas and error classes.

Do not define separate semantic types named `ClientCallableRequest`, `LuaCallbackRequest`, or similar unless they are thin transport envelopes around the canonical capability invocation contract.

### Queueing and host-thread execution

Client-hosted capability calls require a bounded queue.

Required behavior:

- no Lua callback runs on ACP/background threads;
- each inbound invocation has one response handle;
- queue overflow is explicit, never silent loss;
- disconnect fails pending callbacks structurally;
- stale-generation queued requests are rejected before Lua execution;
- cancellation can cancel a pending invocation;
- a Lua callback may use ordinary non-conflicting client APIs without deadlocking the transport loop;
- callbacks do not execute while the runtime holds observable store mutation locks.

## Lua ABI

### Paired-schema projection

The generic conversion API is conceptually:

```text
to_lua(schema, value)
from_lua(expected_schema, lua_value)
```

Never use `PhenixValue::schema()` as the sole schema for callable projection.

### Structural conversion

Use ordinary Lua values for primitive/small structural values where practical:

```text
Unit -> nil or explicit unit convention
Bool -> boolean
I64/U64/F64 -> number/integer respecting Lua ABI limits
String -> string
Bytes -> native bytes/userdata or documented string representation
Option -> nil/value according to existing binding convention
List/Array -> sequence table
Map/Table -> table
Variant -> generated/structural tagged representation
```

Preserve existing #491 structural conversion/error conventions unless this spec requires otherwise.

### Remote callable projection

For `(Type::Callable, PhenixValue::Callable(ref))`, return Lua callable userdata or a Lua function closure backed by native userdata.

It must retain:

```text
CallableRef
authoritative input schema
authoritative output schema
client/native invocation handle
```

Calling it performs generic capability invocation. It is not generated operation-specific native code.

Recommended Lua ergonomics:

```lua
phenix.sessions.create({ ... })
local stop = phenix.sessions.state.listen(opts, callback)
stop()
```

Whether the proxy is technically userdata with `__call` or a closure around userdata is an ABI implementation detail. Prefer the representation with deterministic lifetime/error behavior under LuaJIT/Lua 5.1.

### Local Lua function lifting

`from_lua(expected_schema, value)` handles a Lua function only when `expected_schema` is `Type::Callable`.

Algorithm:

1. allocate a local callable id;
2. retain the Lua function in the Lua registry on host-thread-owned state;
3. associate the exact callable schema;
4. mint a client-owned ref with current connection generation;
5. return `PhenixValue::Callable(ref)`.

Reject a Lua function where the expected schema is not callable.

Do not infer arbitrary function input/output types from Lua code.

### Callable values returned from runtime

A callable may return another callable.

Example:

```lua
local stop = state.listen(opts, callback)
stop()
```

The returned `CallableRef` is projected recursively using the returned output schema.

This is the preferred stop/unsubscribe shape where it fits the contract.

### Object projection

`PhenixValue::Object` projects to opaque userdata preserving reference identity and stale-reference behavior. Do not invent implicit methods unless the object's contract has a defined method projection mechanism.

## Observable integration (#503)

### Preserve the observable store

The transactional observable store, paths, exact/recursive matching, diff/full modes, snapshot policies, commit IDs, versions, coalescing, and allocation guarantees from `spec/observable-values.md` remain authoritative.

This spec changes only the SDK/foreign-language projection.

### Observable SDK value

Replace the idea that the binding generator must special-case each `SdkObservableResource` with a value-level resource surface.

A resource may materialize as:

```text
Table {
  get: Callable<GetInput -> GetResult>
  listen: Callable<ListenInput -> StopCallable>
}
```

or an equivalent table that exposes subscribe/unsubscribe separately.

Preferred high-level Lua surface:

```lua
local current = phenix.sessions.state.get()

local stop = phenix.sessions.state.listen({
  path = ...,
  scope = "recursive",
  mode = "diff",
  initial = "full",
}, function(change)
  ...
end)

stop()
```

### Listener callback schema

`listen` must accept a typed callable value for the listener.

Conceptually:

```text
ObservableListener = Callable<ObservableDelivery -> Unit>
```

The Lua function is lifted generically. The observable runtime stores/invokes the resulting `CallableRef`; it does not know Lua exists.

### Lazy native change projection

Observable deliveries can contain large full snapshots/diffs. Preserve the planned lazy native projection at the Lua boundary.

The semantic value remains a valid `PhenixValue` matching the delivery schema. The Lua binding may represent the delivery or payload with userdata that:

- exposes commit id, value id, from_version, version, address, subscription id, generation eagerly/cheaply;
- converts diff changes/full snapshot values only when Lua reads them;
- keeps underlying Rust-owned values alive safely;
- never changes delivery semantics.

This is a performance projection, not a new semantic value type.

### Unsubscribe/stale deliveries

Required behavior remains:

- stop/unsubscribe prevents future callback admission;
- already queued deliveries with a stale subscription generation do not invoke the Lua listener;
- callback invocation happens after observable store locks are released;
- version gaps remain recoverable through full get/snapshot semantics.

## Client interaction handlers

### Permission

Phenix-native client configuration should be expressible as a callable slot, conceptually:

```text
set_permission_handler: Callable<PermissionHandler -> Unit>
PermissionHandler: Callable<PermissionRequest -> PermissionResponse>
```

The Lua frontend supplies a function, which is lifted into a callable ref.

ACP standard permission requests remain supported for interoperability. The ACP adapter may translate standard permission callbacks to the same local handler slot. There must not be separate Lua-facing permission semantics depending on transport.

The handler can only return responses permitted by the authoritative permission contract. It cannot mint broader runtime authority.

### Elicitation/questionnaires

Likewise:

```text
set_elicitation_handler: Callable<ElicitationHandler -> Unit>
ElicitationHandler: Callable<ElicitationRequest -> ElicitationResponse>
```

The request contains authoritative `PhenixSchema`; the client may render a simple prompt or a rich questionnaire.

ACP standard elicitation remains an interoperability mapping to the same semantic handler.

Do not encode questionnaires as assistant prose and reparse them.

## Client-provided tools (#505)

### Tool behavior is a callable value

A client-provided tool is structured metadata plus a callable capability.

Conceptually:

```text
ClientToolDefinition {
  id: CallableId
  description: String
  input_schema: PhenixSchema
  output_schema: PhenixSchema
  capabilities: ...existing policy metadata...
  requires_permission: bool
  invoke: Callable<PhenixValue -> PhenixValue>
}
```

Prefer a statically typed contract where `invoke` has the exact declared input/output schemas rather than a permanently `Any -> Any` callable.

The application-facing encoding may be a typed record that lowers to `PhenixValue::Table`; the critical invariant is that behavior is `PhenixValue::Callable`, not a separately named callback route.

### Admission, not callback registration

The runtime still needs session-scoped admission/removal operations, conceptually:

```text
session.tools.add(tool_definition)
session.tools.remove(tool_id or admission_handle)
```

These operations manage model visibility/lifetime. They do not define how the behavior executes.

On add:

1. validate session and tool metadata;
2. validate the callable ref/schema matches declared input/output;
3. record ephemeral admission tied to client connection generation;
4. project ordinary `CallableDescriptor(kind = Tool)` into the existing session tool catalog;
5. apply existing permission/capability policy.

On model invocation:

1. use ordinary model-facing tool id/descriptor;
2. run ordinary permission checks;
3. invoke the stored `CallableRef` through generic capability invocation;
4. validate output;
5. report ordinary tool result/failure.

No backend needs a `client_tool` concept.

### Tool lifetime

Client tool admission is ephemeral:

- scoped to session + client owner generation;
- disconnect removes all admissions owned by that generation;
- reconnect does not revive them;
- frontend may re-admit configured templates after reconnect;
- same tool id may exist in unrelated sessions;
- duplicate effective tool id in one session is deterministic conflict.

### Lua authoring helper

The binding/frontend may expose:

```lua
local stop = phenix.tools.register({
  id = "nvim.context.selection",
  description = "Return the current Neovim selection",
  input = schema.unit(),
  output = selection_schema,
  requires_permission = false,
}, function(_)
  return current_selection()
end)
```

This helper must be thin sugar for:

```text
Lua function -> lifted CallableRef
+ structured tool-definition value
+ session admission
```

It is not a second callback protocol.

## Neovim client (#504)

### Namespace boundary

Keep:

```lua
local phenix = require("phenix")
local frontend = require("phenix_nvim")
```

`phenix` is the generic native value/capability SDK binding. `phenix_nvim` is UI/editor integration.

Never install `clients/nvim/lua/phenix/init.lua`; it would shadow the native module.

### Runtime adapter

`phenix_nvim.runtime` obtains the SDK root and consumes nested value tables, callable proxies, and observable resource tables.

Do not make the frontend depend on handwritten ACP methods or on generated per-operation Lua functions as its semantic foundation.

Thin ergonomic wrappers are allowed if they call the generic proxies.

### Product behavior unchanged

The following #504 product contract remains unchanged:

- separate transcript and compose buffers;
- `Reference` inserts editor context at remembered compose cursor and never sends;
- ordered multimodal compose document;
- snapshot selection references;
- image attachments and optional `vim.ui.img` preview;
- one explicit Send;
- runtime-authoritative transcript/session/edit state;
- incremental Markdown/structured transcript projection;
- runtime-backed diff review;
- durable resume;
- deterministic standalone mirror.

### Handlers

Permission and elicitation adapters should configure the generic local handler/callable mechanism. The frontend should not own transport-specific callback schemas.

## Neovim integrations (#506)

### Adapter registries are ergonomics

Keep replaceable frontend adapters for:

- reference providers and pickers;
- transcript node renderers;
- review UI;
- permission UI;
- questionnaire/form UI;
- image acquisition/rendering;
- notification/navigation/session/status surfaces.

These registries are frontend authoring/configuration surfaces, not separate runtime callback protocols.

Where behavior crosses the runtime/client boundary, lower Lua functions to first-class callable values.

### Default `nvim.*` tools

Implement default tools as ordinary client-provided tool definitions backed by lifted Lua callables:

```text
nvim.context.current_location
nvim.context.selection
nvim.context.buffer
nvim.context.buffers
nvim.context.diagnostics
nvim.context.quickfix
nvim.context.viewport

nvim.ui.show_location
nvim.ui.highlight_range
nvim.ui.focus_buffer
nvim.ui.pick_file
nvim.ui.show_diff
```

Tool implementations reuse the same normalized editor-context/navigation helpers as frontend actions. Do not duplicate selection/location logic for tools.

### Privileged Lua tools

Likewise:

```text
nvim.lua.eval
nvim.lua.exec
nvim.lua.reload_module
```

are ordinary client-provided tools backed by lifted Lua functions.

Policy:

- disabled by default or explicit opt-in;
- `eval` permission-gated when enabled unless explicitly configured otherwise;
- `exec` permission-gated;
- no sandbox claim;
- use protected Lua execution;
- non-convertible results fail structurally;
- reload defaults to `phenix_nvim.*` and restores old module value on failed reload.

This enables the agent to inspect/debug the live Neovim plugin through the same public tool mechanism.

## Error model

Capability ABI failures must remain structural.

At minimum distinguish:

```text
unknown capability/reference
stale generation
contract/schema mismatch
invalid input
invalid output
provider disconnected
provider failed
cancelled
queue full / host unavailable
permission denied
```

Map these through existing `ApplicationError`/kernel failure classes where equivalents exist. Add dedicated variants only when an existing class would lose necessary semantics.

Never classify by display-string matching.

Language bindings should expose the existing typed Lua error convention rather than throw opaque strings for capability failures.

## Cancellation and concurrency

Required invariants:

- generic capability invocation is cancellable;
- client disconnect cancels/fails outstanding client-owned capability calls;
- runtime/plugin unload makes old refs stale;
- no invocation waits for client/Lua while holding observable store locks or plugin graph mutation locks;
- local callback queue is bounded;
- callback reentrancy cannot deadlock the same worker loop;
- Neovim dispatch executes on the main thread;
- callback completion is one-shot.

## Security and authority

A capability ref conveys only the ability represented by that callable/object and its runtime policy.

Rules:

- a client cannot forge another provider owner/generation;
- passing a client-owned callable to runtime does not grant it kernel/plugin host authority;
- model-visible client tools still go through normal callable permission/capability policy;
- arbitrary Lua execution is privileged because it executes with the Neovim process's local authority;
- capability refs must not expose raw pointers, process handles, plugin host references, or unvalidated service ids to Lua.

## Transport serialization

`PhenixValue::Callable/Object` refs are serializable identities, not executable code.

Transport requirements:

- encode owner, generation, contract, id losslessly;
- reject unsupported/unknown owner kinds;
- preserve authoritative schema separately when a callable value is projected;
- no transport is allowed to silently coerce callable/object refs into strings/maps and lose capability semantics;
- protocol-level tests must round-trip refs and stale-generation errors.

## Generated bindings after this change

Generation remains useful for:

- fixed contract/type declarations;
- schema metadata;
- documentation/completion;
- optional ergonomic wrappers;
- static language types in languages that support them.

Generation is no longer responsible for implementing every operation's transport semantics.

The fundamental foreign-language runtime surface is:

```text
get SDK `(schema, value)`
project values
invoke CallableRef
lift local host function into CallableRef
preserve ObjectRef
structural errors
```

Do not delete useful generated APIs solely for purity if they are thin wrappers over this ABI.

## Implementation ownership by PR

### #503

Owns the generic ABI foundation needed immediately by Lua observables:

- [ ] generalize capability owner/generation identity so refs can be client-owned without fake plugin ids;
- [ ] define fixed `SdkValue` `(schema, value)` application type and SDK-get operation;
- [ ] materialize resolved SDK namespace/value graph from `SdkContribution` metadata;
- [ ] define generic capability invocation input/result contract;
- [ ] support runtime/plugin-owned callable invocation through application/ACP transport;
- [ ] support runtime -> client invocation of client-owned callable refs through one generic callback transport;
- [ ] implement paired-schema Lua projection;
- [ ] implement remote callable proxies;
- [ ] implement local Lua function lifting/registry;
- [ ] bounded host-thread callback dispatch, cancellation, disconnect, stale-generation handling;
- [ ] represent observable resources as value-level callable surfaces;
- [ ] implement `listen(opts, lua_function)` through lifted callable refs;
- [ ] preserve lazy native observable delivery payloads;
- [ ] finish observable semantic regressions and Rust/Lua/protocol parity;
- [ ] ensure existing generated operation API remains compatible as thin wrapper/fallback where needed.

### #505

Owns client-tool semantics on top of the ABI:

- [ ] define typed value-level client tool definition with `invoke: Callable`;
- [ ] add session-scoped admission/removal operations or SDK callables;
- [ ] validate callable contract/input/output against tool metadata;
- [ ] project admitted tools into ordinary `CallableDescriptor`/`ToolProvision`;
- [ ] use normal permission/capability policy;
- [ ] invoke tool behavior through generic `CallableRef` path;
- [ ] remove admissions on owner disconnect/generation change;
- [ ] add Lua `tools.register` authoring sugar;
- [ ] prove model -> client tool -> Lua -> tool result end to end;
- [ ] remove/supersede earlier dedicated `ClientCallableRequest`/register-callback semantics where redundant.

### #504

Owns canonical Neovim product implementation:

- [ ] consume generic SDK value graph from `require("phenix")`;
- [ ] use callable proxies/observable value surfaces rather than handwritten protocol calls;
- [ ] implement the existing sidebar/compose/reference/transcript/image/review/session product spec;
- [ ] configure permission/elicitation handlers through the generic callable host mechanism;
- [ ] package and mirror deterministically.

### #506

Owns replaceable integrations and Neovim-provided tools:

- [ ] retain one frontend adapter registry with built-in fallbacks;
- [ ] lower runtime-facing Lua handlers through callable lifting;
- [ ] implement default `nvim.context.*` and `nvim.ui.*` tools as value-level client tools;
- [ ] implement privileged `nvim.lua.*` tools as permission-gated client tools;
- [ ] preserve review/questionnaire/permission/image/navigation adapter semantics;
- [ ] prove third-party Lua tool and custom UI adapters end to end.

## Migration from current PR plans

The following earlier ideas are superseded where present:

```text
special client-callable registration as the semantic behavior model
special Lua callback protocol for client tools
observable-only handwritten listener callback machinery
SDK = generated operation method table
client tool = separate backend/service kind
magic service ids identifying client routes
```

Replace them with:

```text
SDK = (PhenixSchema, PhenixValue) graph
behavior = PhenixValue::Callable(CallableRef)
remote behavior -> host callable proxy
host function -> lifted client-owned CallableRef
tool admission = metadata/lifetime around a callable value
observable listener = callable value
```

Existing transport adapters may retain internal request structs if they are thin envelopes for the generic contracts.

## Required regression matrix

### Core/type system

- [ ] callable schema accepts matching ref only when contracts match;
- [ ] plugin/client/runtime capability owners serialize/deserialize losslessly;
- [ ] generation mismatch is stale, never resolved to current generation;
- [ ] callable input/output compatibility remains contravariant/covariant;
- [ ] SDK schema parses SDK value exactly/compatibly as intended.

### SDK resolution

- [ ] distinct plugin namespaces compose deterministically;
- [ ] duplicate namespace/path conflicts fail;
- [ ] unavailable provider/interface fails before publication;
- [ ] embedded callable refs belong to valid current providers;
- [ ] SDK-get returns deterministic schema/value root;
- [ ] no language-specific metadata is required to reconstruct behavior.

### Generic invocation

- [ ] runtime/plugin-owned callable invokes from Rust client;
- [ ] same callable invokes from Lua proxy;
- [ ] invalid input rejected before provider code;
- [ ] invalid output rejected before caller receives it;
- [ ] stale provider generation fails structurally;
- [ ] cancellation remains structural;
- [ ] provider error classes survive transport.

### Lua lifting

- [ ] Lua function converts only against callable expected schema;
- [ ] lifted ref carries client owner/current generation;
- [ ] runtime invokes lifted function on Lua host thread;
- [ ] Lua handler may return structural values and nested callables;
- [ ] invalid Lua output fails schema validation;
- [ ] Lua exception becomes structural provider failure;
- [ ] disconnect invalidates lifted refs;
- [ ] stale queued invocation does not run Lua;
- [ ] callback queue overflow is explicit.

### Observables

- [ ] SDK contains resource at declared binding path;
- [ ] get returns current value/version;
- [ ] listen accepts ordinary Lua function via generic callable lifting;
- [ ] initial full delivery works;
- [ ] exact/recursive and diff/full semantics match Rust;
- [ ] one delivery per subscription/commit after coalescing;
- [ ] stop/unsubscribe prevents future callbacks;
- [ ] stale queued subscription generation does not run callback;
- [ ] delivery payload remains native/lazy until inspected;
- [ ] version gap can recover with get/full snapshot.

### Client tools

- [ ] Lua tool definition contains lifted callable value;
- [ ] admission makes ordinary tool visible to model;
- [ ] permission-gated tool uses normal permission path;
- [ ] model invocation uses generic capability invocation;
- [ ] result is ordinary tool result;
- [ ] remove/disconnect removes tool admission;
- [ ] reconnect requires new callable refs/admission;
- [ ] unrelated sessions may reuse ids without global collision.

### ACP/protocol

- [ ] SDK value including callable refs round-trips;
- [ ] generic invoke works client -> runtime;
- [ ] generic invoke works runtime -> client;
- [ ] standard permission/elicitation map to same frontend handlers;
- [ ] stale/disconnected/cancelled errors retain class;
- [ ] no Plugin EventBus is introduced for capability invocation or observables.

### Neovim

- [ ] `require("phenix")` exposes value SDK and `require("phenix_nvim")` remains separate;
- [ ] Reference/compose/transcript flow works without handwritten ACP schemas;
- [ ] custom permission/questionnaire adapters execute through one host mechanism;
- [ ] `nvim.context.selection` is an admitted client tool backed by lifted Lua function;
- [ ] `nvim.ui.show_location` manipulates live editor through the same mechanism;
- [ ] opt-in `nvim.lua.eval` can inspect live plugin state after permission;
- [ ] disconnect removes all client tools and stale Lua refs.

## End-to-end acceptance

The stack is complete when one packaged scenario proves the entire capability loop:

```text
1. start Phenix + Lua/Neovim client
2. client retrieves SDK `(schema, value)`
3. Lua sees plugin-provided callable as normal callable proxy
4. Lua invokes it and receives typed result
5. Lua calls observable `listen` with an ordinary Lua function
6. binding lifts function into client-owned CallableRef
7. runtime observable invokes that ref on a change
8. callback executes on Lua host thread and reads lazy delivery payload
9. Lua registers `nvim.context.selection` tool
10. tool definition contains another lifted callable ref
11. model/runtime sees ordinary tool descriptor
12. model invokes it through generic capability invocation
13. Lua function reads Neovim state and returns typed output
14. ordinary tool result reaches model/transcript
15. client disconnects
16. outstanding client-owned refs/admissions become stale/removed
17. reconnect mints a new generation and old refs cannot execute
```

No step may depend on handwritten plugin-specific Lua glue, a parallel client-tool RPC model, or direct Lua execution from a background thread.

## Source/architecture checks

Add deterministic source checks where practical for these invariants:

- no production Lua contains literal `_phenix/...` protocol method ids;
- no `client_tool` backend/service kind is introduced;
- no fake client `PluginId` encoding is used for capability ownership;
- no worker/background module stores or invokes `mlua::Function` outside host-thread state;
- no observable listener has a Lua-specific runtime type;
- no client tool behavior field is a callback id/string when a `CallableRef` should be used;
- `phenix_nvim` contains no ACP framing/schema definitions.

## Completion rule

Do not mark a PR complete merely because its old checklist is satisfied. For #503/#505/#504/#506, this document is authoritative wherever the previous PR body or implementation handoff conflicts.

A worker should update the PR checklist as it implements each owned phase and should stop only at the normal repository stopping condition: exact-head required CI/maintenance is green or the only remaining blocker requires manual CI approval.
