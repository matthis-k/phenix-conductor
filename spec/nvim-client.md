# Canonical Neovim client

status: partial

## Purpose

`clients/nvim` is the canonical Phenix Neovim frontend. The standalone `phenix-nvim` repository is a deterministic export target, not a second implementation.

The frontend is an Application. It consumes the native `phenix` client binding and the generic value/capability SDK. It does not implement runtime plugins, session persistence, ACP framing, application schemas, or model/provider logic.

## Module boundary

```lua
local phenix = require("phenix")
local frontend = require("phenix_nvim")
```

Rules:

- `require("phenix")` remains the native binding.
- `clients/nvim/lua/phenix/init.lua` must never exist.
- only `phenix_nvim.runtime` owns native client userdata;
- other frontend modules consume runtime methods or projected semantic state;
- production Lua contains no raw ACP JSON-RPC, Phenix extension method ids, or private application schema copies.

## State ownership

Phenix owns durable and runtime truth:

- sessions and transcript;
- execution and tool lifecycle;
- models and routing;
- permissions and elicitation requests;
- edit/review state;
- persistence.

The frontend may own only UI and unsent state:

- sidebar layout and focus;
- remembered compose cursor;
- unsent compose document and attachment snapshots;
- transcript render ranges/extmarks derived from runtime state;
- follow-tail state;
- transient request/UI errors.

If required runtime state is missing from the application SDK, fix that contract or contribution. Do not create a second Lua database.

## Runtime adapter

`phenix_nvim.runtime` owns one native client and one bounded Neovim main-thread dispatch loop.

It must:

1. connect through the native binding;
2. retrieve the SDK value graph;
3. retain schema-backed SDK values/callable proxies;
4. create, list, resume, select, cancel, and prompt sessions through public client/application surfaces;
5. dispatch native callbacks only from the Neovim main thread;
6. expose semantic updates to frontend projections;
7. stop polling on teardown;
8. surface structural connection errors without corrupting compose state.

The final transcript/session path uses production SDK observables. Standard ACP session wrappers may be used only as a temporary compatibility path while that production observable contribution is completed.

## Compose and Reference

The compose buffer is an editable multimodal document. `Reference` inserts context at the remembered compose cursor and never sends.

Required flow:

```text
select A -> Reference -> type question A
select B -> Reference -> type question B
Send
```

The serialized order is:

```text
A, "question A", B, "question B"
```

Reference rules:

- visual selection captures exact text and source range before focus changes;
- normal mode captures current file/location;
- selection content is a snapshot and source edits cannot mutate it;
- unsupported selection modes fail clearly instead of capturing the wrong range;
- references retain stable local identity;
- moving/deleting a marker updates the local compose projection;
- built-in picker and typed `@...` references use the same typed compose item constructors.

`Send` serializes text, references, and images in buffer order. It keeps the submitted compose revision until runtime acceptance. Newer user edits are never cleared by an older in-flight send.

## Sidebar

The sidebar uses separate buffers for transcript and compose.

Requirements:

- toggle/focus/resize without destroying compose state;
- remembered compose cursor survives leaving or hiding the sidebar;
- transcript is read-mostly normal Markdown;
- compose remains a normal editable buffer;
- no optional third-party plugin is required.

## Transcript projection

Runtime state is authoritative. The frontend maintains only a projection keyed by stable semantic ids such as message id, execution id, call id, review id, or permission id.

Requirements:

- initial full state comes from the runtime observable;
- ordered diffs update the projection;
- a version gap stops diff application and triggers a fresh full read;
- assistant streaming mutates one message range;
- tool call/result/failure mutates one structured block keyed by call id;
- prose remains real Markdown buffer text;
- full rerender is limited to initial load, gap recovery, invariant repair, or explicit refresh;
- manual scrolling disables follow-tail until the user returns to the end or explicitly re-enables it.

Rendered text is never identity or persistence.

## Permissions and elicitation

The frontend supplies dependency-free built-in presentation for runtime requests through the generic local-callable host.

Permission rules:

- return only a choice declared by the request;
- close/Esc cancels;
- UI failure never widens authority;
- frontend presentation does not persist permission authority.

Basic elicitation is schema-driven. Candidate values are validated against the authoritative `PhenixSchema`. Do not parse free-form prose into structured answers.

## Images

Images are first-class compose/transcript items.

- sending does not depend on preview support;
- file attachment is the universal acquisition fallback;
- `vim.ui.img` is used when available;
- unsupported API/terminal combinations render a stable textual attachment;
- preview placement follows its extmark/window and is released with the owning view;
- bytes/metadata belong to the attachment model, not the renderer.

## Runtime edit review

Edit truth stays in Phenix. The built-in reviewer presents structured runtime review data with normal Neovim buffers/windows.

It must support:

- changed files and hunks;
- next/previous change;
- runtime-backed accept/reject actions;
- waiting for runtime confirmation;
- explicit conflict display;
- no local patch application that pretends runtime acceptance occurred.

#506 may replace presentation through adapters without changing these semantics.

## Sessions and status

The frontend uses Phenix persistence. It does not maintain chat history.

MVP session behavior:

- first send creates or reuses an active session;
- create/list/resume/select sessions;
- resume after Neovim restart;
- cancel active generation;
- show connection/runtime failures in the sidebar.

Status reads are cheap projections of already-known state. Statusline rendering must not perform runtime I/O.

## Packaging and export

The flake exposes:

- `packages.<system>.phenix-nvim`, containing the frontend and compatible native `phenix.so`;
- `packages.<system>.phenix-nvim-export`, containing frontend source plus the conductor revision marker and no native binary.

Required checks load both `require("phenix")` and `require("phenix_nvim")` in packaged headless Neovim and run frontend behavior regressions.

The standalone mirror is generated from `phenix-nvim-export`. Conductor remains the only implementation source of truth.

## Acceptance

#504 is complete when one packaged flow proves:

```text
launch Neovim
-> load native phenix + phenix_nvim
-> connect to packaged Phenix ACP application
-> create/resume a durable session
-> Reference A, type question A
-> Reference B, type question B
-> attach image
-> Send one ordered multimodal turn
-> stream transcript/tool activity through SDK observables
-> handle permission and basic elicitation
-> review a runtime edit
-> close Neovim
-> reopen and resume the same session/transcript
```

The exact head must pass Source, Product, Docs, Maintenance, and the frontend package checks.

## Non-goals

Do not add:

- a second socket or JSON transport;
- frontend-owned application schemas;
- frontend-owned canonical session/transcript state;
- automatic send from `Reference`;
- combined transcript/compose authority;
- full transcript rerender per token;
- an image-preview prerequisite for sending;
- local edit application as fake runtime acceptance;
- dedicated callback semantics where callable values already apply;
- plugin-host APIs in the client binding.
