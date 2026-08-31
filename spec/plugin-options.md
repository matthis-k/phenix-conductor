# Options plugin

## Status

Transitional implementation contract. Option storage, scoping, precedence, and resolution are implemented. Feature-specific definitions still need to move to their owning plugins as required by `spec/plugin-hygiene.md`.

## Purpose

Provide typed option state without moving product policy into Core or centralizing unrelated feature defaults.

`phenix.options` is an ordinary runtime plugin. It owns option definition registration, durable overrides, scope precedence, and resolution through `phenix.options@1`.

## Model

An option has:

- a parsed key;
- one typed default value;
- the scopes where callers may override it;
- the plugin or component that contributes the definition.

Supported value types are boolean, integer, and string.

Supported scopes are global, session, and agent. Session and agent scopes require a non-empty subject identity.

Scope values use these wire forms:

```json
"global"
{"session":"session-1"}
{"agent":"worker"}
```

Resolution order is:

```text
agent override
  -> session override
  -> global override
  -> definition default
```

A missing narrower override falls through. An override never mutates a broader scope.

## Commands

`phenix.options@1` supports:

```text
define
get_definition
set
unset
resolve
list
```

`define` is idempotent for an identical definition and rejects a conflicting redefinition. `set` rejects an unavailable scope or a value whose type differs from the active definition.

Option state is durable and owned by `phenix.options`. Core only provides the ordinary plugin persistence and authority mechanisms.

## Definition ownership

The options plugin must not contain a registry of defaults for unrelated features.

The component that owns behavior contributes its option definitions. Examples:

```text
sessions or session policy
  session.auto_create
  session.reuse_existing
  session.max_turns

model routing
  model.default

tool policy
  tools.confirmation

skill discovery
  skills.auto_load

context policy
  context.auto_load

worker or execution policy
  agent.max_parallel_tasks
```

Those names are examples of current first-party settings, not built-ins owned by `phenix.options`.

Disabling a feature removes its definitions from new resolved graph generations. Existing durable override rows may be preserved for later compatible reactivation, but an inactive definition cannot affect runtime behavior.

A feature may change its default only through a new configuration or implementation revision. Existing executions remain pinned to the graph generation that resolved their behavior.

## SDK interaction

SDK helpers may resolve options before calling ordinary runtime interfaces. The helper does not own the option definition.

For example, a session helper may resolve `session.reuse_existing` and `session.auto_create` only when the selected session implementation or policy component contributed those definitions.

## Startup settings

The wrapper may provide two startup sources: `settings.json` from `PHENIX_CONFIG_DIR`, and typed Nix settings materialized separately by `mkPhenix`.

Runtime `set` values win over startup sources. The preferred startup source is next, then the other startup source, then the active definition default. Within each source, scope precedence is `agent > session > global`.

`mkPhenix.settingsPrecedence` is `"nix"` by default. `"file"` gives `settings.json` precedence over Nix settings.

Startup sources are replaceable snapshots. Removing an entry removes that source's prior value. Runtime values stay separate and durable.

A startup value for an unknown or inactive definition is rejected or reported as unresolved. It must not create an implicit option definition.

## Invariants

- Core has no option registry or option semantics.
- `phenix.options` owns storage, scope, precedence, and resolution, not unrelated feature policy.
- Option keys are parsed before entering state.
- Values preserve their declared type.
- Scope precedence is deterministic.
- Narrower scopes only override broader scopes.
- Feature-specific definitions come from the feature that owns the behavior.
- Disabling a feature cannot leave its option behavior active through a central default registry.
- Reading or defining an option grants no execution authority.

## Required regressions

- agent overrides session, global, and default values;
- session overrides global and default values;
- global overrides the definition default;
- invalid keys fail parsing;
- wrong value types and disallowed scopes fail before state changes;
- identical definitions are idempotent and conflicting definitions fail;
- definitions from two unrelated plugins coexist without either implementation depending on the other;
- removing one feature removes its definitions from the next graph generation without deleting preserved durable overrides;
- the options implementation contains no hard-coded session, model, tool, skill, context, or worker default registry;
- SDK behavior changes only through definitions supplied by the owning feature.
