# Canonical Neovim client implementation handoff

status: partial

## Purpose

Mechanical contract for PR #504. `spec/nvim-client.md` owns product behavior. `spec/value-capability-sdk.md` owns the cross-language ABI. `spec/nvim-client-value-capability.md` resolves overlap.

#503 and #505 are merged. Their value/capability and client-tool contracts are fixed dependencies.

After this document, implementation should not need to choose another transport, state owner, callback model, session projection, review lifecycle, or module boundary.

## Fixed ownership

| Concern | Owner |
| --- | --- |
| application operations, events, callbacks, external data shapes | `phenix-application-interface` |
| ACP translation | `phenix-adapter-acp` |
| stdio process transport and bounded application/callback channels | `phenix-acp-stdio` |
| configured product runtime, application worker, observable projection | `phenix-harness` |
| generic SDK value/callable/observable ABI | `phenix-core` + native `phenix` binding |
| canonical session/execution truth | owning runtime plugins |
| pending edit-review lifecycle | `phenix-plugin-execution` |
| conflict-checked file mutation | workspace provider |
| Neovim client state and presentation | `clients/nvim` |
| optional editor integrations | #506 |

`clients/nvim` never owns canonical transcript, permission, review, execution, or session state.

## Existing application contracts to reuse

Do not add alternate session or prompt types. Use the current `phenix-application-interface` contracts directly:

```text
SessionInfo
SessionSnapshot
SessionUpdate
SessionChange
Message
Content::{Text, Image, Resource}
PromptInput / PromptResult
ExecutionInfo / ExecutionChange
PermissionRequest / PermissionResponse
ElicitationRequest / ElicitationResponse
ApplicationError
```

Use the existing operations:

```text
session-create
session-list
session-resume
session-rename
session-close
prompt
cancel
model-list / model-select
routing-list / routing-select
sdk-get
capability-invoke
```

Lua obtains operation functions through the descriptor-backed native application projection. Production frontend code contains no operation IDs or `_phenix/...` strings.

## Configured ACP application

### Package and executable

Add `rust/crates/phenix-harness/src/application.rs` for the configured application worker and `rust/crates/phenix-harness/src/bin/phenix-acp.rs` for the stdio entrypoint.

Expose `packages.<system>.phenix-acp`. The normal `phenix` product closure may include it, but `phenix-acp` remains an independently runnable executable.

`clients/nvim` defaults to the packaged `phenix-acp` path supplied by Nix. A user may override the command in frontend config.

### Process flow

```text
phenix-acp
  -> resolve normal Harness product configuration for cwd/workspace
  -> open canonical workspace Store Binding
  -> activate resolved Harness generation
  -> resolve SDK contributions
  -> create application projection ObservableStore
  -> create SharedCapabilityRegistry
  -> create one ACP client owner/generation
  -> construct SdkApplicationService
  -> run one configured application worker
  -> run phenix-acp-stdio server
```

`phenix-acp-stdio` stays transport-only. It does not construct product plugins, choose persistence, or interpret session semantics.

### Application worker

One worker owns mutable runtime application state. Requests arrive through the existing bounded `ChannelTransport`. The worker maps fixed application operations to the configured runtime and emits typed `ApplicationEvent` values.

Do not hold kernel/plugin locks while waiting for a client-owned capability callback.

Queue policy:

```text
application invocation queue: bounded, capacity 64
runtime -> client capability queue: bounded, capacity 64
session/application event queue: bounded, capacity 256
```

Queue exhaustion is an explicit typed failure. Never drop permission, elicitation, capability, or session state transitions silently.

A stdio process represents one connected client owner generation. Disconnect retires that generation through `SdkApplicationService::retire_client`, fails pending callbacks, removes client-tool admissions, stops the application worker, and exits.

## Session observable

### Contract

Add these passive application types in `phenix-application-interface/src/types/session.rs`:

```rust
pub struct SessionProjection {
    pub session: SessionInfo,
    pub through_sequence: u64,
    pub updates: Vec<SessionUpdate>,
}

pub struct SessionProjectionState {
    pub sessions: BTreeMap<String, SessionProjection>,
}
```

The map key is exactly `SessionInfo.session_id.as_str()`.

Do not invent a second transcript node schema on the server. `SessionUpdate` remains the portable event vocabulary. Neovim derives render nodes from it.

### Observable identity

The configured application worker registers one observable value:

```text
ValueId: phenix.application.sessions@1
schema: SessionProjectionState
root value: { sessions = {} }
```

`phenix-plugin-api::sdk_contribution()` publishes:

```text
SdkResourceId: sdk/phenix/session-state
binding path: phenix.sessions.state
ValueId: phenix.application.sessions@1
path: root
schema: SessionProjectionState
snapshot policy: copy-on-change
```

The Lua result is therefore:

```lua
phenix.sessions.state.get()
phenix.sessions.state.listen(opts, callback)
```

No frontend-private session event store is allowed.

### Reducer

The application worker owns the projection reducer.

`CreateSession` inserts an empty projection with `through_sequence = 0`.

`ResumeSession { after_sequence = nil }` replaces the session entry with the returned full `SessionSnapshot` converted directly to `SessionProjection`.

For each `SessionUpdate`:

1. find the projection by `session_id`;
2. require `update.sequence == projection.through_sequence + 1`;
3. append the exact update;
4. set `through_sequence = update.sequence`;
5. commit one observable transaction.

If the projection is absent or the sequence is not the expected next value, do not guess. Fetch a full `ResumeSession { after_sequence = nil }`, replace that one session projection, then continue.

A `Closed` update remains in the projection. The frontend may hide closed sessions from the default picker, but history remains runtime-owned.

### Client gap recovery

The frontend tracks both observable version and `SessionProjection.through_sequence`.

On an observable delivery gap, malformed diff, or per-session sequence gap:

1. stop applying diffs;
2. call `phenix.sessions.state.get()`;
3. rebuild the local projection from the full value;
4. replace the expected observable version;
5. resume listening.

Rendered buffer text is never used to recover identity.

## Prompt delivery

`Content` is the only wire representation for #504 prompt content.

Map compose segments exactly:

```text
text
  -> Content::Text { text }

location/reference with textual snapshot
  -> Content::Resource {
       uri,
       mime_type = Some("text/plain") when text is present,
       text = snapshot_or_none,
     }

image
  -> Content::Image { mime_type, data }
```

Do not encode editor coordinates into prompt prose. A reference URI identifies the source. Snapshot text is the captured immutable evidence.

`runtime.prompt` calls descriptor-backed `phenix.application.prompt@1` with one `PromptInput`. It does not concatenate text segments and does not use standard ACP text-only `session:prompt` after the final path lands.

The compose revision captured before request creation is cleared only after a successful application response and only when the current compose revision still equals the submitted revision.

## Permission and elicitation handlers

### Registration operation

Add one fixed application operation rather than transport-specific Lua callback registration:

```rust
pub struct InteractionHandlers {
    pub permission: Option<CallableRef>,
    pub elicitation: Option<CallableRef>,
}

pub struct SetInteractionHandlersInput {
    pub handlers: InteractionHandlers,
}
```

Operation:

```text
phenix.application.interaction-handlers-set@1
capability: phenix.application.capability.interaction@1
input: SetInteractionHandlersInput
output: Acknowledged
```

The server accepts only client-owned refs from the current connection generation whose callable schemas exactly match:

```text
PermissionRequest -> PermissionResponse
ElicitationRequest -> ElicitationResponse
```

Registration replaces the previous handler atomically. Disconnect clears both handlers.

The ACP adapter maps standard ACP permission/elicitation callbacks to these same handler slots. There is no second Lua-facing callback model.

### Frontend behavior

At connect time, `runtime.lua` lifts two Lua functions and registers them once. Those functions dispatch to #504's built-in UI. #506 may replace presentation behind the same frontend-local dispatcher.

Permission validation:

- built-in UI returns only a response permitted by the request semantics;
- Esc, window close, handler error, or invalid adapter output returns `Cancelled`;
- no UI result can widen runtime authority.

Elicitation validation:

- renderer produces a candidate `PhenixValue`;
- native paired-schema conversion validates the candidate against `ElicitationRequest.schema` before response;
- invalid candidate remains in the form with a field/error message;
- close/Esc returns `Cancelled`.

## Runtime edit review

### Owner

`phenix-plugin-execution` owns pending review lifecycle because the review gates an execution-produced mutation. The workspace provider remains the authority for file versions and application of the accepted patch.

Review state is durable enough to survive frontend restart. A frontend disconnect never accepts, rejects, or discards a pending review.

### Application types

Add to `phenix-application-interface`:

```rust
pub struct ReviewHunk {
    pub id: String,
    pub old_start: u64,
    pub old_count: u64,
    pub new_start: u64,
    pub new_count: u64,
    pub unified_diff: String,
}

pub struct ReviewFile {
    pub uri: String,
    pub expected_version: String,
    pub hunks: Vec<ReviewHunk>,
    pub conflict: Option<String>,
}

pub enum ReviewState {
    Pending,
    Accepted,
    Rejected,
    Conflicted { message: String },
}

pub struct ReviewRecord {
    pub id: String,
    pub revision: u64,
    pub session_id: SessionId,
    pub execution_id: String,
    pub files: Vec<ReviewFile>,
    pub state: ReviewState,
}

pub enum ReviewDecision {
    Accept,
    Reject,
}

pub struct ReviewDecisionInput {
    pub review_id: String,
    pub expected_revision: u64,
    pub decision: ReviewDecision,
}
```

Extend `SessionChange` with:

```text
Review { review: ReviewRecord }
```

Add application operation:

```text
phenix.application.review-decide@1
capability: phenix.application.capability.review@1
input: ReviewDecisionInput
output: ReviewRecord
```

### Decision semantics

`expected_revision` prevents stale UI actions.

Accept:

1. require `Pending` and matching revision;
2. ask the workspace owner to apply the exact prepared edit against every expected file version;
3. on success, persist `Accepted`, increment revision, emit `SessionChange::Review`;
4. on version conflict, persist `Conflicted`, increment revision, emit the updated review;
5. never report acceptance before workspace mutation succeeds.

Reject:

1. require `Pending` and matching revision;
2. discard the prepared mutation through the execution owner;
3. persist `Rejected`, increment revision, emit the update.

Repeated decisions on a terminal review return a typed stale/invalid-state error.

The Neovim reviewer presents only the runtime record. It never runs `git apply`, edits a buffer, or mutates files directly.

## Frontend runtime adapter

`clients/nvim/lua/phenix_nvim/runtime.lua` is the only production frontend module that imports `require("phenix")`.

Keep one state record:

```text
config
client
sdk
application
connection: disconnected | connecting | connected | failed
connection_error?
active_session_id?
session_state              # derived copy of SDK observable value
session_observable_version?
session_stop?              # remote stop callable
pending_requests[]
one poll timer
listeners[]
```

`active_session_id` is frontend selection, not canonical session state.

### Polling

One timer calls `client:poll()` and request polling on the Neovim main thread. No subscription, request, tool, or adapter creates its own timer.

Per tick:

1. process at most `poll_budget` native host events/callbacks;
2. process each currently pending request at most once;
3. emit coalesced frontend projection notifications after native dispatch;
4. return to Neovim.

Do not run Lua callbacks on background threads.

### Connection readiness

`connect()` succeeds only after all required #504 capabilities exist:

```text
sessions
session-list
session-resume
prompt
sdk
capabilities
permission
elicitation
review
```

and the SDK has callable resources at:

```text
phenix.sessions.state.get
phenix.sessions.state.listen
```

A missing required capability is a connection error with the missing capability/path named explicitly.

### Session operations

Use descriptor-backed application operations for create/list/resume/close/cancel/prompt. Do not retain the ACP session helper as a semantic fallback after #504 is complete.

After create or resume, set `active_session_id`. The session observable remains the source of transcript state.

## Transcript projection

`transcript/model.lua` reduces one selected `SessionProjection` into stable frontend nodes.

Node IDs are deterministic:

```text
message update       -> session:<sid>:sequence:<n>
streaming assistant  -> session:<sid>:execution:<eid>:assistant
execution state      -> session:<sid>:execution:<eid>:state
tool call/result     -> session:<sid>:execution:<eid>:tool:<call_id>
diagnostic           -> session:<sid>:sequence:<n>:diagnostic
review               -> session:<sid>:review:<review_id>
```

`TextDelta` appends to the one assistant streaming node for its execution. A later completed assistant `Message` may replace/finalize that node only when the reducer can associate it with the same execution from ordered session state. Otherwise it remains a separate immutable message node.

Tool result/failure mutates the existing call-id node. Unknown result call IDs are reducer invariant failures and trigger full session-state rebuild.

The renderer updates only changed extmark ranges. Full rerender is allowed only for initial render, observable/session gap recovery, explicit refresh, or reducer invariant repair.

Manual scroll disables follow-tail. Follow-tail resumes when the cursor reaches the final transcript line or the user invokes the explicit follow action.

## Compose model

`ComposeDocument` remains frontend-local and has exact invariants:

```text
items: id -> immutable non-text item
next_id: monotonic local integer
revision: monotonic local integer
```

Every inserted marker references exactly one item ID. Duplicate insertion of the same source creates a new item ID.

`Reference` captures source data before focus moves to the sidebar.

Coordinates are zero-based UTF-8 byte columns, matching Neovim API columns. Characterwise and linewise selections are supported. Blockwise selection returns a typed frontend error until exact rectangular snapshots are implemented.

Serialization walks the buffer/extmarks in byte order and emits `Content` items in that order. Missing item, duplicate marker identity, invalid extmark range, or unsupported item kind aborts Send without clearing compose state.

## Images

The attachment model stores:

```text
id
source_uri?
mime_type
bytes
optional display_name
```

Preview state is renderer-only.

File acquisition reads bytes once into the attachment snapshot. Sending works when no image renderer is available.

`vim.ui.img` is optional. Preview cleanup is tied to the owning extmark/window lifecycle. Closing/toggling the sidebar must not leak preview windows or mutate attachment bytes.

## Built-in permission and form UI

#504 ships dependency-free built-ins so #506 is optional.

Permission UI uses `vim.ui.select` when available and a minimal native floating fallback otherwise.

Basic elicitation supports exactly:

```text
string
bool
i64
u64
f64
optional of a supported scalar
record/table containing supported fields recursively
unit variant / enum of unit variants
list of supported scalar or unit-variant values
```

Unsupported callable/object/map/recursive-unbounded/compound-union shapes fail with `unsupported_schema`; they are never converted to a prose question.

## Sessions and status

`session.lua` is presentation-only. It lists `SessionInfo`, lets the user choose one, then calls runtime resume/select.

Status is a pure read of cached frontend projection:

```text
connection
active_session_id
active_execution_state?
model/routing label when already projected
last_error?
```

Statusline code performs no native client calls, request polling, filesystem I/O, or timer creation.

## Packaging and mirror

`modules/nvim-client.nix` owns `packages.phenix-nvim` and `packages.phenix-nvim-export`.

Add the packaged `phenix-acp` executable to the frontend test closure. `phenix-nvim` must start it exactly as production config does.

`phenix-nvim-export` contains only frontend source, license/metadata, and the exact conductor revision marker. It contains no native `.so` and no runtime binary.

The mirror check compares the exported file set and content hashes, excluding the revision marker value only where the mirror injects its own source revision metadata. Drift fails CI.

## Required deterministic tests

### Rust/application

- configured Harness serves the fixed application descriptor over real stdio ACP;
- create/list/resume/prompt/cancel use configured runtime services;
- `SessionProjectionState` initializes from full resume and appends ordered updates;
- unknown/gapped session update repairs through full resume before publication;
- one observable transaction corresponds to one accepted session update or one repair replacement;
- multimodal `PromptInput` round-trips Text + Resource + Image in order;
- handler registration rejects wrong contract, wrong owner, stale generation, and disconnected owner;
- disconnect retires handlers and pending client callbacks;
- review accept applies exact prepared edit before terminal state;
- review conflict never reports accepted;
- review reject never mutates workspace.

### Lua/frontend

- only `runtime.lua` imports `phenix`;
- connection fails when a required capability or `phenix.sessions.state` callable is absent;
- one poll timer services callbacks, observables, and requests;
- A -> question A -> B -> question B serializes in that order;
- image send succeeds with renderer disabled;
- in-flight send never clears newer edits;
- session observable full/diff updates produce stable transcript nodes;
- observable or session sequence gap performs full get/rebuild;
- tool lifecycle keeps one node per call id;
- manual scroll disables follow-tail and end-of-buffer restores it;
- permission close/error returns Cancelled;
- basic elicitation validates against the original schema;
- review action waits for runtime result and conflict remains visible;
- restart creates a new client generation, resumes the durable session, and reconstructs transcript from runtime state.

### Packaged acceptance

One realized check performs:

```text
packaged Neovim
-> packaged phenix.so + phenix_nvim
-> packaged phenix-acp
-> create durable session
-> Reference A + text + Reference B + text + image
-> Send one multimodal prompt
-> observe assistant streaming and tool lifecycle through phenix.sessions.state
-> answer one permission request
-> answer one basic elicitation
-> accept or reject one runtime edit review
-> exit Neovim
-> restart Neovim and phenix-acp in the same workspace
-> resume the same session
-> verify transcript/review terminal state from durable runtime data
```

No fixture-only code path may satisfy the final product check.

## Implementation order

1. Add application projection and review/interaction passive types.
2. Implement the configured Harness application worker and `phenix-acp` binary.
3. Publish `phenix.sessions.state` from `phenix-plugin-api` and wire its reducer.
4. Add interaction-handler registration and client-generation validation.
5. Add execution-owned durable review lifecycle and application mapping.
6. Replace `runtime.lua` ACP helper semantics with descriptor-backed application + SDK observable semantics.
7. Finish ordered multimodal Send.
8. Finish transcript subscription, reducer, gap recovery, and follow-tail.
9. Finish built-in permission/elicitation and runtime-backed review UI.
10. Finish image lifecycle and session/status presentation.
11. Add export parity and the realized end-to-end product check.
12. Reconcile PR body against exact HEAD, remove `.phenix/work/pr-504.md`, run exact-head Source, Rust, Product, Docs, Maintenance.

## Completion gate

#504 is complete only when every remaining PR item is implementation or validation work described above, the temporary working-state file is absent, and the realized acceptance flow passes on exact HEAD.
