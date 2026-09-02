# Conductor lifecycle hooks

status: implemented

## Contract

Hooks are immutable configuration-revision semantics attached to explicit conductor lifecycle events. Each hook declares an event, dependency ordering, action, and failure policy. Hooks for one event form a dependency DAG; configuration registration order does not determine execution order.

The supported lifecycle events are execution creation, execution completion, execution failure, context loading, callable start, and callable completion. The conductor resolves hooks from the configuration revision pinned to the affected execution.

Supported failure policies are `ignore`, `warn`, and `fail_operation`. `ignore` discards the hook failure. `warn` records a conductor event and allows the operation to continue. `fail_operation` returns the typed hook failure to the operation boundary. A veto is therefore effective only at a boundary that has not yet committed the transition or side effect it protects. Execution-creation hook failure leaves the already-created durable execution in the failed state rather than deleting history.

Same-hook recursive re-entry in one causal chain is blocked by default. Nested canonical operations may trigger other hooks, but an active hook cannot recursively invoke itself through a child execution, context load, or callable.

Hooks may observe, request canonical context injection, request a normal callable, veto explicitly supported operations, and emit metadata. Context requests use the normal exact-revision injection path. Callable and orchestration requests use the normal conductor callable APIs and inherit their authority, policy, lease, schema, and persistence checks. Hook metadata is a normal durable execution event. Hooks cannot directly mutate prompt state, bypass authority/leases/persistence, or perform privileged side effects outside normal conductor operations. Multi-step behavior invokes an orchestration rather than hiding another scheduler inside hooks.

Runtime resources such as currently connected frontend providers are not configuration semantics.

## Invariants

1. Hook definitions are pinned by immutable configuration revision.
2. Hook ordering is an explicit DAG and is independent of registration order.
3. Hooks cannot bypass execution authority, policy, leases, schemas, or durable ownership boundaries.
4. Recursive re-entry is controlled by causal hook identity.
5. Multi-step hook work uses canonical orchestration.
6. Runtime connection state is not part of hook configuration identity.
7. Hook-generated metadata and warnings use the canonical execution event stream rather than a parallel hook log.
