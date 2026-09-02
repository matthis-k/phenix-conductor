# Plugin provider resolution

status: implemented

## Purpose

Select providers for replaceable service/capability contracts using pinned kernel policy, runtime availability, and authority.

Permissions determine eligibility. Product policy determines preference among eligible providers.

Requires `spec/plugins.md` and `spec/plugin-contributions.md`.

## Domain

The kernel may define generic provider-resolution types:

```text
CapabilityId
CapabilityVersion
CapabilityRequirement
CapabilityProviderId
ProviderBinding
ProviderPriority
PluginPermissionGrant
CapabilityPermissionRequirement
ProviderAvailability
ResolvedCapabilityProvider
```

Capability identity is opaque to the kernel beyond contract/version matching. `artifact.read`, `session.open`, `model.complete`, and similar names remain userspace contracts.

## Configuration

Plugins advertise capability implementations. They do not assign their own effective priority.

Kernel/Harness policy supplies:

- enabled/disabled state;
- permission grant;
- integer priority;
- optional explicit binding;
- optional pre-dispatch fallback policy;
- opaque scope selectors when a contract needs scoping.

These values are pinned kernel configuration semantics.

## Effective permission

A provider is eligible only when the operation fits all bounds:

```text
caller authority
  ∩ configured plugin grant
  ∩ capability operation requirements
```

Priority, first-party status, bundling, origin, or explicit binding cannot expand authority.

## Resolution order

For one request:

1. resolve the pinned kernel policy/configuration snapshot;
2. match capability ID and compatible contract version;
3. discard disabled providers;
4. discard unavailable providers that cannot satisfy the request;
5. discard providers whose effective permissions do not satisfy the operation;
6. apply any declared opaque scope requirement;
7. if an explicit configured binding exists, select it when eligible;
8. otherwise select the highest configured per-capability priority;
9. break equal-priority ties by stable provider identity.

The resolver never invokes provider code while deciding which provider wins.

## Availability

Availability is runtime state. Configuration expresses preference, not current health.

At minimum:

```text
starting
ready
degraded
unavailable
stopped
```

`degraded` is eligible only when the provider explicitly reports that it can satisfy the requested operation.

## Fallback

Fallback may choose another eligible provider before dispatch.

After dispatch starts, the kernel does not transparently invoke another provider for the same logical operation unless the userspace capability contract explicitly declares safe idempotent replay semantics.

Mutating or ambiguity-sensitive contracts default to no post-dispatch provider switch.

## Selection provenance

The kernel records generic selection provenance:

- capability and contract version;
- selected provider identity;
- pinned kernel policy/configuration identity;
- effective permission bound;
- selection reason;
- provider runtime generation/epoch when relevant.

The owning userspace service decides how that provenance is associated with its domain records.

## Scope

The kernel supports generic scope keys/selectors where a contract requires them. It does not define workspace, session, execution, model, repository, or other product scope semantics.

A userspace contract defines the meaning and validation of its scope value. Scope never grants priority or authority by itself.

## Invariants

- Permissions determine eligibility; configured policy determines preference.
- First-party Phenix providers receive no implicit priority.
- Explicit binding never bypasses permission or availability checks.
- Same pinned policy, authority, availability, scope, and request produce the same selected provider.
- Provider selection cannot execute provider code.
- Runtime availability is not durable product semantics.
- Pre-dispatch fallback never broadens authority.
- Post-dispatch switching requires an explicit safe contract.
- The resolver understands no agent-domain semantics.

## Required regressions

- higher-priority unauthorized provider loses to an eligible lower-priority provider;
- explicit binding beats priority only when eligible;
- unavailable preferred provider falls back deterministically;
- equal-priority providers resolve by stable ID;
- priority/grant changes change pinned kernel policy identity;
- manifest metadata cannot self-promote a provider;
- resolver records exact selected provider and reason;
- mutating capability is not replayed after ambiguous post-dispatch failure;
- alternate third-party provider resolves through the same path as a first-party Phenix provider;
- kernel resolver requires no session/execution/workspace-specific type.