# Local client socket

Status: specification only. Implement after the repository cleanup inventory is complete and `spec/repository-cleanup.md` has been deleted.

## Goal

Let local frontends attach to one running Phenix process without adding another client API.

Use the canonical `phenix-client` `ClientEnvelope` and `ServerMessage` types on the wire. The socket is transport only.

## Transport

- Add a local Unix domain socket transport on Unix.
- Default to `$XDG_RUNTIME_DIR/phenix/phenix.sock` when a runtime directory is available.
- Allow an explicit socket path for tests, wrappers, and unusual environments.
- Keep stdio support. Stdio and socket connections must use the same connection handler and protocol semantics.
- Frame messages as newline-delimited JSON. Do not add a second request or event schema.
- Reject malformed or oversized frames with a connection-local protocol error and close that connection cleanly.
- Bound per-connection output buffering so a stalled client cannot grow memory without limit.

TCP, HTTP, WebSocket, and remote authentication are outside this PR.

## Local security

- Create the socket parent directory for the current user only.
- Create the socket so other users cannot connect by default.
- Treat filesystem ownership and permissions as the local authentication boundary.
- Refuse to replace a non-socket path.
- Remove a stale socket only after proving no live listener owns it.

A future remote transport must define its own authentication and authorization contract.

## Connection semantics

Each accepted connection is one frontend connection.

- Request IDs and frontend service registrations are connection-scoped.
- Disconnecting removes connection-scoped frontend providers and pending calls owned by that connection.
- Durable sessions and executions are not owned by the socket connection and must not disappear on disconnect.
- Multiple local clients may connect concurrently.
- Server events preserve their canonical ordering per connection.
- Slow or broken clients must not block unrelated client connections.

## Architecture

Factor the conductor transport boundary around one reusable connection operation, conceptually:

```text
transport stream
    |
    v
ClientEnvelope <-> connection handler <-> ServerMessage
    |
    v
conductor/runtime
```

The connection handler owns protocol decoding, request dispatch, event forwarding, and connection cleanup. Stdio and Unix sockets only supply byte streams.

Do not put session, routing, tool, plugin, authority, or persistence semantics in the socket implementation.

## Product integration

Expose a supported way to run the product as a local long-lived server, for example through the existing Harness/conductor executable with an explicit socket mode.

Do not make daemonization, service-manager integration, or background process supervision part of this PR. NixOS, systemd user units, and other launchers can wrap the foreground server later.

## Regression coverage

- A client can connect over the Unix socket and initialize through the canonical client protocol.
- A socket client can create a session, submit a prompt or callable request, and receive ordered responses/events.
- Two clients can operate independently at the same time.
- Disconnecting one client removes only its connection-scoped frontend providers and pending calls.
- Durable session state remains available after reconnect.
- A malformed frame closes only the offending connection.
- A stalled client cannot make per-connection buffering grow without bound.
- Stdio and socket paths produce equivalent canonical protocol behavior for the same request sequence.
- Socket permissions prevent unintended cross-user access by default.

## Completion

- [ ] one canonical `phenix-client` wire is used by stdio and Unix sockets;
- [ ] local socket connection lifecycle is isolated from durable session/execution lifecycle;
- [ ] multiple clients can connect concurrently;
- [ ] local permissions form an explicit security boundary;
- [ ] transport buffering and frame size are bounded;
- [ ] no transport-specific domain API or duplicate state owner is introduced;
- [ ] exact-head Source, Rust, Product, and Maintenance validation passes.
