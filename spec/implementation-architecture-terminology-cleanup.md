---
temporary: true
---

# Architecture terminology convergence

## Goal

Remove stale current-state terminology after the ACP and plugin-runtime migrations. This PR changes wording and names only where the architecture is already decided.

## Required changes

- Update README and current docs to use Adapter, Client SDK, Binding, Transport, Application, runtime provider, plugin runtime, graph generation, and artifact revision according to their owning specs.
- Remove stale current references to old `phenix-acp`, `phenixClients`, `mkPhenixClient`, `phenix.sdk`, `phenix.cli`, and the closed `External` runtime model where they describe current architecture.
- Distinguish the internal `phenix-client` wire from application-side Client SDKs.
- Distinguish plugin artifacts from product/domain artifacts and plugin resources from packaged/configuration resources when context is ambiguous.
- Keep genuine historical references only outside normative current architecture.
- Update repository examples, package descriptions, comments, and test names when they encode obsolete current terminology.

## Boundary

Do not change semantics here. Any wording conflict that exposes two plausible architectural meanings becomes an explicit finding for the following precision or architecture PRs.

## Completion

- current architecture docs use one vocabulary;
- README matches implemented package/runtime roles;
- no obsolete current identity remains without an explicit reason;
- Source, Docs, Product, and Maintenance pass on the exact head;
- delete this temporary implementation slice before merge.
