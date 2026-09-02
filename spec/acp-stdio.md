# ACP stdio application

status: specification-only

Canonical application-integration terminology is defined by #442. ACP adapter semantics are defined by #437.

## Goal

Provide a spawnable ACP stdio entrypoint for applications that want to launch Phenix as an ACP agent process.

The package is `phenix-acp-stdio`. It owns the `phenix-acp` executable.

It is not a new protocol, runtime plugin, Client SDK, or application UI. It is a concrete composition of the ACP adapter with stdio transport and a configured Phenix runtime boundary.

## Architecture

```text
Application
  Neovim / editor / ACP client
        |
        | ACP JSON-RPC
        | stdin / stdout
        v
phenix-acp
  phenix-acp-stdio
        |
        v
phenix-adapter-acp
        |
        v
configured Phenix runtime
```

The executable reuses `phenix-adapter-acp`. It must not implement a second ACP translation layer.

## Stdio contract

- stdin carries ACP requests and notifications only;
- stdout carries ACP responses and notifications only;
- diagnostics and logs go to stderr;
- stdout buffering must not reorder protocol messages;
- EOF shuts down the connection cleanly;
- process termination cancels only process-scoped work unless canonical Phenix policy says otherwise;
- durable Phenix sessions remain resumable after process restart;
- malformed protocol input fails the ACP connection without corrupting durable runtime state.

Stdio framing follows the pinned ACP specification. Do not introduce Phenix-specific framing around ACP messages.

## Runtime ownership

The stdio executable may construct or launch the configured Phenix product needed to serve the ACP connection, but it owns no parallel session, transcript, routing, authentication, permission, tool, execution, or persistence state.

All durable semantics remain in Phenix. The stdio process keeps only connection and protocol state.

If the configured product is hosted out of process, the implementation may reuse an internal transport such as `phenix-transport-socket`. That choice stays below ACP and must not change ACP behavior.

## ACP extensions

Expose the same standard ACP methods and negotiated `_phenix/...` extensions defined by `phenix-adapter-acp`.

The stdio package adds no stdio-specific ACP methods or metadata.

## Application integration

This is the preferred simple process integration for editors that already support spawning ACP agents.

For example, `phenix-nvim` may either:

- spawn `phenix-acp` and speak ACP over stdio directly; or
- use `phenix-binding-lua` / `phenix-client-acp` when it wants an in-process application API.

Both paths must expose equivalent ACP and Phenix-extension semantics.

## Packaging

Expose an independently buildable `phenix-acp-stdio` package containing `bin/phenix-acp`.

Do not require the socket transport package for the basic stdio build.

Do not add a runtime plugin identity for the stdio executable. Runtime adapter identity remains `phenix.adapter.acp` from #437.

## Regression coverage

- spawning `phenix-acp` completes ACP `initialize` over stdin/stdout;
- session creation, prompt streaming, cancellation, list, and resume behave the same as the shared ACP adapter contract;
- negotiated Phenix extensions match `phenix-adapter-acp`;
- logs never appear on stdout;
- EOF exits cleanly;
- malformed ACP input does not corrupt durable Phenix state;
- restarting the stdio process can resume a durable session;
- stdio behavior does not require `phenix-transport-socket`;
- an in-process Client SDK connection and stdio connection produce equivalent protocol semantics for the same supported operations.

## Completion

- [ ] `phenix-acp-stdio` is independently buildable;
- [ ] `phenix-acp` is the stdio executable entrypoint;
- [ ] ACP translation is reused from `phenix-adapter-acp`;
- [ ] stdout is protocol-only and stderr is diagnostic-only;
- [ ] no duplicate durable or domain state is introduced;
- [ ] stdio requires no socket dependency;
- [ ] standard ACP and Phenix extension behavior matches #437;
- [ ] exact-head Source, Rust, Product, and Maintenance validation passes.
