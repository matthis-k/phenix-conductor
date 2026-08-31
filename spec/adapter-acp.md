# ACP adapter

Status: specification only. Implement after repository cleanup.

## Terminology

Phenix uses these terms consistently:

- **Application**: user-facing software such as Neovim, a terminal CLI, or a browser UI.
- **Protocol**: the message contract between an application and Phenix, such as ACP.
- **Adapter**: the Phenix-side implementation of an external protocol.
- **Client SDK**: reusable application-side code for speaking a protocol.
- **Binding**: a language-native API over a client SDK.
- **Transport**: the mechanism that moves protocol bytes or messages.

ACP is a protocol. This package is its Phenix adapter.

## Goal

Provide ACP as the primary compatibility protocol for rich Phenix applications without exposing the internal `phenix-client` wire or conductor internals.

The first-party runtime package is `phenix-adapter-acp`, with plugin identity `phenix.adapter.acp` and package-set entry `phenixPlugins.${system}.adapter-acp`.

The adapter owns ACP translation only. It owns no Phenix session, transcript, routing, authority, credential, execution, or persistence state.

## Application boundary

Applications speak ACP to the adapter:

```text
application
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

Transport is independent. ACP stdio is valid directly. A separately running adapter or runtime may use `phenix-transport-socket` from #436 or another transport without changing ACP semantics.

## Packaging

Migrate the current ACP implementation into the adapter role:

- crate/package: `phenix-adapter-acp`;
- runtime id: `phenix.adapter.acp`;
- package-set entry: `phenixPlugins.${system}.adapter-acp`;
- executable entrypoint: `phenix-acp` where a standalone stdio process is useful;
- remove the old `phenixClients.${system}.acp` / `mkPhenixClient` public category if it has no remaining consumer;
- keep adapter selection, omission, replacement, authority, and configuration on the ordinary plugin path.

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

Advertise only implemented capabilities.

ACP protocol-version handling stays inside the adapter.

## Mapping rules

- ACP session identity maps to canonical Phenix `SessionId`.
- session creation creates exactly one canonical Phenix session.
- prompting enters the canonical Phenix execution path rather than invoking a model directly.
- cancellation targets the matching canonical execution.
- session list/resume reconstruct from runtime-owned state.
- adapter exit/disconnect never implies deletion of durable Phenix state.
- workspace or filesystem inputs narrow authority through existing canonical types; unsupported security-relevant input fails explicitly.
- client callbacks use the canonical frontend-service/authority path.

## Phenix ACP extensions

Use versioned, capability-negotiated ACP extensions for Phenix concepts that ACP does not represent.

Prefer standard ACP fields, methods, updates, config options, and `_meta` when they preserve the semantics. Add an extension only when the concept would otherwise be lost or distorted.

Expected extension families include:

- skill discovery and activation;
- orchestration/callable discovery and invocation;
- session tree or lineage data;
- Phenix routing-profile metadata that cannot map cleanly to ACP config options;
- execution-tree inspection;
- graph generation and invocation provenance;
- structured Phenix diagnostics.

Use a Phenix-owned namespace such as `_phenix/...`; never use application-specific names such as `_nvim/...`.

Extensions expose Phenix semantics, not internal transport envelopes. Remove `_phenix/client/envelope` as an ordinary application path once standard ACP plus domain extensions cover its consumers.

## Streaming

Translate canonical Phenix events into standard ACP updates where possible. Preserve ordering and stable identities needed for correlation.

The adapter must not synthesize state absent from Phenix or persist a second transcript to make ACP easier.

## Transport independence

The ACP adapter must not require #436.

- stdio works without a socket library;
- socket-backed deployment may reuse `phenix-transport-socket`;
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
- stdio behavior works without the socket transport library;
- socket-backed and stdio ACP have equivalent protocol semantics;
- diagnostics never corrupt protocol stdout.

## Completion

- [ ] `phenix-adapter-acp` is the ordinary first-party ACP adapter plugin;
- [ ] standard ACP covers every concept it represents cleanly;
- [ ] Phenix-only concepts use versioned capability-negotiated ACP extensions;
- [ ] applications do not require the internal `phenix-client` protocol;
- [ ] no parallel session, transcript, routing, authority, credential, execution, or persistence state exists;
- [ ] ACP semantics are independent of transport;
- [ ] obsolete `phenixClients.acp` packaging is removed when unused;
- [ ] exact-head Source, Rust, Product, and Maintenance validation passes.
