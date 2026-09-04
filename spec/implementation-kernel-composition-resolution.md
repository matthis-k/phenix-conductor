---
temporary: true
---

# Kernel composition and provider resolution closure

## Goal

Converge Plugin contributions, Provider selection, and Graph Generation binding onto one kernel-owned composition model.

## Required changes

- Use one canonical contribution representation derived from Plugin authoring declarations or runtime builders.
- Keep concrete Plugin dependencies distinct from Interface Imports.
- Resolve Providers during candidate Graph Generation construction, never by live search at invocation time.
- Check structural compatibility, version compatibility, Effective Authority, enablement, scope, and Product Composition Policy before activation.
- Let Effective Authority determine eligibility and Product Composition Policy determine preference.
- Make explicit Provider binding unable to bypass compatibility or Effective Authority.
- Break equal preference deterministically by stable identity.
- Resolve optional fallback plans only when the Interface permits fallback and pin them to the same Graph Generation.
- Treat post-dispatch Provider failure as execution failure, not generic Provider search.
- Record the resolved Provider plan, actual Provider, Graph Generation, artifact or Runtime Provider identity, Effective Authority bound, selection reason, fallback reason, and outcome in provenance.
- Keep the kernel resolver product-domain neutral and remove parallel registries or dispatch paths that duplicate the resolved graph.

## Audit

Reconcile `plugin-contributions.md` and `plugin-resolution.md` against the implementation after the Rust-native Plugin migration. Preserve already-correct code. Add missing behavior or regressions instead of rewriting equivalent mechanisms.

## Completion

- contribution and resolution specs describe current behavior accurately;
- every completion criterion has concrete regression coverage or an explicit remaining feature owner;
- no broad architecture requirement remains `specification-only` solely because lifecycle metadata is stale;
- Source, Rust, Product, and Maintenance pass on the exact head;
- delete this temporary implementation slice before merge.
