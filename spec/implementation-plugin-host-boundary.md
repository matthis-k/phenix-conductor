---
temporary: true
---

# Plugin host boundary closure

## Goal

Make `PluginHost` the single supported executable-plugin boundary and prove first-party and third-party capability parity.

## Required changes

- Pin graph generation and effective authority before provider code executes.
- Create one live-call scope per executable invocation and close it on success, failure, cancellation, crash, or disconnect.
- Expose only explicit host capabilities and resolved interface imports. Do not expose mutable kernel state, raw registries, persistence backend handles, SQL connections, or private first-party host APIs.
- Route callbacks back through ordinary graph dispatch and authority checks.
- Keep runtime-provider authority separate from guest-plugin authority.
- Normalize host, bridge, cancellation, and execution failures at the kernel boundary while preserving userspace typed domain errors.
- Keep process-local handles generation-bound and non-durable.
- Activate resource-only plugins without fake executable instances.
- Prove an alternate third-party implementation can request every host capability needed by an equivalent first-party plugin.

## Audit

Reconcile `plugin-host.md` against current Core and the Rust-native authoring model. Preserve existing correct mechanisms and add missing regressions or implementation only where required.

## Completion

- `plugin-host.md` accurately describes current behavior and has concrete coverage for its baseline invariants;
- product-domain behavior is absent from the generic host API;
- Source, Rust, Product, and Maintenance pass on the exact head;
- delete this temporary implementation slice before merge.
