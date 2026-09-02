# Socket transport library

status: specification-only

Implementation follows repository cleanup.

## Terminology

Phenix uses these terms consistently:

- **Application**: user-facing software such as Neovim, a terminal CLI, or a browser UI.
- **Protocol**: the message contract between an application and Phenix, such as ACP.
- **Adapter**: the Phenix-side implementation of an external protocol.
- **Client SDK**: reusable application-side code for speaking a protocol.
- **Binding**: a language-native API over a client SDK.
- **Transport**: the mechanism that moves protocol bytes or messages, such as stdio, a Unix socket, HTTP, or WebSocket.

A socket is a transport. It is not an application, protocol, adapter, or active Phenix plugin.

## Goal

Provide reusable local Unix-socket transport code that adapters, client SDKs, applications, tests, and future bindings can share without making sockets part of Phenix runtime semantics.

The first-party Rust crate/package is `phenix-transport-socket`.

It has no plugin identity and does not appear in `phenixPlugins`. Linking the library does not start a listener, open a connection, alter the resolved plugin graph, or grant authority.

## Ownership

`phenix-transport-socket` owns only local socket mechanics:

- parsed Unix socket endpoints;
- connect and listen primitives;
- user-only directory and socket permission setup;
- stale-socket handling;
- bounded framing and buffering helpers;
- connection lifecycle and shutdown helpers;
- reusable client and server halves where that reduces duplication.

Callers own protocol semantics and lifecycle policy.

The library owns no sessions, executions, routing, models, tools, authentication, permissions, persistence, frontend services, or protocol-specific request types.

## API shape

Prefer invariant-bearing Rust types and small composable operations. Invalid paths or impossible transport states should fail at construction rather than through later validation.

The library should support both sides of a local connection without forcing a server architecture:

```text
application/client SDK            adapter/runtime
        |                               |
        +---- phenix-transport-socket --+
```

A caller may use only the client half, only the listener half, or neither.

Protocol framing should be generic enough for ACP or another message protocol. Do not hard-code `phenix-client`, ACP, or JSON-RPC domain types into the transport library. A small generic codec/framing trait is preferable to a second protocol abstraction.

## Local security

- Prefer `$XDG_RUNTIME_DIR` for default local runtime endpoints when a caller requests a default.
- Create transport-owned directories for the current user only.
- Create sockets so other users cannot connect by default.
- Refuse to replace a non-socket path.
- Remove a stale socket only after proving no live listener owns it.
- Bound frame size and queued output.
- A malformed frame or broken peer affects only that connection.

Remote authentication and authorization belong to the adapter/protocol using a remote transport, not this library.

## Consumers

Expected consumers include:

- `phenix-adapter-acp` when an ACP adapter needs a local persistent-runtime connection;
- an ACP client SDK when an application connects to a separately running adapter;
- the terminal application;
- integration tests;
- future adapters or applications that need local IPC.

No consumer is required to use sockets. ACP over stdio remains valid. A browser-oriented adapter may use HTTP or WebSocket instead.

## Packaging

Expose the Rust crate as an ordinary independently buildable package/library. Do not create `phenixSockets`, `phenixTransports`, a transport registry, or a special plugin category.

If Nix convenience outputs are useful, they package the library artifact only. Runtime selection remains the responsibility of the application or adapter that uses it.

## Regression coverage

- constructing an invalid endpoint fails before connect/listen;
- user-only directory/socket permissions are enforced on Unix;
- connect/listen can exchange framed messages without protocol-specific types;
- frame and queued-output limits are enforced;
- malformed input closes only the affected connection;
- stale-socket handling never removes a live listener;
- client-only use does not start a listener;
- linking the library changes no Phenix plugin graph or runtime state;
- two independent consumers can use the same library without shared global state.

## Completion

- [ ] `phenix-transport-socket` is an ordinary passive Rust library;
- [ ] it has no plugin identity or runtime activation;
- [ ] client and server socket mechanics are reusable by adapters and applications;
- [ ] protocol/domain semantics stay outside the crate;
- [ ] security-sensitive endpoint and permission invariants are represented at the boundary;
- [ ] framing and buffering are bounded;
- [ ] exact-head Source, Rust, Product, and Maintenance validation passes.
