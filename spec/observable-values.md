---
status: proposed
---

# Observable Phenix values

## Purpose

Provide one language-neutral mechanism for observing structured `PhenixValue` state without turning each field mutation into a Plugin Event.

Plugins own the meaning and persistence of their state. Core owns generic value identity, atomic commit, subscription matching, versioning, and change delivery. Language bindings project the same contract into Lua, Python, JavaScript, and later clients.

The mechanism is optimized for UI and adapter state that changes often. It must avoid whole-tree cloning, deep diffing, per-listener copies, and internal serialization unless the selected snapshot policy or a transport boundary requires them.

## Boundary

`PhenixValue` remains the structural interchange value. It does not gain listener state or mutable identity.

Core adds an observable value store. A registered value has:

```text
ValueId
owner PluginId
schema PhenixSchema
version ValueVersion
snapshot policy
current PhenixValue
```

A value reference is a normal capability reference. The SDK may expose an `ObservableRef` newtype over `ObjectRef` with contract `phenix.observable@1`; no new `PhenixValue` variant is required.

The store is runtime state. Durable product state remains owned by the Plugin that registered the value.

## Addressing

Subscriptions address one registered root plus an optional structural path.

```text
ValueAddress {
  value: ValueId
  path: ValuePath
}
```

`ValuePath` uses typed segments rather than runtime strings where possible:

```text
field Key
map_key String
index u32
variant_payload
option_payload
```

Bindings may accept language-native strings and indexes, then compile them once when the subscription is created. Hot-path delivery must not rebuild or parse path strings.

A path is interpreted against the registered root schema. Invalid paths fail subscription creation.

## Subscription scope

A subscription chooses one scope:

```text
exact
recursive
```

**Exact** observes replacement, creation, or removal of the addressed value. Descendant mutations do not match. Replacing an ancestor still matches because it replaces or removes the addressed value.

**Recursive** observes the addressed value and every descendant. Replacing an ancestor also matches because the observed subtree changes as a unit.

Subscription matching uses an index keyed by `ValueId` and compiled path. Delivery must not scan every subscription for each mutation.

## Delivery mode

A subscription chooses one delivery mode:

```text
diff
full
```

**Diff** receives the canonical change set for the observed address relative to that address.

**Full** receives the committed value at the observed address for the resulting version.

Both modes carry:

```text
CommitId
ValueId
from_version
version
address
```

A subscription receives at most one delivery for one commit. Ten matching writes in one transaction still produce one delivery to that subscription.

A commit whose final observed value equals its pre-transaction value produces no delivery for that subscription.

## Canonical changes

Transactions record mutations while they happen. Core does not compute diffs by comparing complete old and new trees after commit.

The first contract should support:

```text
replace path value
remove path
splice path start delete_count inserted_values
```

`splice` covers list insertion, removal, and replacement without replacing an entire list.

The transaction canonicalizes overlapping writes before delivery:

- repeated writes to one path collapse to the final write;
- replacing an ancestor removes redundant descendant changes;
- a write followed by restoration of the original value disappears from the committed change set;
- list edits may coalesce when doing so preserves the same final state and ordering.

The committed change set is immutable.

## Atomic transactions

All observable mutations go through a transaction boundary.

```rust
values.transaction([sessions, routing], |tx| {
    tx.splice(sessions, path!["items"], 4, 1, [replacement])?;
    tx.replace(routing, path!["selected"], profile)?;
    Ok(())
})?;
```

The transaction publishes one `CommitId` after every participating root has reached its final state.

Listeners cannot observe an intermediate state. A callback that reads any root changed by the same commit sees the committed state for that commit or a later commit, never a partial commit.

Locks for multi-root commits use deterministic `ValueId` order. The public API should allow callers to declare touched roots up front so the common path does not need an allocating dynamic lock set.

If the transaction fails, no version changes and no listener receives a delivery.

## Versions

Each changed root increments its `ValueVersion` once per commit. Unchanged roots keep their version.

`CommitId` orders roots changed atomically together. `ValueVersion` detects stale per-root delivery.

Bindings and remote adapters must surface both. A consumer that detects a gap can request a full value instead of attempting to apply an incomplete diff.

## Snapshot policy

Storage policy is separate from listener delivery mode.

Initial policies:

```text
current_only
copy_on_change
```

**Current only** keeps no prior full snapshot solely for observation. It is suitable for synchronous internal observation or states whose transport adapter explicitly materializes its own payload.

**Copy on change** creates one stable committed snapshot when the registered root changes. All listeners and adapters share that snapshot. Copying occurs once per changed root per commit, never once per listener.

A later retained-history policy may keep a bounded number of committed snapshots, but the first implementation must not require history to support ordinary observation.

Requesting a delivery guarantee that the selected snapshot policy cannot provide must fail explicitly. The runtime must not add an unrequested hidden deep copy.

## Allocation and copy rules

The internal hot path targets zero heap allocations for a small transaction when the selected snapshot policy does not require a snapshot and the mutation payload already owns its values.

The implementation should use inline storage for small mutation and matched-subscription sets, with heap spill only when they outgrow the inline capacity.

The following rules are part of the contract:

- no internal serialization for local observation;
- no whole-state clone for `diff` delivery;
- no post-commit deep diff;
- no snapshot copy per listener;
- no path-string allocation during delivery;
- no notification for a no-op final state;
- one subscription delivery at most per commit;
- foreign-language materialization happens at the binding or transport boundary;
- a stable copy required by `copy_on_change` is shared by every consumer of that committed version.

An implementation may reuse container allocations during transformations. The contract does not require persistent immutable trees or structural sharing.

## Internal and foreign listeners

Core exposes committed changes in native form. The generic Plugin EventBus is not the observation transport.

The existing EventBus carries Plugin Event semantics such as authority checks, causal ancestry, listener dependency DAGs, asynchronous admission, receipts, and failure policy. Observable state does not pay those costs for each value change.

A Plugin may explicitly bridge a committed observable change into a Plugin Event when the event semantics are useful.

Language and protocol bindings consume the same committed observation contract. They may queue compact native change handles and materialize language values lazily.

For example, Lua may expose:

```lua
local stop = phenix.sessions.state:listen({
  scope = "recursive",
  mode = "diff",
}, function(change)
  -- `change` may remain native userdata until fields are requested.
end)
```

A full listener may expose:

```lua
phenix.sessions.state:listen({
  scope = "recursive",
  mode = "full",
}, function(state)
  render(state)
end)
```

Lua tables, JavaScript objects, Python objects, ACP extension payloads, and other foreign representations may allocate when materialized. Core should avoid that work until the consumer crosses that boundary.

## SDK contributions

Plugin SDK contributions may expose observable resources alongside typed interfaces and client helpers.

A generated binding resolves the SDK namespace and resource metadata, then returns a language-native observable handle. Plugin-specific Lua, Python, or JavaScript glue is not required for ordinary get, listen, transaction, or unsubscribe behavior.

The default API plugin may expose session, model, routing, skill, execution, and similar read models this way without owning their underlying product state.

## Concurrency

A committed change is immutable after publication.

Subscription creation and removal are safe during concurrent commits. A subscription has a defined admission point. It receives either commits admitted after that point or a requested initial full state followed by later commits.

Unsubscription prevents future admission. Already admitted foreign-language deliveries may remain in that binding's bounded queue and carry the subscription generation so stale deliveries can be discarded without invoking user code.

Listener callbacks do not run while value-store mutation locks are held.

## Initial state

Subscription creation may request:

```text
initial = none
initial = full
```

`initial = full` returns or admits one current full value with its current version before later commits for that subscription. The implementation must define the admission boundary so no commit can be lost between the initial snapshot and subscription activation.

## Errors

The contract uses typed errors for at least:

```text
unknown_value
invalid_path
schema_mismatch
stale_reference
unsupported_snapshot_policy
transaction_conflict
subscription_capacity
closed
```

Bindings preserve the error kind instead of reducing it to a display string.

## Performance expectations

For a commit touching `k` canonical paths, matching cost should scale with the touched path depth and the number of matching subscriptions, not with total store size or total subscription count.

For the common small transaction with `current_only` state and local listeners, the target is:

```text
mutation
-> inline change log
-> canonicalization
-> version bump
-> indexed subscription match
-> borrowed/shared committed change
-> listener
```

No serialization and no whole-state copy occur on this path.

`copy_on_change` adds at most one stable snapshot copy per changed root per commit. Cross-language conversion adds cost only at the language or protocol boundary.

## Required regressions

- exact subscription ignores descendant-only mutation;
- exact subscription fires when its addressed value or an ancestor is replaced;
- recursive subscription fires for descendant mutation;
- one subscription receives one delivery for many matching writes in one commit;
- repeated writes to one path collapse to the final change;
- write then restore produces no delivery;
- multi-root transaction publishes one `CommitId` and exposes no partial state;
- failed transaction changes no version and emits nothing;
- `diff` delivery comes from the transaction mutation log rather than a whole-tree comparison;
- `copy_on_change` copies at most once per changed root regardless of listener count;
- `current_only` local small mutation has an allocation-free tested path where the mutation payload itself requires no allocation;
- full and diff listeners may observe the same commit without duplicate snapshot copies;
- subscription matching does not scan unrelated values or subscriptions;
- initial full state and later changes have no admission gap;
- unsubscription prevents later callback admission;
- callbacks run after mutation locks are released;
- foreign-language queues can discard stale subscription generations;
- Lua, Rust, and one protocol-level regression observe equivalent value versions and change semantics;
- ordinary value observation does not route through the Plugin EventBus.
