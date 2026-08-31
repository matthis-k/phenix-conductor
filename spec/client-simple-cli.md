# CLI plugin

Status: specification only. Implement after repository cleanup and on the canonical local connection from #436.

## Goal

Provide a deliberately small Pi-like Phenix terminal frontend as an ordinary plugin.

The first-party package is `phenix-plugin-cli`, exposed through `phenixPlugins.${system}.cli`, with plugin identity `phenix.cli`.

Running `phenix` should open a plain interactive conversation. Running `phenix <prompt...>` should support a simple one-shot request.

The CLI plugin owns terminal interaction and rendering only. It must not own agent logic, routing policy, session persistence, tool execution, provider state, or durable configuration.

## Rename the existing command utility plugin

The current repository already uses `phenix-plugin-cli` / `phenix.cli` for discovery and management of command-line tools. That name belongs to the human-facing terminal client after this PR.

Rename the existing command utility plugin to:

- crate/package: `phenix-plugin-command-toolbelt`;
- plugin id: `phenix.command-toolbelt`;
- public package-set entry: `phenixPlugins.${system}.command-toolbelt`.

`phenix.command-toolbelt` continues to own command discovery, availability/version probing, and any command-tool execution/service semantics it already owns. Do not merge those responsibilities into the interactive CLI plugin.

This is a prerelease repository: update current consumers directly and do not retain `phenix.cli` as a compatibility alias for the old command-tool plugin.

## Plugin ownership

- `phenix.cli` is selected, omitted, configured, and replaced through ordinary plugin composition.
- The plugin owns the `phenix` terminal executable entrypoint.
- Omitting `phenix.cli` omits the interactive CLI without changing conductor behavior.
- If plugin metadata/packaging needs a generic executable-entrypoint concept, add it generically rather than creating a CLI-specific package path.
- No separate `phenixClients`, CLI registry, frontend registry, or privileged conductor path is introduced.

## Shape

Keep the implementation deliberately small:

- no full-screen TUI;
- no alternate screen buffer;
- no local transcript database;
- no duplicate config system;
- no CLI-owned orchestration or tool loop;
- no provider-specific API calls.

Use ordinary terminal input/output plus minimal ANSI formatting when supported.

## Connection

Use the canonical `phenix-client` protocol through the selected local transport plugin.

The default first-party composition may use `phenix.socket` from #436. The CLI must depend only on the canonical connection capability, not socket internals.

Normal behavior should be zero-configuration:

1. connect through an available selected local transport;
2. optionally spawn/connect through another supported local transport when explicitly configured;
3. expose explicit connection overrides for scripts and debugging.

The CLI must not change protocol semantics based on transport.

## Interactive mode

`phenix` with no prompt enters a line-oriented REPL.

```text
> user prompt
assistant output
> next prompt
```

- Create a session on first use unless the user selected an existing session.
- Stream assistant output and useful execution updates as they arrive.
- Keep tool rendering compact by default.
- Write diagnostics to stderr.
- EOF exits cleanly.
- Ctrl-C during an active turn requests cancellation; Ctrl-C while idle exits.

Do not require a terminal UI framework for the first implementation.

## One-shot mode

`phenix <prompt...>` submits one prompt, prints the final user-visible output, and exits.

- Exit zero on a successful completed turn.
- Exit non-zero on connection, protocol, policy, authentication, routing, backend, tool, or cancellation failures.
- Keep machine-readable output as a later explicit option rather than changing the default human output contract.

## Commands

Keep slash commands few and directly backed by canonical client operations.

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

Rules:

- `/new` creates a canonical session.
- `/sessions` reads conductor-owned session state.
- `/resume` selects an existing canonical session.
- `/model` lists/selects routing choices exposed by canonical routing/session-target APIs.
- `/cancel` requests cancellation of the active execution.
- `/quit` exits without deleting the session.

Add another slash command only when it maps to a stable canonical operation and materially improves the basic terminal workflow.

## Rendering

Default rendering:

- assistant text as normal stdout;
- reasoning only when canonical policy exposes it;
- tool start/completion as compact lines;
- long tool payloads omitted unless user-visible;
- errors on stderr;
- no spinner or UI state that obscures protocol state.

A verbose flag may print more execution metadata. The default stays quiet.

## State

The conductor is authoritative.

The CLI plugin may keep only ephemeral UI/connection state such as selected session ID, active request/execution ID, and terminal rendering state.

It must reconstruct durable state after reconnect. Do not persist a second transcript, session list, model-selection database, or pending-operation journal.

## Authentication and permissions

Use canonical Phenix authentication and frontend-service flows.

- Collect only canonical authentication input requested by the server.
- Do not store secrets in CLI history.
- Permission prompts default to an explicit user decision.
- Running locally must not grant extra authority.

## Regression coverage

- `phenix.cli` is selectable/omittable/replacable through ordinary plugin composition.
- Omitting `phenix.cli` removes the terminal executable/integration without a conductor fallback.
- The old command utility plugin is renamed to `phenix.command-toolbelt` and retains its command-tool behavior.
- No old `phenix.cli` compatibility alias points to command-toolbelt semantics.
- `phenix` can create a session and complete two sequential prompts in one REPL.
- `phenix <prompt>` completes one turn and returns the correct exit status.
- `/sessions` and `/resume` use conductor-owned session state.
- `/model` uses canonical routing/target contracts without direct provider calls.
- Ctrl-C cancels an active execution and leaves the session usable.
- Disconnect/reconnect restores session state without a local transcript store.
- CLI output keeps assistant output ordered and diagnostics separate.
- Different local transports have equivalent client semantics.

## Completion

- [ ] `phenix-plugin-cli` is an ordinary first-party plugin in `phenixPlugins`;
- [ ] the plugin owns the `phenix` terminal executable;
- [ ] the existing command utility plugin is renamed to `phenix-plugin-command-toolbelt` / `phenix.command-toolbelt`;
- [ ] selection, omission, and replacement use ordinary plugin composition;
- [ ] one-shot and line-oriented interactive modes work;
- [ ] no TUI framework or duplicate durable state is introduced;
- [ ] model/auth/permission behavior goes through canonical Phenix contracts;
- [ ] the client uses the shared canonical connection capability from the selected transport plugin;
- [ ] exact-head Source, Rust, Product, and Maintenance validation passes.
