---
temporary: true
---

# Canonical ACP adapter convergence

## Goal

Converge the existing ACP implementation onto the canonical application-integration roles without compatibility aliases.

## Required changes

- Rename the current Phenix-side ACP runtime package to `phenix-adapter-acp`.
- Use runtime plugin identity `phenix.adapter.acp` and package-set entry `phenixPlugins.${system}.adapter-acp`.
- Classify the adapter as a runtime plugin. It owns ACP translation only.
- Move stdio process ownership to `phenix-acp-stdio`; the adapter must not own process or transport lifecycle.
- Remove obsolete `phenixClients.${system}.acp`, `mkPhenixClient`, old `phenix-acp` runtime packaging, and old ACP runtime identities when no remaining consumer requires them.
- Keep the internal `phenix-client` wire internal. Do not present it as the public ACP Client SDK.
- Update Nix composition, product packaging, catalog entries, tests, examples, and architecture checks directly.
- Author the adapter through the canonical Rust-native plugin model from `plugin-authoring-macro.md`.

## Invariants

- Adapter, Client SDK, Binding, Transport, and Application remain distinct roles.
- Transport choice does not change ACP semantics.
- Durable Phenix state remains runtime-owned.
- No prerelease compatibility alias preserves the old package or runtime identity.

## Completion

- canonical package, runtime, and package-set identities are the only current ACP adapter identities;
- stdio ownership is outside the adapter;
- obsolete client-package category is removed when unused;
- `adapter-acp.md` and `acp-stdio.md` accurately classify current implementation state;
- Source, Rust, Product, and Maintenance pass on the exact head;
- delete this temporary implementation slice before merge.
