# Local socket plugin

Status: specification only. Implement after the repository cleanup inventory is complete and `spec/repository-cleanup.md` has been deleted.

## Goal

Provide local multi-client IPC as an ordinary optional Phenix plugin, not as privileged conductor transport behavior.

The first-party package is `phenix-plugin-socket`, exposed through `phenixPlugins.${system}.socket`, with plugin identity `phenix.socket`.

Omitting the plugin must omit the socket listener completely. The conductor must not contain a hidden socket fallback.

## Ownership

`phenix-client` remains the canonical client/server protocol owner. Core/conductor may expose the smallest generic connection mechanism needed by transports, but they do not own Unix socket policy, paths, listener lifecycle, permissions, framing, or accept loops.

`phenix.socket` owns:

- Unix-domain socket path selection;
- listener creation and teardown;
- local filesystem permission policy;
- accept-loop lifecycle;
- per-connection framing and buffering;
- adaptation from a byte stream to the canonical client connection boundary.

It owns no session, routing, model, tool, authority, frontend-service, or persistence semantics.

If the current plugin API lacks a generic way for a transport plugin to hand a `ClientEnvelope` stream to the canonical connection handler, add one narrow transport-neutral interface in the generic runtime. Do not add a socket-specific core API.

## Plugin contract

- Package and select the implementation through the same `phenixPlugins` path as other first-party plugins.
- Plugin activation starts the configured listener; plugin deactivation stops accepting new connections and closes plugin-owned listener resources cleanly.
- Requested filesystem/process authority is explicit in the plugin manifest and attenuated by Harness policy.
- Stable configuration contributes socket settings through the canonical configuration/resolution path.
- Development-mode reload must not mutate the active listener until a replacement plugin graph is valid.
- Plugin metadata and diagnostics expose whether the socket plugin is selected and its configured endpoint without exposing secrets.

No `phenixSockets`, `phenixTransports`, special conductor registry, or second plugin model may be introduced.

## Transport

- Add a local Unix domain socket transport on Unix.
- Default to `$XDG_RUNTIME_DIR/phenix/phenix.sock` when a runtime directory is available.
- Allow an explicit socket path for tests, wrappers, and unusual environments.
- Keep any existing supported stdio connection path independent. Stdio and socket connections must reach the same canonical connection handler and protocol semantics.
- Frame messages as newline-delimited JSON using canonical `phenix-client` `ClientEnvelope` and `ServerMessage` values.
- Reject malformed or oversized frames with a connection-local protocol error and close only that connection.
- Bound per-connection output buffering so a stalled client cannot grow memory without limit.

TCP, HTTP, WebSocket, remote authentication, daemonization, and service-manager integration are outside this PR.

## Local security

- Create the socket parent directory for the current user only.
- Create the socket so other users cannot connect by default.
- Treat filesystem ownership and permissions as the local authentication boundary.
- Refuse to replace a non-socket path.
- Remove a stale socket only after proving no live listener owns it.
- Do not let plugin configuration grant runtime authority beyond the Harness grant.

A future remote transport must be another plugin with its own authentication and authorization contract.

## Connection semantics

Each accepted socket connection is one canonical frontend connection.

- Request IDs and frontend-service registrations are connection-scoped.
- Disconnecting removes connection-scoped frontend providers and pending calls owned by that connection.
- Durable sessions and executions are not owned by the socket connection and must not disappear on disconnect.
- Multiple local clients may connect concurrently.
- Server events preserve canonical ordering per connection.
- Slow or broken clients must not block unrelated client connections.

## Architecture

```text
phenix-plugin-socket
  Unix listener / permissions / framing
              |
              v
      canonical client connection
              |
              v
       phenix-client protocol
              |
              v
       conductor/runtime
```

The plugin owns the transport edge. The canonical connection handler owns protocol dispatch, event forwarding, and connection-scoped cleanup.

Do not place socket handling in `phenix-conductor::main`, the Harness executable, or another first-party plugin as a convenience path.

## Product integration

The default Harness may select `phenix.socket`, but that is product policy rather than core behavior.

A custom Harness can omit or replace the socket plugin without recompiling or modifying the conductor. The public wrapper should expose only the selected plugin's socket behavior.

## Regression coverage

- A Harness with `phenix.socket` selected can accept a canonical client connection.
- A Harness without `phenix.socket` has no socket listener or socket-specific fallback.
- An alternate transport plugin can use the same generic canonical connection mechanism without socket code.
- A socket client can create a session, submit work, and receive ordered responses/events.
- Two clients can operate independently at the same time.
- Disconnecting one client removes only its connection-scoped frontend providers and pending calls.
- Durable session state remains available after reconnect.
- A malformed frame closes only the offending connection.
- A stalled client cannot make per-connection buffering grow without bound.
- Socket permissions prevent unintended cross-user access by default.
- Harness authority can deny or attenuate the socket plugin's requested capabilities.

## Completion

- [ ] `phenix-plugin-socket` is an ordinary first-party plugin in `phenixPlugins`;
- [ ] socket selection/omission/replacement uses ordinary plugin composition;
- [ ] no socket listener or socket policy remains built into the conductor/Harness;
- [ ] one canonical `phenix-client` wire is reused;
- [ ] local socket lifecycle is isolated from durable session/execution lifecycle;
- [ ] multiple clients can connect concurrently;
- [ ] local permissions form an explicit security boundary;
- [ ] transport buffering and frame size are bounded;
- [ ] no transport-specific domain API, registry, or duplicate state owner is introduced;
- [ ] exact-head Source, Rust, Product, and Maintenance validation passes.
