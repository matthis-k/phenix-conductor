# Neovim client value/capability precedence

status: partial

## Purpose

This file defines the runtime-facing ABI precedence for `spec/nvim-client.md`. Product behavior stays in that document. Mechanical implementation details stay in `spec/nvim-client-implementation.md`.

Read `spec/value-capability-sdk.md` first.

## Boundary

```lua
local phenix = require("phenix")
local frontend = require("phenix_nvim")
```

The native module projects the generic `(PhenixSchema, PhenixValue)` SDK graph into Lua. Structural values become ordinary tables. Behavior becomes callable proxies. Observable resources expose value-level `get`/`listen` behavior. Lua functions crossing the runtime boundary are lifted to client-owned `CallableRef`s.

`phenix_nvim` owns editor behavior only.

## Runtime adapter

The runtime adapter must:

1. connect through the native client;
2. retrieve and retain the SDK value graph;
3. use callable proxies or thin wrappers for application behavior;
4. use observable value resources for runtime projections;
5. dispatch lifted Lua callbacks only on the Neovim main thread;
6. discard client-owned refs/subscriptions when their connection generation retires.

Do not duplicate schema or operation ids in frontend Lua.

## Observables

Transcript/session projections consume value-level resources:

```lua
local current = phenix.sessions.state.get()
local stop = phenix.sessions.state.listen(opts, function(change)
  -- update projection
end)
```

The listener is a normal Lua function lifted by the generic capability ABI. A missing production observable must be fixed in the owning SDK contribution, not replaced with a private frontend event database.

## Permission and elicitation handlers

Runtime-facing UI handlers use the same generic local-callable mechanism. ACP permission/elicitation mapping stays below the frontend.

The frontend returns typed responses only. Presentation errors or cancellation never broaden authority.

## Client tools

Where frontend or later integration code exposes Neovim behavior as model-visible tools, use #505 semantics:

```text
Lua function
-> client-owned CallableRef
-> tool-definition value
-> session admission
-> ordinary model tool
```

No dedicated client callback registry or client-tool protocol is allowed.

## Compatibility paths

The native binding may expose standard ACP session helpers. They can bridge incomplete application/SDK coverage during implementation, but they are not a second semantic source of truth. The completed #504 path uses the generic SDK value graph for Phenix-specific runtime behavior and observables.

## Completion

This precedence is satisfied when the packaged Neovim acceptance flow uses the generic value/capability SDK for runtime-facing Phenix behavior, with no handwritten protocol, private application schemas, or background-thread Lua execution.
