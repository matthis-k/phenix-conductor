# Client-provided tools implementation handoff

## Status

status: partial

## Purpose

Mechanical implementation plan for PR #505.

Read, in order:

1. `spec/value-capability-sdk.md` — authoritative cross-language ABI.
2. `spec/client-provided-tools.md` — #505 semantics.
3. this file — implementation sequencing and completion gate.

Do not implement the superseded dedicated client-callback architecture from older revisions of this PR.

## Dependency gate

#503 must already provide:

- generic capability owner/generation identities including client ownership;
- `SdkValue { schema, value }` retrieval;
- generic `CallableRef` invocation in both transport directions;
- paired-schema Lua conversion;
- Lua function lifting into client-owned refs;
- bounded host-thread dispatch and structural stale/disconnect/cancel errors.

If those are missing, fix #503 rather than recreating them in #505.

## Fixed decisions

1. Tool behavior is a `PhenixValue::Callable`, not a callback id.
2. Tool admission is session-scoped and ephemeral.
3. Tool admission and callable invocation are separate concerns.
4. Client tools use ordinary `CallableDescriptor(kind = Tool)` for admission metadata.
5. The active model path projects runtime/plugin and admitted-client tools into ordinary `ModelToolDescriptor` values before model routing.
6. Providers and backends do not learn a client-tool variant or origin flag.
7. Client tool behavior invokes through #503 generic capability invocation.
8. Existing callable permission/capability policy applies.
9. Client disconnect removes admissions but callable generation invalidation is owned by #503.
10. `phenix.tools.register` is authoring sugar over function lifting + tool-definition construction + admission.
11. Do not route client tools through runtime plugin SDK registration.
12. Do not expose `PluginHost`/component APIs through client bindings.
13. Do not retain the old dedicated `ClientCallableRequest` model if generic invocation can represent the same semantics.

## Current code to inspect

Before editing, locate the current exact-head implementations of:

```text
phenix-domain
  CallableDescriptor
  CallablePolicy
  CallableKind

phenix-core / phenix-plugin-execution
  ModelToolDescriptor
  ModelToolCall
  AgentLoopCommand

phenix-application-interface
  ListCallables / InvokeCallable / session operations

phenix-acp-stdio
  SdkApplicationService
  model_tool_surface
  execute_admitted_client_tool_call

phenix-binding-lua
  generic callable lifting/invocation from #503
```

Do not assume old backend `ToolProvision` paths are the active model-routing path. Preserve ownership boundaries, not stale file names.

## Phase 1: define the value-level tool contract

Add an application-owned typed record equivalent to:

```text
ClientToolDefinition {
  id: CallableId
  description: String
  input: PhenixSchema
  output: PhenixSchema
  capabilities: Vec<String> or existing application policy projection
  requires_permission: bool
  invoke: CallableRef
}
```

The generated `PhenixSchema` for `invoke` must be a callable schema compatible with `input -> output`.

If the macro/type system cannot express a field whose callable input/output depends on sibling fields, use a generic callable contract plus explicit admission-time compatibility validation. Do not fall back to an untyped callback id.

Required validation rejects invalid ids or schemas, stale/unavailable callable refs, input/output mismatch, and unsupported policy metadata.

## Phase 2: add session admission operations

Expose descriptor-driven application operations or SDK callables with semantics equivalent to:

```text
ClientToolAddInput {
  session_id
  tool: ClientToolDefinition
}

ClientToolAdmission {
  admission_id
  callable_id
}

ClientToolRemoveInput {
  session_id
  admission_id
}
```

The transport/runtime injects the current client owner generation. Public input must not accept a caller-selected connection owner.

### Add algorithm

1. resolve current connection owner generation;
2. resolve the target session scope;
3. validate tool definition and callable schema;
4. reject duplicate effective `(session_id, callable_id)`;
5. allocate admission identity;
6. store ephemeral admission indexed by session + callable id + owner generation;
7. make the descriptor visible to future model-tool projection;
8. return success only after step 7.

### Remove algorithm

1. resolve admission under session/current owner generation;
2. remove it from future discovery/invocation admission;
3. unregister the callable when no admission still references it;
4. return stable stale/not-found error for repeated removal;
5. never target a later admission using a stale handle.

## Phase 3: ephemeral admission registry

Use session, callable, owner, and generation indexes sufficient for deterministic conflict handling, cleanup, and invocation lookup.

Registry requirements:

- not durable;
- no serialization into session persistence;
- deterministic duplicate handling;
- efficient cleanup by owner generation;
- efficient lookup by `(session, callable_id)` during invocation.

## Phase 4: merge into the ordinary model surface

The current model-routing ABI consumes `ModelToolDescriptor`, not the older backend `ToolProvision` seam.

The application host merges:

```text
runtime/plugin ModelToolDescriptor values
+ client admission descriptors for current session
```

before model routing.

Rules:

- no new provider or backend API;
- no client-tool-specific descriptor type;
- no service-id magic;
- duplicate effective id is a conflict before provider observation;
- ordering is deterministic;
- provider encoding remains unchanged and origin-neutral.

Do not keep an unused backend-specific merge helper merely to match an older version of this handoff.

## Phase 5: route invocation through generic `CallableRef`

For a `ModelToolCall` that resolves to a client admission:

1. resolve the admitted descriptor under the session;
2. run existing permission policy first;
3. validate model input against the tool input schema;
4. invoke the stored `CallableRef` through #503 generic capability dispatch;
5. validate output against the tool output schema;
6. emit ordinary typed `ExecutionChange::ToolResult` or `ToolFailed`.

`ToolFailed` preserves structural failure when it carries typed `ApplicationError`; do not flatten cancellation/disconnect/provider failures into display strings.

## Phase 6: disconnect cleanup

On client generation death:

1. remove every admission indexed by owner generation;
2. make them absent from new model turns immediately;
3. prevent new invocation admission;
4. allow already-running generic capability calls to resolve through #503 cancellation/disconnect semantics;
5. do not touch admissions owned by another client generation.

Reconnect does not restore anything automatically at runtime level.

## Phase 7: Lua `tools.register`

Build on the generic #503 Lua API.

Target semantics:

```lua
local stop = phenix.tools.register({
  session_id = session_id,
  id = "fixture.client.echo",
  description = "Echo input",
  input = input_schema,
  output = output_schema,
  requires_permission = false,
}, function(input)
  return input
end)
```

Implementation:

1. obtain expected callable schema from tool input/output;
2. lift the Lua function through the generic callable machinery;
3. construct `ClientToolDefinition` containing the resulting `CallableRef`;
4. invoke tool-add admission API;
5. return a stop handle that removes the admission;
6. stop is idempotent locally while remote stale generation/removal remains structural.

The helper must not maintain a second Lua callback registry.

Frontend-local template storage remains a frontend concern. Reconnect creates new refs and admissions.

## Phase 8: permission parity

Prove both:

```text
model tool call -> permission allow -> generic capability invocation
model tool call -> permission deny -> no client handler execution
```

Client-local implementation does not bypass model execution policy.

## Phase 9: tests

### Rust/domain

- tool definition schema/contract validation;
- admission duplicate conflict;
- unrelated sessions reuse ids;
- admission removal;
- owner-generation cleanup;
- model-facing descriptor parity and deterministic merge.

### Execution

- client capability route passes ordinary permission path;
- generic capability success becomes ordinary tool result;
- generic capability error becomes ordinary typed tool failure;
- cancellation/disconnect remain typed;
- no provider/backend special case.

### Application/ACP

- add/remove operations descriptor-driven;
- owner identity injected internally;
- no public forgeable connection id;
- nested client callables in capability inputs are admitted against the capability input schema;
- typed errors survive ACP.

### Lua

- ordinary Lua function lifted through #503;
- `tools.register` admits tool;
- host model path invokes the exact Lua handler on the host thread;
- output validation;
- stop removes tool;
- reconnect requires new ref/admission;
- stale old ref cannot execute.

## Phase 10: packaged e2e

Use deterministic fixture tool `fixture.client.echo`.

Required sequence:

```text
connect Lua client
register fixture tool with Lua function
verify admission is in the effective model tool surface
construct/receive typed ModelToolCall through the host model boundary
execute ordinary permission/tool lifecycle
verify Lua host-thread handler receives typed input
return typed output
verify ordinary tool result
remove tool
verify no future discovery/invocation
```

Reconnect generation semantics may remain in a focused Rust integration regression when the packaged fixture does not restart the process.

## Source checks

Add deterministic checks where practical:

- no `client_tool` backend/domain kind;
- no magic service-id prefix for client calls;
- no production Lua `ClientCallableRequest`/raw `_phenix/...` framing;
- no `mlua::Function` stored in runtime/server registries;
- client tool definition behavior field is a callable capability/ref;
- no durable persistence of client admissions.

## Completion gate

#505 is complete only when:

- all phases above are implemented;
- `spec/value-capability-sdk.md` #505 ownership requirements are satisfied;
- old callback-specific semantics have been removed or reduced to transport-only envelopes;
- exact-head Source, Rust, Product, Docs, and Maintenance checks are green, or the only remaining blocker is manual CI approval.
