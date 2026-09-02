# ACP adapter

status: specification-only

Canonical application-integration terminology is defined by #442. Implementation follows repository cleanup.

## Goal

Provide ACP as the primary compatibility protocol for rich Phenix applications without exposing the internal `phenix-client` wire or conductor internals.

The first-party runtime package is `phenix-adapter-acp`, with plugin identity `phenix.adapter.acp` and package-set entry `phenixPlugins.${system}.adapter-acp`.

The adapter owns ACP translation only. It owns no Phenix session, transcript, routing, authority, credential, execution, persistence, process, or transport state.

## Application boundary

```text
Application
  Neovim / CLI / other UI
        |
        | ACP + negotiated Phenix ACP extensions
        v
phenix-adapter-acp
        |
        | canonical internal Phenix calls/events
        v
Phenix runtime
```

Applications must not need the internal `phenix-client` protocol.

## Packaging

Migrate the current ACP implementation into the adapter role:

- crate/package: `phenix-adapter-acp`;
- runtime id: `phenix.adapter.acp`;
- package-set entry: `phenixPlugins.${system}.adapter-acp`;
- remove the old `phenixClients.${system}.acp` / `mkPhenixClient` public category if it has no remaining consumer;
- keep adapter selection, omission, replacement, authority, and configuration on the ordinary plugin path.

The adapter package does not own the stdio executable. #443 defines `phenix-acp-stdio`, which composes this adapter with stdin/stdout and owns `bin/phenix-acp`.

Adapter is a runtime role. Plugin remains the generic packaging/lifecycle mechanism.

## Standard ACP first

Implement the pinned ACP version faithfully before adding Phenix extensions.

Baseline includes the standard methods that map to canonical Phenix behavior, including:

- initialization and capability negotiation;
- authentication where ACP semantics match;
- session creation, listing, resume/load, close, and prompt;
- cancellation;
- session updates;
- tool-call and progress updates;
- permissions and client-provided capabilities;
- configuration/mode options where semantics match.

Advertise only implemented capabilities. ACP protocol-version handling stays inside the adapter.

## Mapping rules

- ACP session identity maps to canonical Phenix `SessionId`.
- Session creation creates exactly one canonical Phenix session.
- Prompting enters the canonical Phenix execution path rather than invoking a model directly.
- Cancellation targets the matching canonical execution.
- Session list/resume reconstruct from runtime-owned state.
- Adapter disconnect never implies deletion of durable Phenix state.
- Workspace or filesystem inputs narrow authority through existing canonical types; unsupported security-relevant input fails explicitly.
- Client callbacks use the canonical frontend-service and authority path.

## Phenix ACP extensions

Use versioned, capability-negotiated ACP extensions for Phenix concepts that ACP does not represent.

Prefer standard ACP fields, methods, updates, config options, and `_meta` when they preserve the semantics. Add an extension only when the concept would otherwise be lost or distorted.

Expected extension families include:

- skill discovery and activation;
- orchestration/callable discovery and invocation;
- session tree or lineage data;
- routing-profile metadata that cannot map cleanly to ACP config options;
- execution-tree inspection;
- graph generation and invocation provenance;
- structured Phenix diagnostics.

Use a Phenix-owned namespace such as `_phenix/...`; never use application-specific names such as `_nvim/...`.

Extensions expose Phenix semantics, not internal transport envelopes. Remove `_phenix/client/envelope` as an ordinary application path once standard ACP plus domain extensions cover its consumers.

## Streaming

Translate canonical Phenix events into standard ACP updates where possible. Preserve ordering and stable identities needed for correlation.

The adapter must not synthesize state absent from Phenix or persist a second transcript to make ACP easier.

## Transport independence

ACP semantics are transport-independent.

- #443 provides the standard spawnable stdio composition;
- stdio requires no socket library;
- socket-backed deployment may reuse `phenix-transport-socket` from #436;
- future transports can carry the same ACP protocol;
- transport choice must not change sessions, capabilities, extensions, authority, or event semantics.

## Regression coverage

- `phenix.adapter.acp` is selectable, omittable, and replaceable through ordinary plugin composition;
- ACP `initialize` advertises only real capabilities and negotiated Phenix extensions;
- standard session create/list/resume/prompt/cancel operations map to canonical Phenix state;
- standard ACP operations do not require an internal envelope extension;
- Phenix-only concepts use namespaced negotiated extensions;
- an application can use skills, orchestration, session lineage, and routing metadata without importing `phenix-client`;
- frontend callbacks preserve canonical authority checks;
- adapter disconnect does not delete durable sessions;
- stdio and other transports expose equivalent ACP semantics;
- transport/process concerns stay outside the adapter translation layer.

## Completion

- [ ] `phenix-adapter-acp` is the ordinary first-party ACP adapter plugin;
- [ ] standard ACP covers every concept it represents cleanly;
- [ ] Phenix-only concepts use versioned capability-negotiated ACP extensions;
- [ ] applications do not require the internal `phenix-client` protocol;
- [ ] no parallel session, transcript, routing, authority, credential, execution, or persistence state exists;
- [ ] ACP semantics are independent of transport;
- [ ] stdio process ownership lives in #443 rather than this adapter package;
- [ ] obsolete `phenixClients.acp` packaging is removed when unused;
- [ ] exact-head Source, Rust, Product, and Maintenance validation passes.
