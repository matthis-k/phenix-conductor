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
4. Client tools use ordinary `CallableDescriptor(kind = Tool)` model-facing shape.
5. Client tools use ordinary `ToolProvision`; backends do not learn a client-tool variant.
6. Client tool behavior invokes through #503 generic capability invocation.
7. Existing callable permission/capability policy applies.
8. Client disconnect removes admissions but callable generation invalidation is owned by #503.
9. `phenix.tools.register` is authoring sugar over function lifting + tool-definition construction + admission.
10. Do not route client tools through runtime plugin SDK registration.
11. Do not expose `PluginHost`/component APIs through client bindings.
12. Do not retain the old dedicated `ClientCallableRequest` model if generic invocation can represent the same semantics.

## Current code to inspect

Before editing, locate the current exact-head implementations of:

```text
phenix-domain
  CallableDescriptor
  CallablePolicy
  CallableKind

phenix-backend
  ToolProvision
  PreparedToolSurface

phenix-plugin-execution
  session/model tool catalog assembly
  permission path
  callable/tool result lifecycle

phenix-application-interface
  ListCallables / InvokeCallable / session operations

phenix-binding-lua
  generic callable lifting/invocation from #503
```

Do not assume file names from this handoff if #503 refactored them. Preserve the ownership boundaries, not stale paths.

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

Required validation function:

```text
validate_client_tool_definition(definition, callable_schema_registry/context)
```

It must reject:

- invalid id;
- invalid schemas;
- callable ref whose contract is unavailable/stale;
- callable input incompatible with declared input;
- callable output incompatible with declared output;
- unsupported policy metadata.

## Phase 2: add session admission operations

Expose descriptor-driven application operations or SDK callables with semantics equivalent to:

```text
ClientToolAddInput {
  session_id
  tool: ClientToolDefinition
}

ClientToolAdmission {
  admission_id
  generation or stable admission identity
  callable_id
}

ClientToolRemoveInput {
  session_id
  admission_id
  generation if needed
}
```

Names are flexible. Semantics are not.

The transport/runtime must inject/know the current client owner generation. Do not accept a caller-selected connection owner in public input.

### Add algorithm

1. resolve current connection owner generation;
2. resolve session and reject closed/missing session;
3. validate tool definition;
4. verify the `invoke` ref belongs to a live callable capability;
5. verify callable schema matches declared tool schema;
6. check duplicate effective `(session_id, callable_id)`;
7. allocate admission identity;
8. store ephemeral admission indexed by session + callable id + owner generation;
9. make descriptor visible to future tool provisioning;
10. return success only after step 9.

### Remove algorithm

1. resolve admission under session/current owner generation;
2. remove it from future discovery/invocation admission;
3. clear indexes;
4. return stable stale/not-found error for repeated removal;
5. never target a later admission using a stale handle.

## Phase 3: ephemeral admission registry

Suggested shape:

```rust
struct ClientToolAdmissions {
    by_key: BTreeMap<AdmissionKey, Admission>,
    by_session_callable: BTreeMap<(SessionId, CallableId), AdmissionKey>,
    by_owner_generation: BTreeMap<CapabilityOwnerGeneration, BTreeSet<AdmissionKey>>,
}

struct Admission {
    id: AdmissionId,
    session_id: SessionId,
    owner_generation: CapabilityOwnerGeneration,
    descriptor: CallableDescriptor,
    invoke: CallableRef,
}
```

The exact owner-generation type should reuse #503.

Registry requirements:

- not durable;
- no serialization into session persistence;
- deterministic duplicate handling;
- efficient cleanup by owner generation;
- efficient lookup by `(session, callable_id)` during invocation.

## Phase 4: merge into ordinary tool provisioning

Find the exact path that currently constructs the session/model `ToolProvision`.

Modify it to merge:

```text
runtime/plugin tool descriptors
+ client admission descriptors for current session
```

Then feed that combined list through the existing backend capability preparation.

Rules:

- no new backend API;
- no `ClientToolProvision`;
- no service-id magic;
- duplicate id is an invariant error/conflict;
- ordering is deterministic.

Add a focused test proving runtime and client tools are indistinguishable at the backend surface except identity/content.

## Phase 5: route invocation through generic `CallableRef`

At ordinary tool execution time, the selected tool needs behavior ownership.

If current runtime tool registry stores only descriptors + service ids, generalize internal route metadata to distinguish implementation location without leaking it to backends, conceptually:

```text
ToolRoute::Runtime(existing route)
ToolRoute::Capability(CallableRef)
```

This is internal execution metadata, not a new tool kind.

For `ToolRoute::Capability`:

1. run the existing permission policy first;
2. validate model input against tool input schema;
3. invoke `CallableRef` using #503 generic capability dispatcher;
4. validate output against tool output schema;
5. emit ordinary tool result/failure events.

Do not create a bespoke client-callback request.

## Phase 6: disconnect cleanup

Hook the application/transport connection lifecycle into the admission registry.

On client generation death:

1. remove every admission indexed by owner generation;
2. make them absent from new model turns/tool provision immediately;
3. prevent new invocation admission;
4. allow already-running generic capability calls to resolve through #503 cancellation/disconnect semantics;
5. do not touch admissions owned by another client generation.

Reconnect does not restore anything automatically at runtime level.

## Phase 7: Lua `tools.register`

Build on the generic #503 Lua API.

Target semantics:

```lua
local stop = phenix.tools.register({
  session_id = session_id, -- may be implicit frontend current session
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
2. call generic `from_lua(Type::Callable, handler)` so the handler is lifted;
3. construct `ClientToolDefinition` containing resulting `CallableRef`;
4. invoke tool-add admission API;
5. return native/Lua stop handle that removes the admission;
6. stop is idempotent locally but must not hide a stale remote generation as success when explicit remote removal is attempted twice.

The helper must not maintain a second Lua callback registry. Use #503's registry.

### Templates before session

If `phenix_nvim` wants tools declared before a session exists, implement this in frontend Lua as a template registry.

At session activation:

```text
template -> new lifted Lua callable ref -> session admission
```

On disconnect/session switch, remove/drop active admissions. On reconnect, create new refs/admissions.

Do not make generic native tool admission global just to support this ergonomic feature.

## Phase 8: permission parity

Add a permission-gated fixture tool.

Prove:

```text
model tool call
-> existing permission request
-> allowed
-> generic capability invocation
```

and denial prevents Lua handler execution.

Client local tool policy must not bypass execution authority merely because the callable is outside the runtime process.

## Phase 9: tests

### Rust/domain

- tool definition schema/contract validation;
- admission duplicate conflict;
- unrelated sessions reuse ids;
- admission removal;
- owner-generation cleanup;
- model-facing descriptor parity.

### Execution

- client capability route passes ordinary permission path;
- generic capability success becomes ordinary tool result;
- generic capability error becomes ordinary structural tool failure;
- cancellation/disconnect behavior;
- no backend special case.

### Application/ACP

- add/remove operations descriptor-driven;
- owner identity injected internally;
- no public forgeable connection id;
- typed errors survive ACP.

### Lua

- ordinary Lua function lifted through #503;
- `tools.register` admits tool;
- model invocation executes exact Lua handler on host thread;
- output validation;
- stop removes tool;
- reconnect requires new ref/admission;
- stale old ref cannot execute.

## Phase 10: packaged e2e

Use a deterministic fixture tool `fixture.client.echo`.

Required sequence:

```text
spawn/connect Lua client
create/resume session
register fixture tool with Lua function
verify tool is in effective model tool surface
invoke through execution path
verify Lua host-thread handler receives typed input
return typed output
verify ordinary tool-result event/output
remove tool
verify no future discovery/invocation
reconnect client
verify old admission/ref stale
register new generation and verify it works
```

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
- old callback-specific semantics have been removed or clearly reduced to transport-only envelopes;
- exact-head Source, Rust, Product, Docs, and Maintenance checks are green, or the only remaining blocker is manual CI approval.
