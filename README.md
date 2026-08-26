# Phenix AI

This repository owns the generic Phenix runtime, conductor, shared client contracts, independently packaged first-party plugins, client adapters, and the supported Harness product. The current GitHub repository name is temporary.

The Neovim frontend lives in `matthis-k/phenix-nvim`. This repository does not own editor windows, input handling, transcript presentation, Neovim packaging, or frontend-specific tests.

The project is under active architectural development. Replace obsolete contracts instead of preserving compatibility layers.

## Architecture

```text
frontends / protocol adapters
            |
            v
      phenix-harness
      product policy
            |
            v
     phenix-conductor
 generic server process
            |
            v
       phenix-core
 generic runtime mechanisms
            |
            v
  selected plugin providers
```

`phenix-core` owns plugin identity and lifecycle, deterministic service resolution, authority attenuation, generic persistence, events, tasks, and embedded, external, and resource-only hosting. It does not own session, context, execution, planning, tool, model, frontend, or other first-party agent semantics.

`phenix-conductor` owns the generic server process and client transport. It hosts only configured plugins. A zero-plugin conductor has no first-party fallback behavior.

First-party `phenix-plugin-*` crates own Phenix agent-domain services through the same core contracts available to alternate providers. A thin `phenix-plugin-catalog` collects embedded factories but owns no durable state or product policy.

`phenix-harness` owns the supported product assembly. It selects plugins, grants authority, chooses persistence, loads product configuration and skills, and exposes the wrapped `phenix` product.

`phenix-client` owns the canonical client/server contract. `phenix-acp` translates that contract to ACP without owning application semantics.

### Rust boundaries

| Crate or package | Responsibility |
| --- | --- |
| `phenix-core` | Generic plugin host, trust boundaries, persistence enforcement, events, tasks |
| `phenix-client` | Canonical client/server contracts |
| `phenix-conductor` | Generic configured server and transport |
| `phenix-plugin-*` | Independently owned first-party services |
| `phenix-plugin-catalog` | Thin embedded-factory catalog |
| `phenix-harness` | Supported conductor + selected-plugin product assembly |
| `phenix-acp` | ACP adapter to `phenix-client` |
| `phenix-backend-*` | Provider/backend adapters |

There is no UI crate or Neovim plugin in this repository.

## Product composition

The normal `phenix` package is the supported Harness composition. It is built through the same public package interfaces available to users.

Nix exposes independently packaged first-party plugins through `phenixPlugins.<system>.*` and client adapters through `phenixClients.<system>.*`. `wrappers.phenix.wrap` and `lib.mkPhenix` assemble a conductor with an explicit plugin selection. Omitting a plugin removes its service unless another selected provider supplies the same contract.

Plugin-owned durable state is canonical. Core enforces namespace ownership, migrations, transactions, and authority without interpreting first-party domain rows.

### Product configuration and skills

Supported runtime configuration lives in `config/phenix/runtime.nix`. Skills and product resources live under `config/phenix/skills/`.

The Harness packages these resources and loads agent definitions, orchestration definitions, and routing profiles through plugin-owned services. Product configuration does not become hidden conductor policy.

Project context and skills are context-plugin resources. Their metadata never expands execution authority. Script or workspace access still uses ordinary service authority.

## Packages

The flake exposes:

- `packages.<system>.phenix-core`;
- `packages.<system>.phenix-client`;
- `packages.<system>.phenix-conductor`;
- `packages.<system>.phenix-harness`;
- `packages.<system>.phenix`;
- `phenixPlugins.<system>.*`;
- `phenixClients.<system>.*`;
- `wrappers.phenix.wrap`;
- `lib.mkPhenixPlugin`;
- `lib.mkPhenixClient`;
- `lib.mkPhenix`.

`phenix-kernel`, `phenix-protocol`, and `phenix-plugin-suite` are superseded names. Do not restore them as competing package owners.

## Protocol and provider boundaries

Frontends and protocol adapters reach configured conductor services through the canonical client contract. They do not own durable application state.

Backend adapters translate execution requests into provider protocols. Provider conversation state is disposable. Durable Phenix state stays with the owning plugins.

Authentication and provider selection are plugin and Harness concerns. They must not become privileged core APIs or ambient process authority.

## Design rules

- Keep one canonical typed API per semantic operation.
- Keep one durable owner per semantic domain.
- Parse external data at boundaries and keep invalid runtime states difficult to represent internally.
- Preserve typed failure modes across configuration, transport, protocol, plugin, and provider boundaries.
- Authority only attenuates across plugin, task, retry, event, persistence, and provider boundaries.
- Zero-plugin mode has no hidden first-party fallbacks.
- First-party plugins use the same contracts as alternate plugins.
- Do not add parallel frontend-to-agent protocols or duplicate orchestration registries.
- Keep frontend-specific behavior and packaging in frontend repositories.
- Tests should assert behavior, protocol semantics, or cross-boundary integration rather than duplicated configuration facts.

## Development

```sh
nix develop
maintenance fix
maintenance all
```

Validation is separated into source, Rust, integration/system, realized product, Nix composition, and Maintenance boundaries. Product validation exercises installed conductor and Harness compositions. Frontend behavior is tested in frontend repositories.

See `DEVELOPMENT.md` for focused validation commands and `rust/ARCHITECTURE.md` for the current ownership model.
