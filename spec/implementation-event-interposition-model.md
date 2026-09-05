---
temporary: true
---

# Event and interposition model closure

## Goal

Make events, listeners, layers, hooks, controllers, and tasks distinct mechanisms with one kernel-owned implementation path for each semantic role.

## Required changes

- Treat an event as a fact that already occurred. Event listeners cannot reject, transform, or roll back the originating operation.
- Use service layers for synchronous pre-completion interposition, denial, transformation, wrapping, or delegation.
- Lower observation-only hooks to events/listeners and behavior-changing hooks to layers. Do not retain a privileged hook runtime.
- Give events stable type/version identity, emitter identity, causality, graph generation, and structural payload.
- Resolve listener dependency DAGs deterministically; reject cycles and bound causal same-listener re-entry.
- Allow independent listeners to run concurrently when the event contract permits it.
- Make delivery failure policy explicit and separate from operation success or rollback semantics.
- Keep recurring or multi-step behavior in controllers/tasks rather than listeners.
- Keep product event payload meaning outside Core.
- Route listener actions through ordinary host capabilities and service imports with normal authority checks.

## Audit

Reconcile `plugin-events.md`, service layering, hooks, controllers, and task behavior against the current implementation. Preserve equivalent mechanisms and remove duplicate semantic paths.

## Completion

- event versus layer semantics are mechanically distinct and tested;
- hooks are authoring sugar rather than a second runtime mechanism;
- `plugin-events.md` lifecycle metadata matches actual coverage;
- Source, Rust, Product, and Maintenance pass on the exact head;
- delete this temporary implementation slice before merge.
