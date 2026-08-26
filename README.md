# Phenix AI

This repository contains the Phenix kernel, replaceable first-party Plugin Suite, Harness product assembly, and protocol/backend adapters. The current GitHub repository name is temporary.

The Neovim frontend lives separately in `matthis-k/phenix-nvim`. This repository does not own editor windows, input handling, transcript presentation, Neovim plugin packaging, or frontend-specific tests.

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
      phenix-kernel
 generic mechanisms only
            |
            v
  selected plugin providers
  Phenix Plugin Suite or alternatives
```

`phenix-kernel` owns plugin lifecycle, provider resolution, authority attenuation, generic persistence, events, and tasks. It does not own session, context, execution, planning, tool, model, frontend, or other agent-domain semantics.

`phenix-plugin-suite` implements the first-party Phenix services through the same contracts available to alternate plugins. `phenix-harness` selects the plugin set and product policy. Omitting a provider removes the service. Replacing a provider does not require a kernel change.

`phenix-conductor` remains in the Rust workspace as migration source and compatibility coverage while the plugin migration is completed. It is not a supported product package and must not gain new domain ownership.

ACP is one protocol boundary. `phenix-acp` owns ACP wire interoperability, while the supported product semantics remain in Harness-selected plugins.

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

## Product composition

The supported runtime is `phenix-harness`. It activates a selected plugin set through ordinary kernel manifests and service resolution. The default composition loads the first-party Plugin Suite.

Nix composition can select a subset of embedded first-party plugins, add external or resource-only plugins, or replace a first-party provider. Provider omission must remove the service rather than expose a kernel fallback.

Plugin-owned durable state is canonical. The kernel enforces namespace ownership, migrations, transactions, and authority without interpreting first-party domain rows.

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

There is no supported `phenix-conductor` package or app. The conductor crate remains only inside the Rust workspace while compatibility paths are migrated or removed.

## Protocol and provider boundaries

ACP is an adapter boundary. Frontends and other ACP clients should reach the supported Harness/plugin composition rather than a second conductor-owned semantic runtime.

Backend adapters translate plugin-owned execution requests into provider protocols. Provider conversation state is disposable. Durable Phenix state stays with the owning plugins.

Authentication and provider selection are product/plugin concerns. They must not become privileged kernel APIs or ambient process authority.

## Design rules

- Keep one canonical typed API per semantic operation.
- Keep one durable owner per semantic domain.
- Parse external data at boundaries and keep invalid runtime states difficult to represent internally.
- Preserve typed failure modes across configuration, transport, protocol, plugin, and provider boundaries.
- Authority only attenuates across plugin, task, retry, event, persistence, and provider boundaries.
- Kernel-only mode has no hidden first-party fallbacks.
- First-party plugins use the same contracts as alternate plugins.
- Do not add parallel frontend-to-agent protocols or duplicate orchestration implementations.
- Keep frontend-specific behavior and packaging in frontend repositories.
- Tests should assert domain behavior, user-visible protocol semantics, or cross-boundary integration rather than duplicated configuration facts.

## Development

```sh
nix develop
maintenance fix
maintenance all
```

Validation is separated into source, Rust, integration/system, realized product, Nix composition, and Maintenance boundaries. Product validation exercises the installed Harness and plugin compositions. Frontend behavior is tested in frontend repositories.

See `DEVELOPMENT.md` for focused validation commands and `rust/ARCHITECTURE.md` for the current ownership model.
