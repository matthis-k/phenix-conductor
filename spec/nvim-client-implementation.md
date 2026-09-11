# Neovim client implementation handoff

## Purpose

This document is the mechanical implementation plan for `spec/nvim-client.md` and PR #504.

Implement the canonical Neovim frontend after #503 and #505. Do not keep the current standalone frontend as a second implementation. Migrate useful UI/state concepts, delete its handwritten transport/protocol ownership, then make conductor the source of truth.

## Dependency gate

Do not start implementation until the direct parent provides:

### From #503

- generated application operations through `phenix-binding-lua`;
- generated observable resources at SDK `binding_path`s;
- lazy native observable change userdata;
- get/listen/unsubscribe behavior with generation filtering.

### From #505

- host-thread callback dispatch;
- permission handler registration;
- elicitation handler registration;
- client-provided callable/tool registration;
- connection-generation cleanup.

#504 consumes those APIs. It does not duplicate them.

## Fixed decisions

1. The native client module remains `require("phenix")`.
2. The Neovim frontend module is `require("phenix_nvim")`.
3. Never install `clients/nvim/lua/phenix/init.lua`; that would shadow the native module.
4. Canonical frontend source lives in conductor under `clients/nvim`.
5. The standalone `phenix-nvim` repository becomes a deterministic mirror only.
6. Production frontend Lua contains no ACP JSON-RPC framing and no Phenix application schemas.
7. The sidebar has two buffers: read-mostly transcript and editable compose.
8. `Reference` means insert editor context into the current compose document. It never sends.
9. `Send` is the only operation that commits the compose document as a user turn.
10. Selection references are snapshots captured at reference time.
11. Runtime transcript/session/model/tool state is authoritative. Local frontend state only owns unsent compose/UI state.
12. Transcript prose is real Markdown buffer text.
13. Structured transcript nodes retain identity outside their rendered text.
14. Image attachment support does not depend on image preview support.
15. Runtime edit state is authoritative. The frontend reviews edits but does not invent a second edit protocol.
16. #504 ships a built-in review path. #506 makes review and other presentation surfaces replaceable.
17. No optional third-party Neovim plugin is required for the MVP.

## Canonical package layout

Create exactly one frontend package root:

```text
clients/nvim/
  plugin/phenix.lua
  lua/phenix_nvim/init.lua
  lua/phenix_nvim/config.lua
  lua/phenix_nvim/runtime.lua
  lua/phenix_nvim/actions.lua
  lua/phenix_nvim/state.lua
  lua/phenix_nvim/sidebar.lua
  lua/phenix_nvim/compose/model.lua
  lua/phenix_nvim/compose/buffer.lua
  lua/phenix_nvim/reference.lua
  lua/phenix_nvim/context.lua
  lua/phenix_nvim/transcript/model.lua
  lua/phenix_nvim/transcript/buffer.lua
  lua/phenix_nvim/transcript/render.lua
  lua/phenix_nvim/image.lua
  lua/phenix_nvim/review.lua
  lua/phenix_nvim/sessions.lua
  lua/phenix_nvim/status.lua
  lua/phenix_nvim/util.lua
  tests/
```

If implementation proves one module is unnecessary, merge adjacent responsibilities. Do not create a second transport/controller stack.

#506 may later add:

```text
lua/phenix_nvim/integrations/
lua/phenix_nvim/tools/
```

Do not prematurely implement #506 plugin-specific adapters in #504.

## Existing standalone migration map

The current standalone repository is input material, not an API compatibility requirement.

### Delete or fully replace

`lua/phenix/transport.lua`
: delete from production migration. It implements newline-delimited JSON/process/socket framing now owned by `phenix-binding-lua` + ACP.

`lua/phenix/conductor.lua`
: do not carry its request ids, JSON command tables, protocol validation, or event transport forward. Replace its useful connection lifecycle role with `phenix_nvim.runtime` over `require("phenix")`.

Any module that constructs raw conductor commands such as `type = "submit"`, `type = "initialize"`, `type = "get_snapshot"`
: rewrite to generated application operations/observables.

### Reuse concepts, rewrite data source

`controller.lua`
: keep orchestration of UI actions if useful; state must come from `runtime.lua` + observables, not local protocol events.

`controller_actions.lua`
: split stable user actions into `actions.lua`; actions call runtime/compose/sidebar modules rather than constructing transport requests.

`projection.lua`
: useful concept. Replace event-reducer ownership with projection from authoritative transcript/session observables.

`projection_renderer.lua`
: migrate rendering concepts to `transcript/render.lua`; keep structured node identity and incremental range updates.

`session.lua`
: retain local view helpers only. Do not persist canonical session history/state locally.

`session_actions.lua`
: map to generated session operations.

`session_selector.lua`
: keep picker behavior behind `sessions.lua`; #506 later makes picker replaceable.

`markdown.lua`
: retain only Markdown/editor presentation helpers. No transcript state ownership.

`mappings.lua`
: convert mappings to thin calls into `actions.lua`.

`settings.lua`
: migrate frontend-only UI settings. Delete provider/model/runtime semantic configuration that belongs to Phenix.

`execution_tree*.lua`
: keep only if the application API/observables expose the required structured state. Do not preserve a private frontend execution schema.

### Rule for every migrated module

For every table field, ask:

```text
Is this unsent compose/UI state?
  yes -> frontend may own it
  no -> can it be read from generated application state?
       yes -> remove local authority and project it
       no -> add/fix the application contract in the appropriate upstream PR; do not invent a Lua mirror
```

## Runtime module

`runtime.lua` is the only frontend module that directly owns the native `phenix` client userdata.

Suggested state:

```lua
Runtime = {
  client = <native Client>,
  application = <generated application table>,
  connection = "disconnected" | "connecting" | "connected" | "failed",
  active_session_id = nil,
  subscriptions = {},
  callback_dispatch_active = false,
}
```

Other modules receive `Runtime` methods or observable projections. Do not call `require("phenix")` from every UI file.

### Connection setup

1. Load native module with `local native = require("phenix")`.
2. Call `native.connect` with configured command/args/env.
3. Obtain `client:application()`.
4. Install permission/elicitation handlers from #505 using built-in #504 UI implementations.
5. Start host polling/dispatch loop.
6. Resolve required observable resources.
7. Resume/select active session if configured/available.
8. Subscribe transcript/session state.
9. Mark connection ready only after required client-facing surfaces exist.

If a required capability is unavailable, show one structural connection error and keep sidebar recoverable. Do not nil-index generated operations later.

### Poll/dispatch loop

The native client is passive. Neovim owns scheduling.

Use one timer/scheduled loop for:

- session notifications needed by ACP compatibility;
- #503 observable delivery dispatch;
- #505 callback dispatch;
- request completion polling where native handles require it.

Do not create one timer per observable/tool.

Constraints:

- all Lua callbacks execute on Neovim main thread;
- stop timer on client teardown;
- no busy loop;
- use a bounded amount of work per tick;
- if more work remains, schedule another tick rather than blocking UI;
- no host callback may execute from a libuv worker callback without `vim.schedule`/main-thread handoff.

Start with a small fixed tick while connected. Optimization to activity-aware cadence is allowed after correctness; do not build a complex scheduler first.

## Local frontend state

`state.lua` owns only:

```text
sidebar open/focus/layout state
remembered compose cursor
active compose document
render range/extmark maps
follow-tail flag
selected active session id as frontend preference
transient request/error state
```

It must not own canonical:

```text
session transcript
session persistence
model catalog
routing catalog
runtime tool catalog
execution truth
permission authority
edit acceptance truth
```

Those are projected from Phenix.

## Compose document model

### Canonical local representation

Use ordered segments:

```text
ComposeDocument {
  segments: [
    Text,
    SelectionReference,
    FileReference,
    DiagnosticReference,
    DiffReference,
    ImageAttachment,
    ...
  ]
}
```

Every non-text segment has a stable local id.

Suggested Lua shape:

```lua
{
  id = "ref-17",
  kind = "selection",
  source = {
    uri = "file:///workspace/foo.rs",
    start_line = 20,
    start_column = 1,
    end_line = 38,
    end_column = 4,
  },
  snapshot = "...exact selected bytes/text...",
}
```

Use 0/1-based coordinates consistently. Convert Neovim's coordinate convention at the boundary and document it in one helper.

### Buffer projection

The compose buffer is an editor for the document, not the authoritative storage for attachment payloads.

Recommended MVP projection:

- text remains real editable buffer text;
- a reference is anchored by an extmark and one visible marker/header line;
- reference preview may be buffer lines or virtual lines, but the snapshot payload remains in `ComposeDocument`;
- image preview is anchored to the attachment extmark;
- `references_by_mark[extmark_id] -> segment_id` keeps identity stable.

Do not serialize a selection by rereading the source buffer at send time.

### Interleaving text and references

The send serializer must preserve exact composition order.

Implement one function with tests:

```text
compose.serialize(buffer, extmarks, document)
  -> ordered application Content/attachment values
```

Algorithm:

1. get all active reference extmarks with positions;
2. sort by buffer position;
3. emit text between prior position and reference anchor as `Text` if non-empty;
4. emit the reference segment from local model;
5. continue after its rendered marker range;
6. emit trailing text;
7. preserve two distinct references even if same source range was referenced twice;
8. fail structurally if an extmark references a missing segment rather than silently dropping context.

### Reference edit/delete behavior

MVP behavior:

- deleting the marker removes the reference from document on next reconciliation;
- explicit `ReferenceRemove` action deletes marker + model segment;
- editing display text must never mutate the stored selection snapshot;
- if implementation cannot make a reference display region selectively read-only, treat edits to the preview as presentation-only or detach the reference explicitly. Do not silently reinterpret edited preview as source snapshot.

## `Reference` action

### Visual mode

1. capture exact selection before changing window/focus;
2. capture source URI/path and exact range;
3. copy text immediately;
4. restore/focus sidebar compose window;
5. insert reference at remembered compose cursor;
6. move compose cursor after inserted reference;
7. enter/retain insert mode according to normal compose behavior;
8. do not send.

The capture must work for characterwise, linewise, and blockwise selections or explicitly reject unsupported blockwise mode with a clear error. Do not silently capture the wrong range.

### Normal mode

Capture:

```text
current file URI
cursor line/column
```

MVP does not need automatic whole-function expansion. #506/polished target may add Tree-sitter providers.

### Remembered cursor

When leaving the compose window, record cursor position/extmark anchor. When `Reference` is invoked from another window, insert at that position even if sidebar is hidden and reopened.

Regression example:

```text
select A -> Reference -> type question A
leave sidebar
select B -> Reference -> type question B
send
```

Serialized sequence must be:

```text
A, "question A", B, "question B"
```

## Explicit context picker

#504 ships a built-in picker only.

Sources:

- current selection if available;
- current file/location;
- open buffers;
- current diagnostics;
- quickfix/location list;
- git diff only if available through a stable built-in source without introducing hard plugin dependency;
- image file attachment separately.

Use `vim.ui.select` or a normal scratch-buffer picker. #506 replaces/extends this through adapters.

Typed `@...` syntax is UI sugar. Parse it into the same reference segment constructors used by `Reference` and the picker. Do not maintain three separate reference representations.

## Send action

`actions.send()` performs:

1. require connected runtime;
2. resolve/create active session;
3. serialize compose document in order;
4. validate at least one content segment exists;
5. call generated prompt operation with typed multimodal content;
6. keep compose document intact while request is pending;
7. after runtime accepts the prompt, clear the exact submitted document version;
8. if user edited compose while request was pending, do not clear the newer edits;
9. on error, leave submitted content available for retry and render/notify error.

Use a compose generation counter:

```text
compose_revision += 1 on local edit/reference change
submitted_revision = compose_revision
clear only if current_revision == submitted_revision
```

This prevents losing text typed while the request is in flight.

## Sidebar

`sidebar.lua` owns windows, not transcript semantics.

Layout:

```text
single right/left sidebar container
  transcript window/buffer
  compose window/buffer below
```

Requirements:

- configurable side and width;
- compose height grows to a reasonable cap or has explicit configured height;
- toggle does not destroy buffers/state;
- close windows without closing runtime session;
- focus compose/transcript actions;
- restore remembered compose cursor;
- resize updates image placements;
- no floating windows required for core interaction.

## Transcript model

### Source of truth

Use #503 generated observable state for transcript/session state. Do not rebuild authoritative transcript from generic `client:poll()` events.

A transcript projection should maintain stable node ids such as:

```text
turn/user message id
assistant message id
execution id
call id
edit/review id
permission request id
image id
```

If current runtime state lacks a stable id needed for incremental rendering, fix the application-facing observable upstream rather than use rendered text as identity.

### Local projection

Suggested model:

```lua
TranscriptProjection = {
  order = { node_id, ... },
  nodes = { [node_id] = structured_node },
  version = <observable value version>,
  commit = <last commit id>,
}
```

Apply initial full snapshot, then diffs.

If observable version is not expected next version:

1. stop applying diffs;
2. call observable `get()` for fresh full state;
3. rebuild projection;
4. resume subscription from current state according to #503 API.

Never guess through a version gap.

## Transcript buffer renderer

The transcript buffer is ordinary Markdown text plus extmarks.

Maintain:

```text
node_id -> { start_mark, end_mark, renderer_kind }
```

### Prose

Write actual Markdown text. Do not conceal the entire transcript into virtual text.

### Structured nodes

Render tool calls/results, permission blocks, edits, references, images, errors, and progress as stable ranges/extmarks.

MVP tool block example:

```text
### Tool · bash
`cargo test -p phenix-core`

<foldable output>
```

Exact decoration may differ. Identity must be `call_id`, not header text.

### Incremental update algorithm

For observable diff:

1. map changed structured node id(s);
2. update only affected buffer range;
3. move/update extmarks automatically;
4. do not rebuild whole transcript per token;
5. for text delta, append into existing assistant message range;
6. for tool lifecycle update, mutate same call block;
7. for completion, preserve block and change status decoration.

A full rerender is allowed only for:

- initial snapshot;
- version-gap recovery;
- renderer invariant failure recovery;
- explicit refresh.

## Follow-tail behavior

Track `follow_tail` per transcript window.

Before applying update:

- if cursor/window view is at or near final transcript line, keep follow enabled;
- if user scrolled away, disable follow;
- when update arrives and follow is enabled, scroll to end;
- returning manually to end can re-enable follow;
- provide explicit action to re-enable.

Do not force cursor to bottom while user is reading history.

## Images

### Attachment model

Store:

```text
id
name
mime_type
bytes or stable file-backed source as supported by application contract
width/height when known
```

Do not make preview renderer the owner of bytes.

### Acquisition

MVP required source: image file path chosen through built-in picker/input.

Validate:

- file exists/readable;
- supported image content/mime according to prompt contract;
- read failure is client error;
- no preview support does not block attachment.

Clipboard acquisition is #506/polished work unless trivial platform-independent support already exists.

### Preview abstraction in #504

Implement an internal default renderer API even before #506 exposes it publicly:

```lua
image.render(attachment, anchor, window) -> handle
handle:update(...)
handle:close()
```

If `vim.ui.img` exists:

- render through it;
- anchor to extmark position;
- recompute on scroll/resize;
- hide/delete when anchor is not visible or window closes.

Otherwise render text:

```text
[image: name · WIDTHxHEIGHT]
```

No code outside `image.lua` calls `vim.ui.img` directly.

## Permission UI

#505 provides typed callback host. #504 ships one dependency-free renderer.

Use sidebar structured block or `vim.ui.select` fallback.

Requirements:

- display description/call context;
- only return contract-allowed values;
- `AllowOnce`, `Deny`, `Cancelled` map exactly;
- closing UI returns `Cancelled`, not allow;
- only one response is accepted;
- callback completion does not mutate runtime authority locally.

#506 may replace presentation.

## Elicitation UI

#505 provides schema + callback response.

#504 needs a basic dependency-free path sufficient for supported standard forms:

- string;
- boolean;
- integer/number;
- optional primitive;
- top-level record/table of those fields.

Use `vim.ui.input` / `vim.ui.select` sequentially if necessary for #504. #506 implements polished questionnaire/form presentation.

The schema remains authoritative. Validate before responding.

## Native review path

`review.lua` receives a structured runtime edit/review object.

MVP can use normal Neovim diff buffers/windows.

Required actions:

```text
open review
next/previous hunk
accept supported runtime granularity
reject supported runtime granularity
close view without deciding
refresh after conflict/runtime change
```

Rules:

- never mark accepted before runtime confirms action;
- never write a second patch protocol;
- if current buffer changed since runtime base, show conflict instead of overwriting;
- buffer synchronization after accepted runtime edit should form meaningful undo boundary;
- close/reopen review must preserve pending runtime state.

Build `review.lua` behind an internal adapter-shaped function so #506 can swap renderer without changing caller.

## Sessions

`sessions.lua` wraps generated operations/observables.

Required:

```text
create/reuse session on first send
new session
list sessions
select active session
resume selected session
close session if exposed
restore last active session preference when still valid
```

Frontend may persist only a small preference such as last selected session id if desired. It must not persist transcript/session content.

On Neovim restart:

1. connect runtime;
2. list/resume sessions from Phenix;
3. choose saved valid session or latest/default policy;
4. get initial transcript snapshot;
5. subscribe;
6. render.

## Status

`status.lua` maintains a cheap local projection of already-observed data:

```text
connection
active session
execution state
model/router summary if available
pending permission/elicitation count
```

Do not perform synchronous runtime requests from statusline evaluation.

#506 exposes public integration hooks.

## Stable actions

`actions.lua` is the only public action surface used by default mappings/commands.

Required functions:

```text
reference
send
toggle
focus_compose
focus_transcript
cancel
new_session
choose_session
attach_image
review_current
```

`plugin/phenix.lua` commands and `mappings.lua` equivalents call these functions only.

Do not put business logic directly in keymap callbacks.

## Configuration

Frontend-only settings:

```text
runtime command/args/env or packaged runtime selection
sidebar side/width/compose height
poll dispatch budget/cadence
keymap enable/overrides
image preview enable
optional local last-session preference
```

Not frontend settings:

```text
model provider credentials
runtime plugin graph
routing semantics
agent definitions
permission authority policy
session persistence policy
```

Those belong to Phenix configuration/runtime.

## Nix packaging

Extend `modules/package-sets.nix`.

Add a Vim plugin package built from `clients/nvim`.

Target exports:

```text
packages.phenix-binding-lua
packages.phenix-nvim
```

`phenix-nvim` source package must not copy `phenix.so` into its repository tree.

Add a packaged headless check whose runtime environment includes:

- Neovim;
- `phenix-nvim` on runtimepath;
- `phenix-binding-lua` on Lua C path;
- packaged ACP/Phenix runtime fixture needed for e2e.

At minimum assert:

```lua
local native = require("phenix")
local frontend = require("phenix_nvim")
assert(native.interface_id == "phenix.application@1")
assert(type(frontend.setup) == "function")
```

Then run actual startup/session/reference/transcript tests.

## Standalone mirror

### Source of truth

Only `clients/nvim` in conductor is editable implementation source after migration.

The standalone repository must contain exported frontend source plus mirror metadata. It must not retain independent transport/protocol implementation.

### Export script

Add one deterministic script, suggested path:

```text
scripts/export-nvim-client
```

Inputs:

```text
conductor source tree
source revision
output directory
```

Output:

```text
plugin/
lua/
doc/ if present
README/release files explicitly owned by mirror
.phenix-conductor-revision
```

Rules:

- clean output before copy;
- sorted/deterministic file set;
- no timestamps in generated content;
- no native binaries;
- no conductor Rust/Nix internals unless deliberately mirror metadata;
- source revision file contains exact conductor commit SHA.

### Parity CI

Add a conductor check that exports `clients/nvim` into a fixture/temp tree and verifies deterministic output.

Standalone repository follow-up CI should compare its source tree to export from recorded conductor revision.

Do not make two-way sync.

## Implementation phases

### Phase 1: create canonical package shell

- create `clients/nvim` tree;
- native/frontend namespace split;
- `setup`, commands, config skeleton;
- package/check loads both modules.

No UI migration yet.

### Phase 2: runtime adapter

- connect native client;
- generated application operations;
- host callback dispatch loop;
- capability failure handling.

Delete dependency on old `transport.lua`/raw command protocol in migrated code.

### Phase 3: session + observable projection

- list/create/resume session;
- initial observable full state;
- subscription;
- version-gap recovery.

Pass headless state tests before transcript rendering.

### Phase 4: sidebar shell

- transcript and compose buffers/windows;
- toggle/focus/resize;
- remembered compose cursor.

### Phase 5: compose + Reference

- segment model;
- extmark anchors;
- selection/current-location capture;
- multi-reference ordering;
- serialize to typed prompt content;
- send revision protection.

### Phase 6: transcript renderer

- Markdown prose;
- incremental message/tool updates;
- structured block identities;
- follow-tail behavior;
- full recovery path.

### Phase 7: permission + basic elicitation

Use #505 host callbacks with dependency-free UI.

### Phase 8: images

- file attachment;
- prompt send;
- `vim.ui.img` renderer if available;
- textual fallback;
- cleanup on window/buffer lifecycle.

### Phase 9: native edit review

- structured runtime edit input;
- native diff;
- accept/reject confirmation;
- conflict behavior.

### Phase 10: migration cleanup

Audit migrated standalone code:

- no raw transport;
- no duplicate schemas;
- no frontend session DB;
- no direct model/provider logic.

### Phase 11: mirror/export

- deterministic export script;
- revision marker;
- parity check.

### Phase 12: packaged e2e

Run the exact acceptance flow from `spec/nvim-client.md`.

## Test matrix

### Module/load

- native `require("phenix")` and frontend `require("phenix_nvim")` coexist;
- frontend package does not shadow native module;
- no optional plugin installed still loads.

### Runtime

- connect/disconnect/reconnect;
- required capability missing gives stable UI error;
- one polling loop dispatches bounded work;
- timer stops on teardown;
- callbacks run main thread.

### Compose

- selection snapshot exact;
- source edit after reference does not change snapshot;
- A/question A/B/question B order;
- remembered insertion cursor across windows;
- duplicate same reference remains two segments;
- remove reference;
- text typed during pending send not cleared;
- failed send preserves compose.

### Transcript

- initial full snapshot;
- diff update only affected node;
- text streaming extends one message;
- tool lifecycle mutates one call block;
- version gap causes full recovery;
- manual scroll stops follow;
- replay/resume renders same transcript as live sequence.

### Images

- file attachment sends without preview API;
- preview API path when mocked/available;
- resize/reposition;
- cleanup;
- textual fallback.

### Sessions

- first send creates/reuses session;
- selection/resume;
- restart restores runtime transcript;
- no local transcript DB.

### Permission/elicitation

- allow/deny/cancel;
- scalar form;
- invalid typed answer not sent;
- UI close cancels.

### Review

- pending diff opens;
- accept/reject reaches runtime;
- local conflict surfaced;
- frontend does not claim success before runtime response.

### Packaging/mirror

- Nix package load;
- native binding in runtime closure/test environment;
- deterministic export twice yields byte-identical tree;
- no `.so` in exported source;
- source revision exact.

## Source/enforcement checks

Add cheap deterministic checks:

- reject production `lua/phenix/` directory inside `clients/nvim`;
- reject `require("phenix.transport")`;
- reject raw `_phenix/` strings in frontend Lua;
- reject raw JSON-RPC method framing in frontend Lua;
- reject checked-in `phenix.so` under `clients/nvim`;
- reject a standalone session/transcript persistence database;
- ensure mappings call stable action module.

## Do not invent

Do not add:

- a socket mode beside native ACP client;
- compatibility fallback to old conductor JSON protocol;
- frontend-owned ACP schemas;
- frontend-owned session history database;
- model/provider registry inside Neovim;
- hidden automatic whole-repo context capture;
- automatic send from `Reference`;
- one input buffer containing both mutable historical transcript and current compose;
- full transcript rerender per token;
- image preview as prerequisite for image send;
- direct file mutation to simulate runtime edit acceptance;
- a second implementation in standalone `phenix-nvim`.

## Completion gate

#504 is complete only when:

- [ ] canonical source exists under `clients/nvim`;
- [ ] native `phenix` and frontend `phenix_nvim` namespaces coexist;
- [ ] old raw transport/protocol is absent from production frontend;
- [ ] runtime uses generated operations/observables;
- [ ] sidebar has separate transcript/compose buffers;
- [ ] `Reference` captures exact visual selection/current location without sending;
- [ ] multi-reference compose ordering is tested;
- [ ] one typed multimodal `Send` works;
- [ ] transcript is incremental Markdown + structured nodes;
- [ ] observable gap recovery works;
- [ ] permission/basic elicitation work through #505;
- [ ] image attachment works with preview and fallback;
- [ ] session restart/resume restores runtime state;
- [ ] native review path is runtime-confirmed;
- [ ] Nix package/check passes;
- [ ] deterministic one-way mirror export exists;
- [ ] full packaged acceptance flow passes;
- [ ] Source, Product, Docs, and Maintenance pass on exact head.
