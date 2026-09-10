# Client-provided tools as callable values

## Status

status: partial

Follow-up to #503.

`spec/value-capability-sdk.md` is the authoritative ABI contract. This file defines only the #505 product/domain slice. Where older callback-specific wording conflicts with the value-capability spec, the value-capability spec wins.

## Goal

Allow a connected client to expose local behavior to a session as ordinary model-visible tools without introducing a second callback semantic model.

A tool is structured metadata plus first-class callable behavior:

```text
ClientToolDefinition {
  id
  description
  input_schema
  output_schema
  capabilities / policy
  requires_permission
  invoke: Callable
}
```

The `invoke` field is semantically `PhenixValue::Callable(CallableRef)` under an authoritative `Type::Callable` schema.

For Lua:

```text
Lua function
  -> #503 generic function lifting
  -> client-owned CallableRef
  -> tool-definition value
  -> session admission
  -> ordinary model tool
```

The tool subsystem does not know or care that the callable is implemented in Lua or Neovim.

## Dependency contract from #503

Do not implement #505 until #503 provides:

- transport-neutral capability owner/generation identity;
- SDK `(PhenixSchema, PhenixValue)` projection;
- generic `CallableRef` invocation;
- runtime -> client invocation of client-owned refs;
- paired-schema Lua conversion;
- Lua function lifting into client-owned `CallableRef`;
- bounded host-thread callback dispatch;
- structural disconnect/stale/cancelled errors.

#505 consumes those primitives. It does not recreate them.

## Boundary

A client-provided tool is not a runtime plugin and does not gain plugin/kernel authority.

```text
model
  -> ordinary ToolProvision / CallableDescriptor
  -> admitted client tool definition
  -> invoke CallableRef
  -> generic capability invocation
  -> owning client
  -> host-language function
```

Runtime/plugin tools keep their existing implementations. Client tools differ only in where their `CallableRef` resolves.

Do not add a `client_tool` backend, service kind, magic service-id prefix, or callback-specific execution branch.

## Tool definition

Use one typed application/domain value carrying:

```text
id: CallableId
description: String
input_schema: PhenixSchema
output_schema: PhenixSchema
capabilities: existing capability/policy metadata
requires_permission: bool
invoke: CallableRef under Type::Callable
```

The callable schema must match the declared tool schema:

```text
invoke.input  compatible with input_schema
invoke.output compatible with output_schema
```

Prefer exact schemas for a tool handler. Do not permanently type client tool behavior as `Any -> Any` merely because transport values are dynamic.

The record may serialize as an application-owned `PhenixValue::Table`; the exact Rust DTO name is secondary to the semantic shape.

## Session admission

A client tool must be admitted into a specific session before the model can see it.

Expose thin admission/removal operations or SDK callables, conceptually:

```text
session.tools.add(tool_definition)
  -> ToolAdmission {
       id / generation or equivalent stop identity
     }

session.tools.remove(admission)
  -> Unit/Acknowledged
```

These operations manage visibility and lifetime only. They do not define callback execution.

### Admission algorithm

1. identify the current host-owned client connection generation;
2. resolve the target session and reject missing/closed session;
3. validate tool id/description/schemas/policy;
4. validate `invoke` is a current callable capability;
5. validate the callable schema against declared input/output;
6. reject duplicate effective `(session, callable_id)` deterministically;
7. store ephemeral admission tied to the client owner generation;
8. project an ordinary `CallableDescriptor { kind = Tool, ... }` into the session tool catalog;
9. acknowledge only after future tool provisioning can observe the tool.

The public input must not allow the client to forge another connection owner/generation.

### Removal algorithm

1. resolve admission identity under the current owner generation;
2. remove it from future tool discovery/admission before acknowledging;
3. prevent later invocation admission;
4. already-running generic capability invocation may finish/cancel under normal semantics;
5. repeated/stale removal fails predictably and cannot target a newer admission.

## Connection lifetime

Client tool admissions are ephemeral.

Required rules:

- disconnect removes every admission owned by that client generation;
- reconnect mints a new capability generation and does not revive old tools;
- a resumed session on another client does not inherit dead client tools;
- the frontend may re-admit configured tool templates after reconnect;
- same tool id may exist in unrelated sessions;
- same tool id may not resolve to two providers in one effective session;
- stale callable refs remain stale even if an identically named tool is re-admitted later.

Do not persist client tool admissions in durable execution/session storage.

## Model-facing projection

Reuse the existing tool/callable representation:

```text
CallableDescriptor {
  id
  kind = Tool
  description
  input_schema
  output_schema
  capabilities
  policy
}
```

Merge admitted client descriptors with runtime/plugin descriptors before ordinary backend tool provisioning.

Rules:

- no backend learns whether the tool is client-hosted;
- duplicate ids are deterministic conflicts, not last-writer wins;
- backend capability filtering/preparation stays unchanged;
- model-visible schemas are the declared exact tool schemas.

## Invocation

The tool execution path remains ordinary through permission/admission, then uses the generic capability ABI for behavior:

```text
model emits tool call
  -> ordinary execution/tool lifecycle
  -> ordinary permission policy
  -> validate tool input
  -> generic invoke(tool.invoke CallableRef, input)
  -> owning client executes function
  -> validate output
  -> ordinary tool result/failure
```

Do not create a dedicated `ClientCallableRequest` semantic type if the generic `CapabilityInvokeInput/Result` already represents the call. A transport adapter may retain an envelope only if it is a thin mapping to the generic contract.

## Permission and authority

Client-local execution occurs with the local authority of the client process, but the model's right to request it is governed by Phenix policy.

At minimum support policy distinction between:

- read/inspection actions;
- UI/navigation mutations;
- editor/file mutations;
- privileged arbitrary-code execution.

Reuse normal callable permission/capability policy. Do not create a Lua-side authority engine.

A tool definition cannot grant kernel/plugin authority simply by declaring a capability string.

## Lua authoring API

Provide thin sugar over #503 function lifting + session admission:

```lua
local stop = phenix.tools.register({
  id = "fixture.client.echo",
  description = "Echo a value from the connected client",
  input = input_schema,
  output = output_schema,
  requires_permission = false,
}, function(input)
  return input
end)

stop()
```

Equivalent semantics:

```text
1. convert handler against expected Type::Callable
2. mint client-owned CallableRef
3. build typed tool-definition value containing that ref
4. admit into active session
5. return admission stop/removal handle
```

If a frontend declares templates before a session exists, that is frontend-local configuration. Native/runtime admission remains session-scoped.

## Permission and elicitation relation

#505 must not create a separate host callback registry for permission or elicitation.

Those frontend handlers should consume the same #503 local callable host machinery, whether ACP interoperability reaches them through a standard ACP callback or Phenix-native callable slot.

Questionnaires remain typed `ElicitationRequest/Response` with authoritative `PhenixSchema`; no prose parsing.

## Errors

Preserve structural distinctions from the generic ABI and ordinary tool execution:

```text
invalid tool definition
schema mismatch
conflicting callable id
stale callable ref
owner disconnected
permission denied
cancelled
host queue unavailable/full
handler/provider failure
invalid handler output
```

Never classify by error text.

## Required regressions

### Admission

- valid tool definition containing a client-owned callable is admitted;
- callable input/output mismatch is rejected;
- malformed tool metadata is rejected;
- duplicate effective id is conflict;
- unrelated sessions can reuse same id;
- remove prevents future discovery/invocation;
- disconnect removes all admissions owned by generation;
- reconnect does not revive admissions or refs.

### Model/tool projection

- admitted client tool and runtime tool use same `CallableDescriptor` shape;
- both pass through the same backend `ToolProvision` preparation;
- permission-gated client tool asks through normal permission path;
- no backend/client-tool special case is required.

### Invocation

- model invocation reaches stored `CallableRef` through generic capability invocation;
- Lua handler executes on host thread;
- input/output schemas validated;
- ordinary tool result is emitted;
- handler failure remains structural;
- cancellation/disconnect propagate correctly.

### Lua API

- `phenix.tools.register` lifts ordinary Lua function;
- return handle removes admission deterministically;
- stale handle cannot remove newer admission;
- configured templates can be re-admitted after reconnect with new refs.

## Packaged acceptance

One regression must prove:

```text
Lua client connects
-> creates/resumes session
-> phenix.tools.register(fixture.client.echo, Lua function)
-> binding lifts handler into CallableRef
-> session admission exposes ordinary model tool
-> runtime invokes tool
-> generic capability request reaches Lua host thread
-> typed output becomes ordinary tool result
-> stop removes admission
-> reconnect changes generation
-> old callable/admission cannot execute
```

## Non-goals

- runtime plugin implementation from Lua;
- persistent client tools after the owning client disappears;
- exposing `PluginHost` or component registration to client Lua;
- Neovim-specific runtime code;
- a second RPC protocol for client callbacks.
