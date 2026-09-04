---
temporary: true
---

# Persistence provider bootstrap closure

## Goal

Converge persistence onto one generic kernel mechanism with explicit bootstrap, plugin-owned durable resources, and replaceable backend providers.

## Required changes

- Keep the baseline local backend generic and product-domain neutral.
- Derive plugin durable schemas from canonical resource declarations; static plugins must not register namespaces manually during startup.
- Keep namespace isolation, schema registration, migration ordering, transactions, recovery validation, and authority in the kernel.
- Never expose SQL, database connections, or backend-native handles to plugins.
- Resolve alternate persistence providers before opening the target store.
- Reject persistence bootstrap dependency cycles before activation.
- Negotiate required generic backend features before store open.
- Require explicit compatible storage format, migration/export-import, or a new store binding when changing providers.
- Support atomic transactions spanning authorized mutations in multiple namespaces when one declared operation requires it.
- Keep persistence-provider authority separate from product-domain authority.
- Preserve the active generation when a persistence candidate cannot prepare safely.

## Audit

Reconcile `plugin-persistence.md` and durable-resource authoring against current Core. Keep FTS, embeddings, memory search, graph traversal, replication, archival, and product query policy outside generic persistence.

## Completion

- persistence bootstrap and durable-resource ownership have one canonical implementation path;
- alternate mock backend conformance is covered;
- `plugin-persistence.md` lifecycle metadata matches actual coverage;
- Source, Rust, Product, and Maintenance pass on the exact head;
- delete this temporary implementation slice before merge.
