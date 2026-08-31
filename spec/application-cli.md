# Terminal CLI application

Status: specification only. Implement after repository cleanup.

## Terminology

Phenix uses these terms consistently:

- **Application**: user-facing software such as Neovim, a terminal CLI, or a browser UI.
- **Protocol**: the message contract between an application and Phenix.
- **Adapter**: the Phenix-side implementation of an external protocol.
- **Client SDK**: reusable application-side code for speaking a protocol.
- **Binding**: a language-native API over a client SDK.
- **Transport**: the mechanism that moves protocol bytes or messages.

The terminal CLI is an application. It is not a Phenix runtime plugin.

## Goal

Provide a deliberately small Pi-like terminal application for Phenix.

The first-party package is `phenix-cli`. It owns the `phenix` executable and no runtime plugin identity.

Running `phenix` opens a line-oriented interactive conversation. Running `phenix <prompt...>` submits one prompt and exits.

The CLI owns terminal interaction and rendering only. Phenix remains authoritative for sessions, execution, routing, tools, authentication, permissions, and persistence.

## Rename the existing command utility plugin

The repository currently uses `phenix-plugin-cli` / `phenix.cli` for command discovery and probing. That runtime component is not the terminal application.

Rename it to:

- crate/package: `phenix-plugin-command-toolbelt`;
- plugin id: `phenix.command-toolbelt`;
- package-set entry: `phenixPlugins.${system}.command-toolbelt`.

Update current consumers directly. Keep no prerelease compatibility alias.

## Application architecture

```text
phenix-cli
   |
   | application-side Client SDK
   v
ACP + negotiated Phenix extensions
   |
   v
phenix-adapter-acp
   |
   v
Phenix runtime
```

The CLI should use the same client SDK as other first-party applications and bindings where practical. It must not import conductor internals or use the internal `phenix-client` wire as its public application API.

Transport stays below the protocol. Stdio may be used when the CLI owns/spawns an adapter process. A persistent deployment may reuse `phenix-transport-socket` from #436. CLI behavior must not change with transport.

## Interactive mode

`phenix` with no prompt enters a simple line-oriented REPL.

```text
> user prompt
assistant output
> next prompt
```

- create a session on first use unless an existing session was selected;
- stream assistant output and useful execution updates;
- keep tool output compact by default;
- send diagnostics to stderr;
- EOF exits cleanly;
- Ctrl-C cancels an active turn; Ctrl-C while idle exits.

Do not require a full-screen TUI framework.

## One-shot mode

`phenix <prompt...>` submits one prompt, prints final user-visible output, and exits.

Return zero on successful completion and non-zero for protocol, policy, authentication, routing, backend, tool, cancellation, or transport failure.

## Commands

Initial commands:

```text
/help
/new [name]
/sessions
/resume <session-id>
/model
/cancel
/quit
```

Use standard ACP operations when available. Use negotiated Phenix ACP extensions for Phenix-only behavior.

Do not add a CLI-specific protocol method for an operation already represented by ACP or a reusable Phenix extension.

## State

Keep only ephemeral application state such as the selected session, active request, and rendering state.

Reconstruct durable state after reconnect. Do not persist a second transcript, session registry, model database, credential store, or execution journal.

## Authentication and permissions

Use ACP authentication, permission, and elicitation flows where they represent the Phenix operation. Use Phenix ACP extensions only when standard ACP cannot preserve the semantics.

Never grant runtime authority because the application is local.

## Packaging

Expose `phenix-cli` as an ordinary application package, not through `phenixPlugins`.

The default Phenix product may include the application for convenience, but installing or omitting the CLI changes no runtime plugin graph.

## Regression coverage

- `phenix-cli` builds and runs without a runtime plugin identity;
- the command utility plugin is renamed to `phenix.command-toolbelt` without behavior loss;
- the CLI can create a session and complete two sequential prompts;
- one-shot mode returns the correct exit status;
- list/resume use runtime-owned state;
- model selection uses ACP config or the negotiated Phenix routing extension rather than direct provider calls;
- Ctrl-C cancels the active execution and leaves the session usable;
- reconnect restores durable state without a local transcript store;
- stdout/stderr remain separated;
- stdio and socket-backed deployments have equivalent application semantics;
- no internal `phenix-client` envelope is required by the application API.

## Completion

- [ ] `phenix-cli` is an application package, not a runtime plugin;
- [ ] it owns the `phenix` terminal executable;
- [ ] command discovery remains in `phenix.command-toolbelt`;
- [ ] one-shot and line-oriented interactive modes work;
- [ ] the application uses the shared client SDK and ACP boundary;
- [ ] no duplicate durable state or runtime policy is introduced;
- [ ] transport remains below protocol semantics;
- [ ] exact-head Source, Rust, Product, and Maintenance validation passes.
