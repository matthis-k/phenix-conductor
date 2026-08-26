# Phenix AI

This repository contains the Phenix kernel, replaceable first-party Plugin Suite, Harness product assembly, and protocol/backend adapters. The current GitHub repository name is temporary.

The Neovim frontend lives separately in `matthis-k/phenix-nvim`. This repository does not own editor windows, input handling, transcript presentation, Neovim plugin packaging, or frontend-specific tests.

The project is under active architectural development. Prefer the current typed Rust/ACP implementation over compatibility layers or historical APIs.

## Architecture

```text
frontends / protocol adapters
            |
            v
      phenix-harness
      product policy
            |
            v
      phenix-kernel
 generic mechanisms only
            |
            v
  selected plugin providers
  Phenix Plugin Suite or alternatives
```

`phenix-kernel` owns plugin lifecycle, provider resolution, authority attenuation, generic persistence, events, and tasks. It does not own session, context, execution, planning, tool, model, frontend, or other agent-domain semantics.

`phenix-plugin-suite` implements the first-party Phenix services through the same contracts available to alternate plugins. `phenix-harness` selects the plugin set and product policy. Omitting a provider removes the service. Replacing a provider does not require a kernel change.

`phenix-conductor` remains in the workspace as migration source and compatibility coverage while the plugin migration is completed. It is not the supported product package and must not gain new domain ownership.

ACP is one protocol boundary. `phenix-acp` contains wire interoperability types; backend adapters translate ACP agents without becoming a second semantic runtime.

### Rust boundaries

| Crate | Responsibility |
| --- | --- |
| `phenix-kernel` | Generic plugin host, trust boundaries, persistence enforcement, events, tasks |
| `phenix-plugin-suite` | Replaceable first-party Phenix services |
| `phenix-harness` | Supported kernel + selected-plugin product assembly |
| `phenix-acp` | ACP wire interoperability boundary |
| `phenix-backend-acp` | ACP backend adapter |
| `phenix-conductor` | Migration source and compatibility coverage until duplicate ownership is removed |

There is no UI crate or Neovim plugin in this repository.

## Configuration

A fresh conductor is unconfigured. A client selects a source root and descriptors, then calls `_phenix/config/load`; the conductor resolves relative paths beneath that root, validates every source, and atomically creates an immutable revision. New session trees use the active revision; existing trees remain pinned to the revision under which they were created. The conductor does not implicitly discover XDG configuration or repository examples.

For the standard ACP projection, initialization order is explicit: `initialize`, then `_phenix/config/load`, then `session/new`. `session/new` cannot create a standard Phenix session before an active configuration revision exists. Frontends are responsible for supplying their selected configuration before requesting the session. After loading, `_phenix/config/get` returns the active revision and its callable workflow catalog; integrations must use that conductor-owned catalog rather than re-derive workflows from their authoring input.

The example authoring configuration under `config/phenix-harness/` is retained as an explicit application configuration. Its name is not the repository name.

The kernel is mechanism, not Phenix policy. Harness composition and plugins own the selected product behavior.


### Project context and skills

Project context and skills are first-party Plugin Suite services. The context plugin owns discovery, exact content identity, injection history, and projection. The supported Harness reaches them through the ordinary kernel service contract. Kernel-only mode has no context or skill behavior.

Project instructions remain ambient input. Discoverable project documents and skills are revisioned resources rather than configuration identity. Skill metadata never expands execution authority; script execution still uses ordinary workspace/tool authority.

## Packages

The flake exposes the supported compositions directly:

- `packages.<system>.phenix-kernel`: kernel-only runtime;
- `packages.<system>.phenix-harness`: default Harness composition;
- `packages.<system>.phenix`: supported product alias for the Harness;
- `lib.mkPhenixPlugin`: external/resource plugin packaging;
- `lib.mkPhenix`: declarative kernel + plugin composition.

The legacy conductor crate remains a migration source inside the Rust workspace. Product composition goes through `phenix-harness`.

## Built-in runtime authentication

The runtime advertises one ACP terminal-auth method per implemented provider. ACP frontends can launch those flows directly; the equivalent command is:

```sh
phenix-conductor runtime auth login <provider>
```

Credentials are stored atomically in `${XDG_STATE_HOME:-$HOME/.local/state}/phenix/credentials.json`; on Unix, newly created credential directories use mode `0700` and credential files are forced to `0600`. Set `PHENIX_CREDENTIAL_FILE` to select a different credential store. Provider-native environment variables remain supported and are used when no stored credential exists.

`openai-responses` is OpenAI's API-key-authenticated Responses API. `openai-codex` is the distinct ChatGPT subscription path: `auth login openai-codex` prints a browser authorization link, verifies the OAuth callback state and PKCE exchange, persists refresh credentials, and refreshes access tokens before use. Its model requests use the Phenix-owned prompt and tool loop against the ChatGPT Codex Responses endpoint, so no additional agent harness or system prompt is introduced.

Configured model identities use `Phenix/provider/model`. The bundled ChatGPT subscription profile is `router.chatgpt-plus`.

The Neovim plugin and configured Neovim wrapper are exported by `phenix-nvim` instead.

## Design rules

- Prefer one canonical typed API over versioned or compatibility surfaces.
- Parse external data at boundaries and keep invalid runtime states difficult to represent internally.
- Preserve typed failure modes across configuration, transport, protocol, and runtime boundaries.
- Standard ACP remains authoritative for singular-agent behavior; Phenix extensions cover aggregate orchestration concepts.
- Do not add parallel frontend-to-agent protocols or duplicate orchestration implementations.
- Keep frontend-specific behavior and packaging in frontend repositories.
- Tests should assert domain behavior, user-visible protocol semantics, or cross-boundary integration, not duplicated configuration facts.

## Development

```sh
nix develop
maintenance fix
maintenance all
```

Validation is separated into source, Rust, integration/system, and realized product boundaries. The product layer exercises the installed Harness and plugin compositions; frontend behavior is tested in frontend repositories.

See `DEVELOPMENT.md` for focused validation commands.
