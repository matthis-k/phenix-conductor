# ACP stdio application

status: partial

The crate provides an ACP server over stdio and a bounded channel transport for a configured application runtime. It does not yet provide `bin/phenix-acp`, package that binary, or connect the channel to a configured Phenix runtime. It is therefore not spawnable and cannot yet serve an editor.

Canonical application-integration terminology is defined by #442. ACP adapter semantics are defined by #437.

## Goal

Provide a spawnable ACP stdio entrypoint for applications that want to launch Phenix as an ACP agent process.

The package is `phenix-acp-stdio`. It will own the `phenix-acp` executable.

It is not a new protocol, runtime plugin, Client SDK, or application UI. It is a concrete composition of the ACP adapter with stdio transport and a configured Phenix runtime boundary.

This is the first editor-viability gate. Once this package and the ACP adapter are complete, `phenix-nvim` can migrate to ACP without waiting for a generated Lua binding.

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
        | application-interface contract
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

## Runtime boundary

The stdio executable must construct or connect to the configured Phenix product through an `ApplicationTransport`. The channel transport in this crate is only the hand-off point. A runtime bridge must receive each invocation, dispatch the matching typed application operation, and return its typed result.

The executable owns no parallel session, transcript, routing, authentication, permission, tool, execution, or persistence state.

All durable semantics remain in Phenix. The stdio process keeps only connection and protocol state.

If the configured product is hosted out of process, the implementation may reuse an internal transport such as `phenix-transport-socket`. That choice stays below ACP and must not change ACP behavior.

## ACP extensions

Expose the same standard ACP methods and negotiated `_phenix/...` extensions defined by `phenix-adapter-acp`.

The stdio package adds no stdio-specific ACP methods or metadata.

Extension schemas remain owned by the fixed application interface through the ACP adapter. The stdio package does not copy them.

## Application integration

This is the preferred simple process integration for editors that already support spawning ACP agents.

For example, `phenix-nvim` may either:

- spawn `phenix-acp` and speak ACP over stdio directly; or
- use `phenix-binding-lua` / `phenix-client-acp` when it wants an in-process application API.

Both paths must expose equivalent ACP and Phenix-extension semantics.

The direct stdio path is the required first migration because it removes the old internal-wire dependency without making language-binding generation a prerequisite for editor use.

## Packaging

Expose an independently buildable `phenix-acp-stdio` package containing `bin/phenix-acp`.

Do not require the socket transport package for the basic stdio build.

Do not add a runtime plugin identity for the stdio executable. Runtime adapter identity remains `phenix.adapter.acp` from #437.

The configured product package should expose or compose `phenix-acp` in a form that downstream Nix consumers such as `phenix-nvim` can pin without rebuilding protocol logic in the editor repository.

## Regression coverage

- spawning `phenix-acp` completes ACP `initialize` over stdin/stdout;
- session creation, prompt streaming, cancellation, list, and resume behave the same as the shared ACP adapter contract;
- negotiated Phenix extensions match `phenix-adapter-acp` and the fixed application descriptor;
- an editor-like subprocess client can initialize, create or resume a session, prompt, consume ordered updates, and cancel without using `phenix-client`;
- logs never appear on stdout;
- EOF exits cleanly;
- malformed ACP input does not corrupt durable Phenix state;
- restarting the stdio process can resume a durable session;
- stdio behavior does not require `phenix-transport-socket`;
- an in-process Client SDK connection and stdio connection produce equivalent protocol semantics for the same supported operations.

## Completion

- [x] `phenix-acp-stdio` is independently buildable;
- [ ] a runtime bridge dispatches typed application operations to configured Phenix services;
- [ ] `phenix-acp` is the stdio executable entrypoint;
- [x] ACP translation is reused from `phenix-adapter-acp`;
- [ ] stdout is protocol-only and stderr is diagnostic-only;
- [ ] no duplicate durable or domain state is introduced;
- [ ] stdio requires no socket dependency;
- [ ] an editor-like process journey uses only ACP and the public application contract;
- [ ] standard ACP and Phenix extension behavior matches #437 and `application-interface.md`;
- [ ] exact-head Source, Rust, Product, Docs, and Maintenance validation passes.
