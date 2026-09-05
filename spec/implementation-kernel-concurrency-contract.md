---
temporary: true
---

# Kernel concurrency and cancellation closure

## Goal

Keep executor choice out of the canonical Phenix contracts while making long-running work, streaming, and cancellation explicit and testable.

## Required changes

- Keep kernel, PluginHost, interface metadata, runtime-provider contracts, and cross-plugin ABI free of executor-specific `Future`, async stream, Tokio, or equivalent types.
- Permit plugins and runtime providers to use async runtimes privately behind adapters.
- Run short kernel operations synchronously.
- Run long blocking I/O on dedicated or bounded blocking workers without holding broad kernel locks or persistence transactions.
- Use appropriate CPU workers for CPU-bound parallel work rather than blocking-I/O pools.
- Represent streaming as correlated typed messages at the Phenix boundary.
- Give every cancellable long-running call an explicit cancellation capability.
- Close live-call scopes on cancellation and reject late results from cancelled scopes.
- Prove unrelated kernel transitions continue while another worker is blocked.
- Use the canonical `bridged plugin runtime` terminology rather than treating `external` as a second semantic runtime class.

## Audit

Reconcile `plugin-threading.md` against Core, host, provider SDK, backends, and runtime bridges. Private Tokio use is acceptable when it does not leak into canonical contracts.

## Completion

- canonical contracts are executor-independent;
- cancellation and late-result semantics have regressions;
- blocked work cannot stall unrelated kernel progress through broad shared locks;
- `plugin-threading.md` lifecycle metadata matches actual coverage;
- Source, Rust, Product, and Maintenance pass on the exact head;
- delete this temporary implementation slice before merge.
