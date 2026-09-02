---
temporary: true
---

# Kernel composition and provider resolution closure

## Goal

Converge plugin contributions, provider selection, and graph binding onto one kernel-owned composition model.

## Required changes

- Use one canonical contribution representation derived from plugin authoring declarations or runtime builders.
- Keep concrete plugin dependencies distinct from interface imports.
- Resolve providers during candidate graph construction, never by live runtime search at invocation time.
- Check structural compatibility, version compatibility, authority, enablement, scope, and composition policy before activation.
- Let authority determine eligibility and composition policy determine preference.
- Make explicit provider binding unable to bypass compatibility or authority.
- Break equal preference deterministically by stable identity.
- Resolve optional fallback plans only when the interface permits fallback and pin them to the same graph generation.
- Treat post-dispatch provider failure as execution failure, not generic provider search.
- Record provider plan, actual provider, graph generation, artifact/runtime identity, authority bound, selection reason, fallback reason, and outcome in provenance.
- Keep the resolver product-domain neutral and remove parallel registries or dispatch paths that duplicate the graph.

## Audit

Reconcile `plugin-contributions.md` and `plugin-resolution.md` against the implementation after the Rust-native plugin migration. Preserve already-correct code; add missing behavior or regressions rather than rewriting equivalent mechanisms.

## Completion

- contribution and resolution specs describe current behavior accurately;
- every completion criterion has concrete regression coverage or an explicit remaining feature owner;
- no broad architecture requirement remains `specification-only` solely because lifecycle metadata was stale;
- Source, Rust, Product, and Maintenance pass on the exact head;
- delete this temporary implementation slice before merge.
