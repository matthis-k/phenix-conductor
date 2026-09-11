# Neovim client package

## Status

Follow-up to #503. Implement only after the generated Lua application binding exposes observable resources with native lazy change handles.

This specification defines the canonical Phenix Neovim client package and its standalone mirror. The Neovim package is a client of the application API. It does not implement or embed the Phenix plugin ABI.

## Goals

- Make editor-to-agent interaction require as little ceremony as normal Neovim editing.
- Keep the Neovim package thin. Phenix owns sessions, models, routing, tools, skills, orchestration, permissions, persistence, and runtime state.
- Make editor context composable. A user can reference several selections, files, diagnostics, diffs, and images before sending one prompt.
- Render the transcript as native Neovim buffers while keeping runtime transcript state authoritative.
- Support reviewable agent edits without duplicating Phenix transport or schemas in Lua.
- Package the canonical client in this repository and publish a one-way standalone mirror for non-Nix Neovim plugin managers.

## Non-goals

- Do not expose `PluginHost`, component registration, service exports, kernel routing internals, or any other plugin ABI through `require("phenix")`.
- Do not make Neovim configuration a place to define runtime plugins.
- Do not duplicate ACP framing, Phenix extension schemas, observable semantics, session persistence, or model/provider logic in Lua.
- Do not make transcript text the authoritative session state.
- Do not require a Markdown renderer or an image-capable terminal for basic use.

## Package boundary

The production path is:

```text
phenix-nvim
  -> require("phenix")
  -> phenix-binding-lua
  -> phenix-client-acp
  -> ACP
  -> Phenix application API
  -> kernel/plugins
```

`phenix-binding-lua` is a client binding only. It exposes generated application operations and observable resources. The Neovim package may depend on it.

A future Lua plugin implementation package is separate:

```text
phenix-plugin-lua
  -> Phenix plugin ABI
  -> kernel
```

The Neovim package must not depend on `phenix-plugin-lua` and must not gain plugin-host capabilities through the client module.

## Canonical package and mirror

The canonical Neovim source lives in this repository, under a client package directory chosen by the implementation. The package is exported through the repository's Nix package set.

The existing standalone `phenix-nvim` repository becomes a one-way mirror for plugin managers that install directly from Git.

Mirror rules:

- conductor is the only implementation source of truth;
- mirror updates are deterministic exports from the canonical package;
- no implementation-only commits land in the mirror;
- the mirror records the conductor source revision it was generated from;
- CI verifies mirror parity;
- tags/releases originate from conductor and are propagated to the mirror;
- the mirror does not carry its own ACP or Phenix application schemas;
- the mirror does not commit native `phenix.so` artifacts.

For non-Nix installs, native binding distribution may use platform release assets or a separately installed `phenix-binding-lua`. The frontend source remains the same.

## Interaction model

### Compose buffer

The sidebar contains a persistent compose buffer. It is an editable multimodal prompt document. Referencing editor context inserts a prompt object at the current compose cursor. Referencing context does not send the prompt.

Example interaction:

1. Select `A` in an editor window.
2. Invoke `Reference`.
3. Type `question A` in the compose buffer.
4. Return to an editor window and select `B`.
5. Invoke `Reference`.
6. Type `question B`.
7. Send once.

The composed prompt is semantically:

```text
<selection A>
question A
<selection B>
question B
```

The frontend must preserve insertion order and the compose cursor between focus changes.

### Reference action

The core editor action means "reference this in the current prompt". User-facing mappings may call it `Ask`, but the internal operation is `Reference`.

Visual mode:

- snapshot the selected text and source metadata;
- open or focus the sidebar;
- insert the selection at the current compose cursor;
- place the compose cursor after the inserted reference.

Normal mode:

- reference the current file and cursor location;
- later implementations may offer the enclosing symbol when this is unambiguous;
- open or focus the sidebar and continue composition.

Other sources can use the same operation:

- diagnostics;
- quickfix/location-list entries;
- git diff or hunk;
- explicit files or buffers;
- images;
- future runtime-provided context resources.

### Prompt representation

Do not reduce the compose document to an unstructured string while editing. Model it as ordered segments, for example:

```text
Prompt
├─ Selection(foo.rs, 20:1..38:4)
├─ Text("Why does this borrow?")
├─ Selection(bar.rs, 71:1..84:12)
├─ Text("Is this related?")
├─ Image(screenshot.png)
└─ Text("What is wrong here?")
```

Selections are snapshots at reference time. Editing the source file later must not silently change already-referenced selection contents.

Explicit references such as a whole-file reference may remain live until send when the runtime contract supports that distinction.

References should remain visibly identifiable in the compose buffer. The user can delete or move a whole reference. The frontend may offer an explicit conversion from a reference to ordinary editable text.

### Explicit references

The compose buffer should also support explicit reference insertion and completion for users who prefer typed composition.

Initial reference kinds:

```text
@this
@selection
@file
@buffers
@diagnostics
@quickfix
@diff
```

File paths should support fuzzy completion. These spellings are UI syntax only. The frontend converts them to typed context references instead of concatenating file contents into ad-hoc prompt text.

### Send

`Send` commits the current compose document as one user turn.

Sending must:

- preserve segment ordering;
- encode text, references, and images through the application/client contract;
- create or reuse the active Phenix session without requiring a separate "new chat" step;
- clear or replace the local compose document only after the request is accepted;
- focus the transcript according to the user's current sidebar state without forcing focus away from the editor when the user has already returned to editing.

## Sidebar layout

Use separate transcript and compose buffers in one sidebar.

```text
┌──────── transcript ────────┐
│ user                       │
│ ...                        │
│                            │
│ assistant                  │
│ ...                        │
├────────────────────────────┤
│ compose                    │
│                            │
│ [foo.rs:20-40]             │
│ Why does this...           │
│ [bar.rs:70-80]             │
│ compared with this?        │
└────────────────────────────┘
```

The transcript is read-mostly. Users can navigate, search, yank, fold, and inspect it with normal Neovim behavior. The compose buffer is fully editable.

`Reference` always targets the compose buffer's remembered cursor even when another window is focused.

## Transcript model

Phenix runtime state is authoritative. The frontend projects structured transcript state into an ordinary Neovim buffer.

Conceptual transcript nodes:

```text
Turn
├─ UserMessage
│  ├─ Text
│  ├─ SelectionReference
│  ├─ FileReference
│  └─ Image
└─ AssistantMessage
   ├─ Text
   ├─ ToolCall
   ├─ ToolResult
   ├─ Permission
   ├─ Edit
   └─ Image
```

The frontend consumes generated observable handles from #503 rather than rebuilding session state from generic event polling.

### Text rendering

Assistant and user prose is real buffer text in Markdown form. This preserves native Neovim behavior:

- search;
- yank;
- motions;
- folds;
- Tree-sitter highlighting;
- optional Markdown rendering plugins.

Raw Markdown must remain usable. Integrations such as `markview.nvim` or `render-markdown.nvim` are optional presentation layers, not protocol dependencies.

### Structured transcript objects

Tool calls, tool results, references, edits, permissions, and images retain structured identity. Render them with buffer ranges plus extmarks, virtual text, virtual lines, conceal, folds, or dedicated actions as appropriate.

Completed tool calls should be compact by default and expandable. Running tool calls must show live status. Tool output updates must modify the existing structured block rather than append duplicate pseudo-messages.

Permissions must be actionable from the transcript/sidebar without requiring a separate terminal UI.

### Streaming

Assistant text streams into the current assistant message. The renderer updates the existing message range rather than rebuilding the whole transcript for every token.

The viewport follows output only when the user is already following the end. Manual scrolling disables automatic following until the user returns to the end or explicitly re-enables it.

Observable version/commit information remains available to the projection layer for gap detection and recovery. It need not be visible in normal UI.

## Image attachments

Images are first-class prompt and transcript objects.

Initial acquisition paths:

- choose an image file;
- paste/import an image from the system clipboard when a platform helper is available;
- accept an image reference supplied by another Neovim integration.

Image acquisition and image rendering are separate concerns. Phenix receives a typed image attachment regardless of preview support.

### Neovim 0.13 image preview

When `vim.ui.img` is available, render attached images inline in the compose/transcript area using the Neovim image API.

The image renderer must be isolated behind a small frontend abstraction because the API is new and terminal support varies.

Required behavior:

- anchor image placement to an extmark or equivalent stable buffer position;
- recompute placement when the sidebar scrolls, resizes, or the attachment moves;
- remove image placements when buffers/windows disappear;
- keep image bytes/metadata owned by the attachment model, not by the renderer;
- degrade to a textual attachment block when preview is unavailable.

Fallback example:

```text
[image: screenshot.png · 1440x900]
```

The absence of image preview must not prevent sending images to capable models.

## Agent edits

Agent file edits are runtime operations, not frontend-local text generation.

The Neovim client projects proposed/applied edits into native review UI. At minimum:

- list changed files;
- jump to next/previous change;
- show diff hunks;
- accept/reject at the supported runtime granularity;
- preserve meaningful Neovim undo behavior for frontend-applied buffer synchronization;
- detect conflicting user edits instead of silently overwriting them.

The frontend must not implement a second file-edit protocol on top of ACP.

## Session interaction

The client uses Phenix session persistence. It does not maintain an independent chat-history database.

MVP session behavior:

- first send creates or reuses an active session;
- toggle/focus sidebar;
- create a new session;
- choose an existing session;
- resume after Neovim restart;
- cancel the active generation;
- show connection/runtime errors in the sidebar.

Later UI may expose hierarchical Phenix sessions, child workers, rename, fork, routing state, cost/token metadata, and richer execution state when the application API provides them.

## Model and runtime state

Model, routing, execution, and other long-lived state are read through generated application observables. Commands are generated application operations.

The Neovim client sees only client-facing namespaces and schemas. It must not know which plugin owns a value, which service implements an operation, or how the kernel routes it.

A compact status component may show the active model/router and execution state. Model selection uses the application operation exposed by Phenix. Provider-specific configuration does not belong in the frontend.

## MVP

The first usable package is complete when it provides:

- packaged `require("phenix")` client binding;
- sidebar with separate transcript and compose buffers;
- `Reference` from visual selection and current location;
- multiple references inserted at the remembered compose cursor before one send;
- explicit file/buffer/context picker;
- typed selection/file/diagnostic/diff references;
- image file attachment;
- image sending through the application contract;
- `vim.ui.img` preview when available plus textual fallback;
- persistent session create/resume/select;
- streamed Markdown transcript;
- structured tool-call/result rendering;
- permission interaction;
- cancellation;
- runtime-originated edit/diff review;
- no handwritten ACP/Phenix protocol definitions in production Lua;
- no frontend-owned session persistence.

Suggested default actions:

```text
Reference       reference current editor object into compose buffer
Send            send current compose document
Toggle          toggle/focus sidebar
New             create new session
Sessions        choose/resume session
Context         choose a context object explicitly
Image           attach image at compose cursor
Cancel          cancel active generation
```

Mappings remain configurable and are not part of the protocol contract.

## Polished target

After the MVP, add capabilities only when they preserve the same interaction model:

- enclosing-symbol references from Tree-sitter;
- operator-pending `Reference`, explain, review, and edit actions with normal motions/text objects and dot-repeat;
- context chips/previews and attachment management;
- quickfix/location-list and diagnostic workflows;
- git hunk/diff references and review actions;
- clipboard image acquisition across supported platforms;
- richer image placement and resize behavior;
- hierarchical Phenix session tree and worker state;
- model/router picker and compact statusline component;
- reusable prompt actions such as explain, review, fix diagnostics, write tests, and review diff;
- multi-file edit review and conflict navigation;
- optional next-edit suggestions if the runtime later exposes them;
- additional multimodal attachment types supported by the application API.

These are frontend affordances over Phenix contracts. They must not create Neovim-specific runtime semantics.

## Required regressions

### Client contract

- `require("phenix")` loads the packaged native client binding;
- the frontend discovers and uses generated operations/observables without duplicated schemas;
- reconnect/resume restores transcript state from Phenix;
- observable version gaps trigger recovery rather than corrupting the transcript projection.

### Compose/reference behavior

- visual `Reference` snapshots the exact selected text and metadata;
- a second reference inserts at the remembered compose cursor without sending the first content;
- ordered references and text survive focus changes between editor and sidebar;
- deleting/moving references keeps prompt ordering correct;
- send emits one ordered multimodal user turn;
- source edits after selection do not mutate the captured selection snapshot.

### Transcript

- streaming extends one assistant message instead of creating duplicate messages;
- tool lifecycle updates mutate one structured tool block;
- manual scroll disables auto-follow;
- reconnect/replay renders the same transcript as live delivery;
- Markdown remains readable without an optional renderer.

### Images

- image attachments send successfully without preview support;
- `vim.ui.img` preview is used when available;
- preview placement follows sidebar movement/resize;
- closing/replacing buffers releases preview state;
- unsupported terminals/API versions show a stable textual fallback.

### Edits

- runtime edits appear as reviewable native diffs;
- accept/reject behavior matches runtime state;
- conflicting local edits are surfaced rather than overwritten;
- no frontend-specific wire protocol is used for edits.

### Packaging and mirror

- Nix package loads the frontend and native Lua binding together;
- standalone mirror contains exactly the exported frontend source plus mirror metadata;
- mirror parity CI fails on unmanaged divergence;
- standalone installation can locate a compatible native client binding without committing binaries to the source mirror.

## End-to-end acceptance

A release candidate must pass one real packaged flow:

```text
launch Neovim
-> load require("phenix")
-> connect to packaged Phenix runtime
-> reference selection A
-> type question A
-> reference selection B from another editor window
-> type question B
-> attach image and preview it when supported
-> send one composed prompt
-> create/reuse session
-> stream assistant text
-> render tool activity
-> receive and review a file edit
-> cancel or complete execution
-> close Neovim
-> reopen Neovim
-> resume the same durable session and transcript
```

The flow must contain zero handwritten ACP JSON-RPC or duplicated Phenix extension schemas in production frontend code.
