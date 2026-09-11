# Canonical Neovim client implementation handoff

status: partial

## Purpose

Mechanical handoff for PR #504. Product semantics are in `spec/nvim-client.md`. Runtime ABI precedence is in `spec/nvim-client-value-capability.md` and `spec/value-capability-sdk.md`.

## Current dependency state

#503 and #505 are on main. The native Lua binding already provides:

- process connection through the ACP client;
- `client:sdk()` and descriptor-backed application projection;
- standard session handles;
- bounded `client:poll()` host dispatch;
- asynchronous request handles;
- generic callable lifting and invocation;
- observable projection machinery when an SDK contribution supplies an observable;
- `phenix.tools.register` over lifted callable refs and client-tool admission.

Two upstream runtime gaps are still material to #504 acceptance:

1. production SDK contributions do not yet publish the transcript/session observables required by the frontend;
2. `phenix-acp-stdio` has no configured-runtime bridge or packaged `phenix-acp` executable, so a packaged editor cannot spawn the real application yet.

Fix those runtime surfaces instead of compensating with private Lua protocols.

## Canonical package

Source lives only under:

```text
clients/nvim/
  plugin/phenix.lua
  lua/phenix_nvim/
  tests/
```

Never create `clients/nvim/lua/phenix/init.lua`.

The current module split is intentionally small:

```text
runtime.lua            native client ownership and bounded dispatch
config.lua             frontend-only settings
state.lua              unsent compose/UI state
sidebar.lua            transcript/compose windows
context.lua            normalized editor location/selection capture
actions.lua            stable user actions
compose/model.lua      snapshot attachments/references
compose/buffer.lua     editable compose projection + ordered serialization
transcript/model.lua   stable-id runtime projection
transcript/buffer.lua  Markdown/extmark rendering
image.lua              attachment acquisition/preview fallback
review.lua             native structured review presentation
sessions.lua           session presentation helpers
status.lua             cheap projected status
util.lua               local helpers
```

Merge modules when responsibilities collapse. Do not add a second controller/transport stack.

## Runtime implementation

`runtime.lua` is the only production frontend module that may call `require("phenix")`.

State may include:

```text
client
sdk/application projection
connection status/error
active session handle
pending native request handles
one poll timer
semantic update listeners
```

It must not store canonical transcript/session history.

Use one bounded main-thread timer for native event/callback dispatch and request completion. Never create one timer per subscription/tool/request.

Connection readiness requires the runtime-facing surfaces the frontend needs. Missing capability is a structural connection error, not a later nil-index failure.

## Sessions and prompt

Use public native/application surfaces for create/list/resume/select/cancel.

The temporary text-only path may use the standard ACP session helper. Ordered multimodal Send is complete only after text, resource references, and image bytes cross the public application/SDK prompt boundary without frontend schema duplication.

## Compose implementation

The buffer owns editable text. `ComposeDocument` owns non-text payloads keyed by stable local ids. Visible markers/extmarks connect the two.

Serialization:

1. read the compose buffer in order;
2. emit text before each reference marker;
3. emit the referenced snapshot/attachment;
4. continue after the marker;
5. preserve duplicate references as distinct items;
6. fail on a marker whose item is missing;
7. reconcile deleted markers without mutating source snapshots.

Visual selection is captured before sidebar focus changes. Characterwise and linewise selections must be exact. Unsupported blockwise capture fails explicitly until implemented correctly.

Track a compose revision. Clear after Send only when the accepted submission revision still equals the current revision.

## Transcript implementation

Use stable semantic ids. Never key by rendered text.

Projection update rules:

- full observable snapshot initializes state;
- each diff has the expected next version/sequence;
- a gap triggers a fresh full read;
- text deltas update one assistant node;
- tool lifecycle updates one call-id node;
- progress/diagnostics update their own stable node;
- renderer updates only affected extmark ranges;
- full rerender is recovery, not the streaming path.

Prose stays normal Markdown buffer text.

The current frontend package contains model/render primitives and deterministic regressions. Live transcript wiring remains incomplete until the production SDK observable is contributed.

## Sidebar and actions

Stable actions are independent of mappings:

```text
reference
send
toggle
cancel
new_session
choose_session
attach_image
```

Commands are thin wrappers over these actions.

Sidebar buffers survive window close/toggle. The remembered compose cursor is restored before `Reference` inserts from another editor window.

## Images

File acquisition stores bytes and MIME metadata independently from preview. Preview uses `vim.ui.img` only when available and falls back to a textual attachment. Multimodal transport acceptance remains open until the public prompt path carries image content end to end.

## Permission, elicitation, and review

Do not add presentation-specific runtime protocols.

- permission UI may return only declared choices;
- Esc/close/error cancels rather than allowing;
- elicitation builds typed candidate values and validates them against the source schema;
- review presentation receives structured runtime review data and invokes runtime-backed actions;
- no reviewer may apply local patches to fake acceptance.

The current native review file is presentation scaffolding only. Runtime-backed review remains open until the application surface supplies the structured review object/actions.

## Packaging

`modules/nvim-client.nix` owns:

- `packages.phenix-nvim`, combining frontend source with the compatible native `phenix.so`;
- `packages.phenix-nvim-export`, a binary-free deterministic mirror source with conductor revision marker;
- headless load and frontend behavior checks.

The package check must prove:

- native `require("phenix")` loads;
- `require("phenix_nvim")` loads;
- only `runtime.lua` imports the native module in production Lua;
- no `lua/phenix/init.lua` shadows the native module;
- production frontend Lua contains no raw `_phenix/...` method ids;
- compose order/snapshot behavior and transcript gap/tool identity regressions pass.

## Remaining implementation order

1. make production session/transcript state available as SDK observables;
2. add the configured-runtime ACP stdio bridge/executable;
3. wire runtime observable subscriptions and gap recovery;
4. carry ordered multimodal prompt content through the public prompt surface;
5. install built-in permission/basic elicitation handlers through generic callables;
6. expose structured runtime review and wire native review actions;
7. complete image placement cleanup and live transcript follow-tail;
8. export/update the standalone mirror and add parity validation;
9. run the full packaged acceptance scenario;
10. remove `.phenix/work/pr-504.md` and validate the cleaned exact head.

## Completion gate

Do not call #504 complete until the PR checklist matches current remote HEAD, the packaged flow in `spec/nvim-client.md` passes, the PR working-state file is absent, and exact-head Source, Product, Docs, Maintenance, and frontend package checks are green.
