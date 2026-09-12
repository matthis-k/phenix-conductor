---
status: partial
source: rust-library-ownership-audit
snapshot: 5c3c0f2ee187c6dd38d569d2467055b8afcd067a
---

# Rust library ownership implementation

This is the active implementation plan for `spec/rust-library-ownership-audit.md`.

The audit is a design reference. This file owns completion. A recommendation is complete only when it is implemented, explicitly rejected with a current architectural reason, or superseded by a better existing mechanism.

## Goal

Phenix owns domain semantics. Maintained Rust libraries own generic protocol, parsing, synchronization, graph, HTTP, validation, error, and code-generation mechanics when they fit without weakening Phenix contracts.

Do not add compatibility fallbacks or parallel APIs. Preserve wire formats, validation, deterministic ordering, redaction, authority, lifecycle, generation identity, transaction semantics, and public error meaning.

## Definition of done

- [ ] Every item below is `done`, `rejected`, or `superseded` with a concrete reason.
- [ ] No local generic mechanism remains when an adopted upstream mechanism fully replaces it.
- [ ] No crate is added only to save trivial code or to hide Phenix semantics.
- [ ] Public API and wire changes are intentional and reflected in deterministic tests.
- [ ] Exact-head Source, Rust, Clippy, Product, Integration, Docs, and Maintenance are green.
- [ ] The final diff records measured hand-maintained Rust LOC change and remaining intentional Phenix-owned mechanics.

## Verified upstream baseline

Checked against current upstream documentation on 2026-09-11. Versions are evidence for this implementation pass, not a promise to pin every crate at the listed patch release.

| Mechanic | Current upstream | Decision target |
| --- | --- | --- |
| MCP | `rmcp` 3.2.x, official Rust SDK, Tokio/async server model, current MCP 2026-07-28 with older-version compatibility | adopt protocol ownership without creating a second Phenix transport/runtime |
| OAuth2 | `oauth2` 5.0.x | replace PKCE, authorization-code exchange, refresh, and token primitives while retaining provider policy/callback UX |
| HTTP types | `http` 1.5.x | canonical method/header/status/URI types at provider boundaries |
| bytes | `bytes` 1.12.x with optional serde | immutable shared payload storage while preserving Phenix wire/value codecs |
| graph algorithms | `petgraph` 0.8.x | DAG/cycle/topological mechanics only; Phenix keeps graph semantics |
| validated newtypes | `nutype` 0.7.x | adopt only where generated validation/serde removes more code than it adds |
| secrets | `secrecy` 0.10.x | explicit secret exposure and redaction where it composes with persisted credential contracts |

## Completed before this implementation PR

- [x] `parking_lot` for capability and prepared-mutation synchronous registry state. (#512)
- [x] `thiserror` for `CapabilityError` while preserving variants/messages. (#512)
- [x] `NonZeroUsize` for event delivery capacity. (#513)
- [x] Remove one-variant `SourceIdentityRule`, `SourceRevisionRule`, and `StableMaterializationRule` placeholder APIs. (#513)
- [x] Official ACP SDK is already the ACP protocol owner (`agent-client-protocol`).

## Decisions and work in this PR

- `thiserror`: adopted where it removes complete `Display`/`Error`/`From` boilerplate without changing taxonomy or messages. Conductor build/serve errors, backend errors, client-tool errors, debug serialization errors, provider credential-store/runtime errors, application-interface errors, context-catalog errors, language-service errors, event delivery errors, observable errors, memory-plugin errors, persistence-provider errors, artifact-revision parse errors, plugin-build plan errors, plugin-build execution/store errors, and SDK provider errors are migrated in the current sweep. `DelegationContractError` intentionally keeps its manual implementation because its public `UnknownRelatedComponent { source, target }` field would be interpreted by `thiserror` as an error source; renaming that public field solely for the derive would change the API.
- `parking_lot`: capability/prepared-mutation registries, ACP MCP bridge and backend session/cancellation state, `EventBus` receipt/subscription/causality state, `TaskRuntime` ownership/call registries, `ObservableStore` state, native-backend session model/tool/history/active state, core runtime plugin-instance/persistence/invocation-trace/provenance state, and `phenix-client-acp` ordered-update and negotiated-extension metadata state use the same non-poisoning synchronous lock policy. Domain cancellation, delivery, transaction, observation, lifecycle, and ownership semantics are unchanged. The persistence-bootstrap recording mutex remains intentionally test-local.
- `nutype`: not adopted as a blanket identifier replacement. Core's existing identifier machinery also owns Phenix-specific schema/value-codec behavior, and replacing it would still require a Phenix wrapper while changing generated error surfaces. The duplicate domain `SessionId` has been removed in favor of core's canonical `SessionId`; this intentionally tightens domain session IDs to the existing core character policy and is covered by a wire-deserialization regression test. Other duplicated validated IDs remain active consolidation work before reconsidering a derive crate.
- `NonZero` integer types: positive interface identifiers and SDK interface/resource schema/migration parsers now parse directly into `NonZeroU64`/`NonZeroU32` rather than accepting zero and rejecting it in a second step. Public wire values and diagnostics are unchanged. Wider configuration metadata/contract-version boundaries remain active work because converting their public struct fields requires a coordinated constructor/API sweep.
- Option contracts: `phenix-plugin-options` now imports and reexports the canonical SDK option IDs, scopes, values, definitions, commands, responses, and `OptionsInterface`. The plugin owns only persistence, precedence, validation policy, and resolution state. This removes the second serde/value-codec/type implementation while keeping existing plugin import paths usable.
- Plugin macro path: the old `plugin_attr.rs -> plugin_attr_legacy.rs` forwarding layer is removed. The existing implementation is now the canonical `plugin_attr.rs`; there is no second compatibility entrypoint to maintain.
- `tokio_util::sync::CancellationToken`: rejected for the current core `TaskRuntime`. The runtime is deliberately thread-based and its cancellation handles are coupled to task ownership, authority attenuation, graph generation, and kernel cancellation events. A Tokio-oriented primitive would not replace those semantics and would add another runtime-adjacent mechanism. Revisit only if `TaskRuntime` itself moves to an async execution model.
- `petgraph`: rejected for the current `ComponentGraph` implementation. The graph contract requires lexicographically deterministic ready-node selection and diagnostics containing the concrete cycle path. `petgraph::algo::toposort` supplies neither contract directly; retaining the existing `BTreeMap`/`BTreeSet` implementation is smaller than adding a second ordering/cycle layer around `petgraph`. Revisit if the graph grows beyond these mechanics.
- `rmcp`: remains an active migration. Current upstream is async/Tokio and newer than the bridge's hand-written MCP revision, so adoption must replace protocol parsing/dispatch deliberately rather than merely adding a dependency beside the existing state machine.
- Provider HTTP/bytes ownership now uses `http::{Method, StatusCode, HeaderMap}`, `url::Url`, and `bytes::Bytes` directly in `ProviderRequest`/`ProviderResponse`. The local method enum, string header maps, byte-vector transport boundary, and conversion-only `send_http` loops are removed. Endpoint identity/validation and provider error/rate-limit normalization remain Phenix-owned.
- Dead binding code: the unreferenced `phenix-binding-lua/src/error.rs` duplicate is removed; the live binding error representation remains in the compiled module until its broader binding split is addressed.

## Active implementation checklist

### Generic mechanics

- [x] Use `parking_lot` for synchronous locks where poisoning is not a contract. `phenix-client-acp` ordered-update and negotiated-extension metadata state now use the shared non-poisoning policy.
- [ ] Finish `thiserror` adoption for subsystem errors with unchanged taxonomy/messages.
- [ ] Consolidate validated IDs/strings and repeated parse/display/serde mechanics. Prefer `nutype` only where it removes real duplication without fighting `ValueCodec` or public wire contracts.
- [ ] Replace late positive integer checks with `NonZeroU32`, `NonZeroU64`, or `NonZeroUsize` at stable boundaries where zero is invalid.
- [ ] Use a non-empty collection type only where emptiness is structurally invalid and the API becomes simpler.
- [ ] Use relative-path tooling only for paths whose domain is truly relative; keep native `Path`/`PathBuf` for filesystem paths.

### Graphs and cancellation

- [x] Keep the current deterministic `ComponentGraph` mechanics; reject `petgraph` because preserving ordered ready-node selection and concrete cycle-path diagnostics would require keeping the custom mechanics around it.
- [x] Keep explicit cancellation handles in the current thread-based `TaskRuntime`; reject `tokio_util::sync::CancellationToken` until the runtime model itself becomes async.

### Provider/protocol boundaries

- [ ] Replace the hand-written ACP MCP protocol/state machine with official `rmcp`; retain only callable mapping, authority/policy, lifecycle, and ACP attachment logic.
- [ ] Replace hand-written OAuth PKCE/token/refresh mechanics with `oauth2` and maintained HTTP/JWT helpers; retain Codex provider policy and UX.
- [ ] Rework credential secret storage around `secrecy` and standard credential/keyring facilities where platform support is acceptable; document the fallback policy.
- [x] Replace provider SDK's duplicate HTTP primitives with canonical `http` types and `bytes::Bytes`; remove conversion-only runtime glue made obsolete by that change.
- [ ] Keep provider normalization/policy, but remove hand-maintained generic wire mechanics where pinned/generated upstream protocol types are practical.

### Core representations

- [ ] Replace the custom general-purpose byte wrapper with `bytes::Bytes` where semantic behavior and serde/codec contracts can be preserved.
- [ ] Consolidate digest/revision representations so raw hash strings do not leak across artifact/workspace boundaries.
- [ ] Use `EnumSet` only for truly closed feature/presentation sets; do not convert open capability namespaces.
- [x] Remove duplicated option-domain ownership and import/reexport canonical SDK option contract types.

### Proc macros, serde, persistence, builders

- [ ] Replace generic proc-macro attribute parsing with `darling` where it materially simplifies the existing parsers.
- [ ] Use `proc-macro-crate` for generated crate lookup and `trybuild` for compile-fail/UI behavior that is currently hand-tested or uncovered.
- [ ] Use `serde_path_to_error`/`serde_with` at configuration boundaries where they improve diagnostics or remove custom parsing mechanics.
- [ ] Evaluate `rusqlite_migration` against current persistence transaction semantics; adopt only if it does not obscure Phenix durable migration meaning.
- [ ] Use `bon`, `derive_more`, and `strum` selectively where they delete repeated mechanical code; do not introduce them as blanket style dependencies.
- [x] Remove the legacy plugin-attribute forwarding path; the implementation now lives at the canonical macro module path.

### Cleanup and verification

- [x] Delete the obsolete plugin macro compatibility path and the unreferenced Lua binding error duplicate found by this sweep.
- [ ] Remove dependencies and helper modules made dead by upstream crate adoption.
- [ ] Update the original audit where recommendations were rejected or superseded.
- [ ] Measure final hand-maintained Rust LOC delta against this snapshot.

## Work order

1. Synchronization and error mechanics.
2. Validated IDs/invariants and duplicate domain types.
3. Canonical bytes/HTTP/secrets.
4. Graph mechanics and cancellation decision.
5. MCP and OAuth protocol ownership.
6. Proc-macro/config/persistence mechanics.
7. Cleanup, measured LOC report, and full exact-head validation.

This order intentionally starts with low-risk mechanical substitutions and leaves protocol-boundary migrations until the shared representations they consume are stable.
