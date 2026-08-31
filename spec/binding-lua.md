# Lua client binding

Status: specification only. Implement on the application-side ACP client SDK from #440.

## Goal

Provide an importable Lua **Binding** over `phenix-client-acp` so Lua applications can use Phenix without implementing ACP or importing conductor/runtime internals.

The first-party Rust crate/package is `phenix-binding-lua`.

It builds a native Lua module loadable as:

```lua
local phenix = require("phenix")
```

The primary consumer is `phenix-nvim`, but the binding must not depend on Neovim APIs.

## Boundary

```text
Lua application
  phenix-nvim / another Lua host
        |
        v
phenix-binding-lua
        |
        v
phenix-client-acp
        |
        | ACP + negotiated `_phenix/...` extensions
        v
phenix-adapter-acp
        |
        v
Phenix runtime
```

The binding wraps the Client SDK. It does not reimplement ACP framing, request correlation, capability negotiation, extension schemas, session reconstruction, or transport logic.

## Runtime compatibility

Support the Lua ABI used by Neovim first, including LuaJIT 2.1 / Lua 5.1-compatible loading.

Build as a Rust `cdylib` or equivalent native module artifact. Package it under a conventional Lua module path so consumers can add the package to `package.cpath` and `require("phenix")` without copying generated files into their source tree.

Do not bundle a second Lua runtime into Neovim.

## Lua API

Expose small Lua-native objects rather than raw Rust or JSON-RPC types.

Initial shape:

```lua
local phenix = require("phenix")

local client = phenix.connect({
  -- transport/application launch options
})

local capabilities = client:capabilities()
local sessions = client:sessions()
local session = sessions:new()

local request = session:prompt("hello")
```

Expected objects include:

- `Client`;
- `Sessions` or equivalent session collection API;
- `Session`;
- request/turn handles;
- typed event/update tables;
- typed capability/extension descriptors;
- typed Phenix extension helpers for skills, orchestration/callables, lineage, routing metadata, execution inspection, provenance, and diagnostics when negotiated.

Lua tables may represent structured values at the language boundary, but conversion rules must be deterministic and reject incompatible shapes rather than silently coercing them.

## Async and host integration

The binding must not block the Lua host event loop while waiting for model, tool, transport, or server work.

Keep the binding host-neutral:

- Rust may own background async work;
- requests return handles rather than blocking until completion;
- applications can drain/poll queued events without blocking, or register callbacks through a host-neutral mechanism;
- callbacks into Lua occur only through a safe application-driven dispatch point;
- the Rust binding never calls `vim.*`, `vim.loop`, `vim.uv`, or other Neovim globals.

`phenix-nvim` can integrate polling/dispatch with Neovim's event loop on the Lua side.

## State and lifetime

Lua userdata/objects are handles over application-side SDK state.

- garbage-collecting a local `Session` handle does not delete the durable Phenix session;
- explicit close maps to the ACP close operation where requested;
- client disconnect releases local protocol resources but does not imply durable session deletion;
- request and event queues are bounded;
- dropped/closed objects fail with typed Lua-visible errors rather than panicking.

## Errors

Translate SDK errors into stable Lua-visible error values with at least:

- kind;
- message;
- optional server/protocol code;
- optional structured details safe for applications.

Distinguish transport, protocol, unsupported-capability, server-rejection, cancellation, and conversion errors.

Do not expose Rust backtraces, internal runtime types, or secrets through normal error values.

## Capabilities and extensions

Expose negotiated ACP and Phenix extension capabilities directly so Lua applications can feature-detect behavior.

A Lua application must be able to use standard ACP operations and supported Phenix extensions without constructing raw `_phenix/...` JSON-RPC requests.

Keep one implementation of each extension schema in `phenix-client-acp`; the Lua binding only converts between Rust SDK values and Lua values.

## Transport

Transport selection is passed to the Client SDK.

The binding may expose constructors for supported SDK transports such as spawned stdio or socket-backed connection options, but socket mechanics stay in `phenix-transport-socket` and protocol logic stays in `phenix-client-acp`.

## Packaging for Neovim

Expose an independently buildable Nix package for the native module.

`phenix-nvim` should be able to add that package as a dependency, extend its Lua native-module search path, and then use:

```lua
local phenix = require("phenix")
```

The Neovim repository should not vendor generated Rust/Lua binding artifacts or duplicate ACP protocol code.

## Regression coverage

- the built native module loads through `require("phenix")` under the supported LuaJIT/Lua ABI;
- loading the module performs no connection or runtime mutation;
- a Lua client can initialize, create/list/resume a session, prompt, receive ordered updates, and cancel;
- Phenix extension capabilities and typed helpers are available only when negotiated;
- the host thread/event loop is not blocked by an active request;
- garbage collection of local handles does not delete durable sessions;
- Lua/Rust value conversion rejects incompatible shapes predictably;
- transport/protocol/server/cancellation errors remain distinguishable;
- the binding contains no Neovim-specific API calls;
- `phenix-nvim` can consume the packaged module without vendoring protocol logic.

## Completion

- [ ] `phenix-binding-lua` builds an importable native Lua module named `phenix`;
- [ ] it depends on `phenix-client-acp` rather than reimplementing ACP;
- [ ] LuaJIT/Neovim loading is supported;
- [ ] the API is asynchronous and host-neutral;
- [ ] standard ACP and Phenix extensions are exposed as Lua-native operations;
- [ ] transport stays below the Client SDK;
- [ ] durable runtime state remains Phenix-owned;
- [ ] exact-head Source, Rust, Product, and Maintenance validation passes.
