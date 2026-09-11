# Neovim client ABI precedence

## Purpose

This file records the ABI revision that applies to `spec/nvim-client.md` and `spec/nvim-client-implementation.md` after `spec/value-capability-sdk.md`.

Read the global value-capability spec first. The older Neovim product/interaction details remain authoritative, but any dependency on a callback-specific or generated-operation-specific semantic model is superseded.

## Fixed client boundary

```lua
local phenix = require("phenix")
local frontend = require("phenix_nvim")
```

`phenix` exposes the generic SDK `(PhenixSchema, PhenixValue)` graph. Nested structural tables form namespaces. Behavior is represented by callable capability proxies. `phenix_nvim` owns editor/UI behavior only.

Do not create `clients/nvim/lua/phenix/init.lua` or any Lua module that shadows the native binding.

## Runtime adapter revision

`phenix_nvim.runtime` should:

1. connect native client;
2. retrieve the SDK value graph;
3. retain the authoritative schema alongside projected values;
4. consume nested callable proxies/resource tables;
5. run one bounded host dispatch loop for pending native requests/capability invocations/observable work;
6. keep all Lua execution on Neovim main thread.

Thin wrappers around SDK callable proxies are allowed. Handwritten ACP framing, protocol method ids, or duplicated schemas are not.

## Observable revision

Observable state still powers transcript/session projections, but the frontend should consume the #503 value-level surface, for example:

```lua
local current = phenix.sessions.state.get()
local stop = phenix.sessions.state.listen(opts, function(change)
  ...
end)
```

The listener is an ordinary Lua function lifted by the generic callable ABI. The frontend does not manage a separate observable callback protocol.

## Permission and elicitation revision

Frontend permission/questionnaire handlers should use the generic local-callable host mechanism from #503.

Transport-specific ACP permission/elicitation mapping remains below the frontend. `phenix_nvim` sees one semantic handler API and returns only valid typed responses.

## Client-tool revision

Where #504 references future client tools, assume #505 semantics:

```text
Lua handler
-> lifted CallableRef
-> tool-definition value
-> session admission
```

Do not depend on a dedicated `ClientCallableRequest` protocol.

## Product semantics unchanged

The following parts of the existing #504 handoff remain unchanged and should be implemented exactly:

- conductor is canonical package source;
- standalone `phenix-nvim` is deterministic one-way mirror;
- separate transcript and compose buffers;
- `Reference` inserts context at remembered compose cursor and never sends;
- ordered compose segments;
- selection snapshots captured at reference time;
- explicit `@...` references may remain live references where specified;
- image attachments are first-class and preview is optional;
- one explicit Send;
- runtime is authoritative for transcript/session/model/tool/edit state;
- transcript prose is real Markdown;
- structured transcript nodes retain identity/extmarks;
- incremental streaming updates instead of full-buffer rerender per token;
- manual scrolling disables follow-tail;
- runtime-backed native diff review;
- restart/resume durable session;
- no optional third-party plugin required for MVP.

## Migration guidance

Delete/replace old standalone transport/protocol ownership as already specified.

For every migrated module:

```text
frontend-only UI/unsent compose state -> may remain local
runtime/application truth -> project from SDK/observable value surface
missing runtime field -> fix upstream contract, do not create private Lua schema
```

## Completion requirement

#504 is complete only if the original product acceptance scenario works while all runtime interaction goes through the value/capability SDK rather than handwritten protocol or the superseded callback model.
