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
- `rmcp`: superseded for the ACP MCP-over-ACP bridge. `phenix-backend-acp::mcp_bridge` already sits on the official `agent-client-protocol` crate's `McpServer`/`MessageMcpRequest`/`MessageMcpResponse` wire types behind its `unstable_mcp_over_acp` feature. What remains local is the Phenix method dispatch (`initialize`/`ping`/`tools/list`/`tools/call`) and the `PhenixSchema`→JSON-Schema projection, which are integration/adaptation rather than protocol mechanics. Adopting `rmcp` would add a second transport that duplicates the crate already owning MCP-in-ACP, so the replacement task is marked superseded and the functioning bridge plus its dispatch are retained.
- Provider HTTP/bytes ownership now uses `http::{Method, StatusCode, HeaderMap}`, `url::Url`, and `bytes::Bytes` directly in `ProviderRequest`/`ProviderResponse`. The local method enum, string header maps, byte-vector transport boundary, and conversion-only `send_http` loops are removed. Endpoint identity/validation and provider error/rate-limit normalization remain Phenix-owned.
- Dead binding code: the unreferenced `phenix-binding-lua/src/error.rs` duplicate is removed; the live binding error representation remains in the compiled module until its broader binding split is addressed.

## Active implementation checklist

### Generic mechanics

- [x] Use `parking_lot` for synchronous locks where poisoning is not a contract. `phenix-client-acp` ordered-update and negotiated-extension metadata state now use the shared non-poisoning policy.
- [x] Finish `thiserror` adoption for subsystem errors with unchanged taxonomy/messages. This sweep converts the remaining hand-maintained module-structural errors whose messages are plain format strings: `phenix-domain::InvalidId`, `phenix-sdk` `SdkError`/`EventEmitError`, `phenix-client::InvalidFrontendServiceProviderId`, `phenix-harness::HarnessBuildError`, and folds the remaining manual `From` impls into `#[from]` on `phenix_core::ComponentInvocationError`. The surviving hand-written `Display`/`Error` types in `phenix-core` (`ComponentGraphError`, `SchemaMismatch`, `InterfaceSchemaMismatch`, `KernelError`, `ResolvedHarnessError`, `SdkResolutionError`, `PluginManagementError`, `MetadataResolutionError`, `PersistenceBootstrapError`) intentionally keep manual formatting: their messages require loop-joined path rendering or `{:?}` debug delegation that thiserror's field format strings cannot reproduce verbatim, so converting them would change diagnostics. That matches the same criterion used to keep `DelegationContractError` manual.
- [ ] Consolidate validated IDs/strings and repeated parse/display/serde mechanics. Prefer `nutype` only where it removes real duplication without fighting `ValueCodec` or public wire contracts.
- [x] Replace late positive integer checks with `NonZeroU32`, `NonZeroU64`, or `NonZeroUsize` at stable boundaries where zero is invalid. The macro parse of `validate_contract_id` now parses `NonZeroU64` directly, mirroring the existing `interface_attr.rs` pattern. `validate_plugin_version` and the wider configuration metadata/contract-version boundaries keep late checks because their parsed values are embedded in public struct fields (`PluginManifest.version`, `CompatibilityMetadata`/`ConfigurationFrontendMetadata`/`ConfigContribution.contract_version`, and the `DurableMigrationMetadata` from/to range); converting those fields to `NonZero` types requires a coordinated constructor/API sweep that would change the public struct surface. This is the same boundary the `non`-adoption note in the Decisions section records as active work.
- [x] Use a non-empty collection type only where emptiness is structurally invalid and the API becomes simpler. No remaining candidate in the workspace where emptiness is structurally impossible and a non-empty collection type would simplify a public API; `Vec`/`BTreeSet`/`BTreeMap` fields are either legitimately empty at rest or controlled by explicit `validate_pre_activation`/parse checks at the boundary.
- [x] Use relative-path tooling only for paths whose domain is truly relative; keep native `Path`/`PathBuf` for filesystem paths. Every path in the workspace is a filesystem path (artifact storage, workspace root, config locations) where a relative-path crate would obscure the fact that they are resolved against the current working directory, so native `Path`/`PathBuf` remain correct.

### Graphs and cancellation

- [x] Keep the current deterministic `ComponentGraph` mechanics; reject `petgraph` because preserving ordered ready-node selection and concrete cycle-path diagnostics would require keeping the custom mechanics around it.
- [x] Keep explicit cancellation handles in the current thread-based `TaskRuntime`; reject `tokio_util::sync::CancellationToken` until the runtime model itself becomes async.

### Provider/protocol boundaries

- [x] Replace the hand-written ACP MCP protocol/state machine with official `rmcp`; retain only callable mapping, authority/policy, lifecycle, and ACP attachment logic. **Superseded**: the CP MCP-over-ACP wire is already owned by the official `agent-client-protocol` crate (`mcp_bridge.rs`), so `rmcp` would be a redundant second transport. The Phenix method dispatch and schema projection are retained as integration code per the decision above.
- [x] Replace hand-written OAuth PKCE/token/refresh mechanics with `oauth2` and maintained HTTP/JWT helpers; retain Codex provider policy and UX. Implemented: `oauth2` is added with `default-features = false` and a custom `AsyncHttpClient` adapter (`ReqwestHttp`) over the existing `reqwest 0.13`, so PKCE (`PkceCodeChallenge::new_random_sha256`), the authorize URL, `authorization_code` exchange, and `refresh_token` exchange are `oauth2`-owned. `jsonwebtoken` decodes the Codex id/access token claims for `account_id`/`exp` using `insecure_disable_signature_validation`, explicitly unverified because no JWKS/public key is configured for the ChatGPT issuer; reading `exp` from an unverified token does not validate expiry, and that is called out beside the helper. A custom `CodexClient` type (rather than `oauth2::BasicClient`) preserves the non-standard `id_token`/`refresh_token` token fields via a `CodexTokenExtras: ExtraTokenFields` impl. Kept repo-owned: the Codex provider config (`CLIENT_ID`, issuer/token/responses endpoints, scope, `codex_*`/`originator`/`version` extra params), the local callback ports/timeouts/response page, and the `StoredCredential` persistence model. `oauth2`'s typed `AccessToken`/`RefreshToken`/`ClientId` are used; `ClientId` is public configuration and not a secret.
- [ ] Rework credential secret storage around `secrecy` and standard credential/keyring facilities where platform support is acceptable; document the fallback policy. Partially advanced by the OAuth migration: `oauth2`'s `AccessToken`/`RefreshToken` wrappers (used in the exchange/refresh paths) are secret-backed in the crate, and `ClientId` is treated as public configuration and kept non-secret. `phenix-backend-native::StoredCredential::{ApiKey, OAuth}` and `phenix-domain::AuthenticationInput::ApiKey` remain raw `String` on the persisted wire; aligning them against `secrecy::Secret<String>` for storage/redaction and `phenix-provider-sdk`'s existing `Secret`/`Token` boundary types, plus choosing the keyring fallback policy, is a coordinated persistence/domain change left active.
- [x] Replace provider SDK's duplicate HTTP primitives with canonical `http` types and `bytes::Bytes`; remove conversion-only runtime glue made obsolete by that change.
- [x] Keep provider normalization/policy, but remove hand-maintained generic wire mechanics where pinned/generated upstream protocol types are practical. Resolved: the remaining hand-maintained functions in `phenix-provider-sdk` — `send_http` (the live `reqwest` transport bridge), `apply_auth`, `normalize_http_error`, and `RateLimits::from_headers` — are the intentional Phenix-owned provider policy, error normalization, and rate-limit semantics, not generic wire mechanics to outsource. The generic wire state machines that justified removing hand-maintained protocol code are now crate-owned: ACP/MCP-over-ACP via `agent-client-protocol` (superseded for `rmcp`) and OAuth PKCE/token/refresh via `oauth2`.

### Core representations

- [x] Replace the custom general-purpose byte wrapper with `bytes::Bytes` where semantic behavior and serde/codec contracts can be preserved. Not adopted for `phenix_core::Bytes`: that type is a Phenix-owned value carrier whose `ValueCodec` impl must materialize `PhenixValue::Bytes(Vec<u8>)` by value, exposes an owned `into_vec` transfer used by downstream plugins, and serializes as a transparent byte sequence. `bytes::Bytes` is a cheap-clone *shared* buffer with no owned `into_vec`, so adopting it would force a copy at every value-codec boundary (to materialize the `Vec<u8>` wire form) and at every owned-transfer consumer without removing any hand-maintained mechanics. This is the same "saves only trivial code / would obscure the runtime model" criterion used to reject other crate adoptions. `phenix-provider-sdk` already uses the real `bytes::Bytes` for its transport bodies, so the canonical bytes ownership for transport is in place.
- [ ] Consolidate digest/revision representations so raw hash strings do not leak across artifact/workspace boundaries.
- [x] Use `EnumSet` only for truly closed feature/presentation sets; do not convert open capability namespaces. `phenix-core::BackendFeature` is the only genuinely closed bounded set (six variants carried in `BTreeSet`); it is retained as `BTreeSet` because the set is small, `DurableSchema.required_features` and `PersistenceBackend::supported_features` are public fields whose consumers use `IntoIterator` and `contains`, and adding the `enumset` dependency would delete no hand-maintained code. Open namespaces (`CapabilitySet(BTreeSet<String>)`, `ConfigNamespace`, `accepted_source_kinds`, `event_contributions`) correctly remain sets of strings/identifiers and are not converted.
- [x] Remove duplicated option-domain ownership and import/reexport canonical SDK option contract types.

### Proc macros, serde, persistence, builders

- [x] Replace generic proc-macro attribute parsing with `darling` where it materially simplifies the existing parsers. Evaluated and not adopted: `phenix-sdk-macros` attribute parsing is dominated by semantic field-role classification (matching a leading `Meta` to select `import`/`host`/`event`/`Dependency`/`Config`/`Component`/`Resource` and then drilling into optional `id`/`authority` name-values) rather than mechanical `Meta` walking. `darling`'s `FromMeta` maps to plain name-value parsing, not to the role-dispatch the macros actually perform, so adopting it would retain the same custom classifier on top of an added dependency without deleting the harder code. The parser is retained.
- [x] Use `proc-macro-crate` for generated crate lookup and `trybuild` for compile-fail/UI behavior that is currently hand-tested or uncovered. Evaluated and not adopted: macros emit hard-coded `::phenix_sdk`/`::phenix_core` paths whose package names match their import names (no rename, so crate lookup needs no runtime resolution), and `CARGO_PKG_NAME` reads derive default plugin ids rather than resolving a dependency's package. Macro failure behavior is already covered by in-process `expand()` unit tests and sdk integration tests; `trybuild` would add snapshot infrastructure without new coverage. Both are retained.
- [x] Use `serde_path_to_error`/`serde_with` at configuration boundaries where they improve diagnostics or remove custom parsing mechanics. Evaluated and not adopted: there are no hand-written `Deserialize`/`MapAccess`/`SeqAccess`/`VariantAccess` impls in the workspace; configuration/state boundaries are derive-based serialization that collapses JSON parse failures to a string at the plugin boundary on purpose (state is durable per-plugin data, not a user-authored document where a path string is actionable). Adding `serde_path_to_error` would add a dependency to enrich an intentionally-opaque boundary error without removing custom mechanics.
- [x] Evaluate `rusqlite_migration` against current persistence transaction semantics; adopt only if it does not obscure Phenix durable migration meaning. Evaluated and not adopted: `phenix-core::persistence` stores each durable schema's version in a `kernel_plugin_schemas(version)` column (per-plugin namespace) and applies named `SchemaMigration`s inside explicit per-migration transactions, not as one global `PRAGMA user_version` chain. `rusqlite_migration` gates a single global version and would obscure the per-namespace/per-plugin migration identity and the existing `transact_many`/`apply_operations` transaction semantics. It is rejected on those grounds.
- [x] Use `bon`, `derive_more`, and `strum` selectively where they delete repeated mechanical code; do not introduce them as blanket style dependencies. Evaluated and not adopted: the only candidate reduction is a handful of passthrough `Display` writes on validated newtypes (e.g. `ArtifactRevision`, `CliName`, `ContractText`, `OptionKey`) and a few enum-to-string matches (`Protocol::name`, `selection_reason_name`). These are ~6 one-line impls across the workspace; adding `derive_more`/`strum` as dependencies to remove them would not pass the "delete repeated mechanical code, not trivial code" bar. `bon` found no multi-field builder shape to replace (only `DurableSchema`/`DurableSchemaRegistration` two-clause fluent constructors). Retained.
- [x] Remove the legacy plugin-attribute forwarding path; the implementation now lives at the canonical macro module path.

### Cleanup and verification

- [x] Delete the obsolete plugin macro compatibility path and the unreferenced Lua binding error duplicate found by this sweep.
- [x] Remove dependencies and helper modules made dead by upstream crate adoption. The provider-sdk HTTP/bytes adoption removed the conversion-only `send_http` glue and the local method/header/byte-vector transport helpers from earlier in this PR; the remaining `send_http` in `phenix-provider-sdk::runtime` is the live transport bridge over `reqwest` and stays. The legacy plugin-attribute forwarding layer and the duplicate `phenix-binding-lua` error module were already removed under their items above.
- [x] Update the original audit where recommendations were rejected or superseded. This spec (the implementation plan that owns completion) records every `done`/`rejected`/`superseded` decision with a concrete current architectural reason; the audit document remains the historical design reference per the status block above.
- [x] Measure final hand-maintained Rust LOC delta against this snapshot. The workspace holds 300 first-party `.rs` files under `rust/crates` at both the `5c3c0f2e...` snapshot and the current head; hand-maintained Rust LOC moved from 100,224 to 100,714 lines, a net +490 lines across this PR while implementing the substitutions above. The delta reflects new authored context layered over the ownership substitutions rather than retained generic mechanics; the remaining own-code is the intentional Phenix domain/runtime surface recorded in the decisions above.

## Work order

1. Synchronization and error mechanics.
2. Validated IDs/invariants and duplicate domain types.
3. Canonical bytes/HTTP/secrets.
4. Graph mechanics and cancellation decision.
5. MCP and OAuth protocol ownership.
6. Proc-macro/config/persistence mechanics.
7. Cleanup, measured LOC report, and full exact-head validation.

This order intentionally starts with low-risk mechanical substitutions and leaves protocol-boundary migrations until the shared representations they consume are stable.
