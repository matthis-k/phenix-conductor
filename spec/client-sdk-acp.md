# ACP client SDK

status: specification-only

Implementation follows stabilization of the ACP adapter contract from #437 and the fixed application interface in `application-interface.md`.

## Goal

Provide one reusable Rust **Client SDK** for Phenix applications that speak ACP plus negotiated Phenix ACP extensions.

The first-party crate/package is `phenix-client-acp`.

It is application-side library code. It has no runtime plugin identity, owns no adapter, and does not appear in `phenixPlugins`.

## Boundary

```text
application
   |
   v
phenix-client-acp
   |
   | generated application API
   | handwritten ACP connection behavior
   v
ACP + negotiated `_phenix/...` extensions
   |
   v
phenix-adapter-acp
   |
   v
Phenix runtime
```

Applications should depend on this SDK or a language binding over it rather than reimplement ACP framing, capability negotiation, Phenix extension schemas, session reconstruction, or request correlation.

The SDK must not expose the internal `phenix-client` runtime wire as its public API.

## Generated application API

The fixed application descriptor owns public operation, event, callback, capability, type, and error identities.

Generate the repetitive Rust API from that descriptor. Generated code includes:

- application request and result types;
- event and callback payload types;
- capability identifiers and feature checks;
- typed operation wrappers;
- Phenix extension identifiers and structural payload codecs;
- deterministic interface-version metadata.

The generator must not inspect runtime Plugin implementation crates. Regenerating the client from the same descriptor must produce the same source.

Handwritten SDK code owns ACP-specific behavior that the application descriptor cannot define:

- ACP initialization and protocol-version negotiation;
- request ids and response correlation;
- standard ACP method mapping;
- transport lifecycle;
- reconnect and resume behavior;
- update-stream ordering and backpressure;
- unknown future ACP extensions.

This split keeps one semantic application API while preserving a faithful ACP implementation.

## API shape

Expose typed async Rust APIs around application concepts:

- initialize and negotiated capabilities;
- authentication;
- session create/list/resume/close;
- prompt and cancellation;
- ordered session/update streams;
- tool/progress/permission/elicitation events;
- model/config options;
- typed Phenix extension clients for skills, orchestration/callables, session lineage, routing metadata, execution inspection, provenance, and diagnostics when advertised.

Prefer typed domain values over raw JSON-RPC payloads. Keep an escape hatch for unknown negotiated extensions only when required for forward compatibility.

## Capability negotiation

The SDK records the capabilities returned by ACP initialization and exposes Phenix extension availability as typed feature checks generated from the application descriptor.

Calling an unavailable operation returns a typed unsupported-capability error before sending an invalid request where possible.

Do not assume every `phenix-adapter-acp` deployment exposes every Phenix extension.

## Transport

Protocol and transport stay separate.

The SDK should support a narrow transport trait or connection abstraction suitable for:

- child-process stdio;
- an already-open stream;
- a socket-backed stream using `phenix-transport-socket` from #436;
- future transports without changing the high-level API.

Do not make Unix sockets mandatory and do not duplicate socket mechanics inside this crate.

## State

Keep only client-side protocol state required to correlate requests, track negotiated capabilities, and project active application handles.

The Phenix runtime remains authoritative for durable sessions, executions, routing, authentication state, tools, and transcripts.

Reconnect/resume reconstructs from ACP/Phenix state rather than a second local database.

## Error model

The application descriptor owns stable application error variants. The ACP client maps transport and protocol failures around them.

Use typed errors for transport failure, protocol failure, unsupported capability, server rejection, cancellation, malformed extension payloads, and generated value conversion failures.

Preserve server error details needed by applications without exposing internal runtime implementation types.

## Binding generation

Language bindings should consume the same fixed application descriptor instead of deriving a second API from the handwritten Rust client implementation.

A binding generator may reuse generated Rust types and a shared native client core, but operation ids, schemas, events, capabilities, and errors remain descriptor-owned.

The Rust generator is the first proof that the descriptor is sufficient for client generation. Later Lua, Python, JavaScript, or other targets must agree with the same descriptor snapshot.

## Consumers

Expected consumers include:

- `phenix-cli`;
- the Lua binding used by `phenix-nvim`;
- tests and example applications;
- future Python/JS/native bindings where a Rust core is appropriate.

No consumer should need to duplicate ACP extension definitions.

## Regression coverage

- generated Rust code is deterministic for a fixed application descriptor;
- the generated API records the exact application interface id and version;
- the SDK initializes against `phenix-adapter-acp` and records negotiated capabilities;
- unavailable operations fail as unsupported without corrupting connection state;
- create/list/resume/prompt/cancel use standard ACP where available;
- Phenix extension clients use ids and schemas from the fixed application descriptor;
- ordered updates remain ordered through the SDK stream;
- reconnect/resume reconstructs durable runtime state without a local transcript store;
- stdio and socket-backed streams expose equivalent protocol semantics;
- no public SDK type requires the internal `phenix-client` wire or a runtime Plugin implementation crate.

## Completion

- [ ] `phenix-client-acp` is an independently buildable application-side Rust library;
- [ ] public application types and operation wrappers are generated from the fixed application descriptor;
- [ ] handwritten code owns only ACP protocol, transport, connection, and runtime client behavior;
- [ ] it centralizes ACP and Phenix extension client logic;
- [ ] applications receive typed APIs and events rather than raw internal envelopes;
- [ ] capability negotiation is explicit;
- [ ] transport is replaceable below the protocol API;
- [ ] durable state remains Phenix-owned;
- [ ] exact-head Source, Rust, Product, Docs, and Maintenance validation passes.
