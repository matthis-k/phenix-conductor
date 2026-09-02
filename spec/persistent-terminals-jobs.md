# Persistent terminals and jobs

status: implemented

## Contract

Terminals and jobs are conductor-owned runtime resources with durable identity and metadata. Execution-owned terminals/jobs are the default. When the owner execution reaches a terminal state, its running execution-owned terminals and jobs are revoked. Longer-lived workspace jobs require explicit promotion before that transition.

Persist durable metadata, output/history references, ownership, authority provenance, lifecycle, and promotion state. Do not persist raw process handles or PTY objects as durable state. Runtime process and PTY handles, when present, are keyed by the durable terminal/job identity and are reconstructed only by live runtime integrations; journal or SQLite restore must never recreate them.

Effective capability is bounded by authority at creation and current execution authority. Authority narrowing must revoke or invalidate incompatible terminal/job capability rather than preserving stale privilege.

Managed language servers remain workspace-scoped infrastructure and are not modeled as ordinary execution terminals/jobs.

## Invariants

1. Process handles are runtime-local; durable identity/metadata is canonical.
2. Execution ownership is the default lifetime; terminal execution revokes running execution-owned resources.
3. Workspace promotion is explicit and durable; promoted jobs survive creator execution termination.
4. Authority can only attenuate over resource lifetime.
5. Narrowed authority revokes incompatible capability.
6. Managed LSP processes remain a distinct workspace service abstraction.
