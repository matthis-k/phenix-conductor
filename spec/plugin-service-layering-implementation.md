# Layered service invocation implementation

Status: implementation plan for `spec/plugin-service-layering.md`.

## Dependency

Requires #403. Keep this PR stacked on `refactor/phenix-plugin-packages` until #403 merges.

## Architectural boundary

This PR must enforce the repository-wide boundary:

> `phenix-core` owns fundamental primitives and simple mechanisms. Plugins own non-trivial behavior, policy, discovery, management, composition, and product semantics.

Core is allowed to contain agent-specific primitives when most plausible plugins would otherwise need to reimplement them or depend on another plugin merely to express normal behavior. Keep those primitives minimal.

Expected core primitives include:

```text
model request/stream/tool-call contracts
tool identity/schema/registration/invocation
skill identity/content/registration/injection
minimal session identity/lifecycle/input/output
minimal context attachment/injection
plugin lifecycle, services, layering, continuations
authority, events, tasks, cancellation, persistence, transactions, provenance
```

Expected plugin-owned behavior includes:

```text
providers/auth/routing/model catalogs and policy
tool suites, discovery, MCP integration, tool-set policy
skill discovery/search/ranking/activation/catalog management
session trees, branching policy, summaries, history search, rich metadata
context discovery/ranking/compaction/repository context
orchestration, workers, planning, artifacts, hooks, jobs, diagnostics
```

Shared first-party use is not enough to move complex behavior into core.

For hooks, core owns only generic events/interception/layering/continuation/cancellation/authority/provenance. `phenix-plugin-hooks` owns configurable hook definitions, conditions, actions, persistence, pre/post domain semantics, and user-facing management. It must use ordinary plugin mechanisms without privileged registration or execution paths.

## Current behavior

The current core resolves one `ProviderBinding` per service. `PluginHost::invoke_service` resolves the service again for nested calls. `ServiceContribution` has no terminal/layer role. Same-service recursive invocation is rejected through the plugin call stack.

The external protocol supports handshake, invoke, result, error, and stop frames. It has no invocation-scoped continuation operation.

The current session plugin owns `parent`, `Children`, and child indexing inside `phenix.sessions@1`. This makes tree semantics part of the base session implementation instead of a replaceable layer/service.

The package split from #403 also treats several primitives too strongly as plugin-owned domains. This PR must correct that boundary where necessary instead of preserving "no agent concepts in core" as an invariant.

## Target

Implement the normative chain model from `spec/plugin-service-layering.md`:

```text
configured layers
  -> optional handle/deny
  -> explicit one-shot continuation
  -> one terminal provider
```

Provider replacement stays terminal-provider selection. Layering is a separate core mechanism.

At the same time, establish minimal core agent primitives and keep richer first-party behavior replaceable above them.

## Core primitive boundary

- [ ] replace any invariant that says core must contain no agent-domain concepts with the primitives/simple-mechanisms rule;
- [ ] identify the minimum model request, stream, completion, tool-call, capability, and cancellation contracts needed broadly by plugins;
- [ ] identify the minimum tool identity, schema, registration, invocation, result, and error contracts needed broadly by plugins;
- [ ] identify the minimum skill identity, descriptor/content, registration, resolution, and injection contracts needed broadly by plugins;
- [ ] identify the minimum session identity, lifecycle, input, output, and cancellation contracts needed broadly by plugins;
- [ ] identify the minimum context item/attachment/injection contracts needed broadly by plugins;
- [ ] keep provider/auth/routing, tool suites/discovery, skill management, session tree/search/summaries, and context ranking/compaction out of core;
- [ ] ensure first-party management plugins use the same primitives available to third-party plugins;
- [ ] ensure core stays usable without first-party model/tool/skill/session/context management plugins;
- [ ] remove or migrate duplicate primitive definitions that force ordinary plugins to depend on a first-party plugin for a fundamental contract;
- [ ] do not move non-trivial behavior into core merely to reduce dependency count.

## Core service-layer contract

- [ ] add an executable service role with `terminal` and `layer` variants;
- [ ] reject one plugin contributing the same service/version in both roles;
- [ ] keep resource-only plugins unable to contribute executable services;
- [ ] resolve a `ResolvedServiceChain` with ordered eligible layers and one terminal provider;
- [ ] preserve current no-layer resolution behavior;
- [ ] keep terminal binding independent from layer enablement/order;
- [ ] make required versus optional layer policy part of the pinned configuration identity;
- [ ] exclude unauthorized, unavailable, disabled, out-of-scope, and incompatible layers before dispatch;
- [ ] fail resolution when a required layer is absent or incompatible;
- [ ] never use registration or activation order as semantic layer order.

## Continuation

- [ ] give an invoked layer access to an opaque continuation for the remaining resolved chain;
- [ ] let a layer delegate once, return a result without delegation, or return an explicit denial;
- [ ] make continuation use single-shot and atomic so concurrent duplicate calls cannot advance twice;
- [ ] bind continuation to invocation identity, service/version, pinned policy, authority, and chain position;
- [ ] reject continuation reuse after the originating invocation returns;
- [ ] let a layer attenuate authority for delegation but never expand it;
- [ ] keep ordinary same-service recursive invocation rejected;
- [ ] make continuation advance the existing chain without re-running provider/layer resolution.

## Failure behavior

- [ ] layer error stops the chain;
- [ ] denial stops the chain with a distinct typed kernel result/error;
- [ ] terminal provider error stops the chain;
- [ ] no dispatched error selects another terminal provider;
- [ ] incompatible contribution means pre-dispatch skip, not runtime `unsupported` fallback.

## External plugin protocol

- [ ] version the external protocol for layered invocation;
- [ ] advertise contribution roles during handshake or derive them from the pinned manifest while validating the peer declaration;
- [ ] include an invocation-scoped continuation token only for layer calls;
- [ ] add a host continuation request/response path that advances one token exactly once;
- [ ] validate generation, request, invocation, service/version, authority, and remaining-chain position;
- [ ] reject stale, duplicated, forged, cross-service, or cross-generation continuation tokens;
- [ ] keep crash, timeout, and protocol failure as terminal failures for the current chain;
- [ ] add external-layer conformance coverage beside embedded-layer coverage.

## Provenance and inspection

- [ ] record the planned ordered layer list and selected terminal provider;
- [ ] record which participants were entered;
- [ ] record each layer outcome as handled, delegated, denied, or failed;
- [ ] record whether the terminal provider was reached and its result status;
- [ ] bind provenance to the pinned configuration and effective authority;
- [ ] expose enough inspection data for debug tooling to answer why a service call took a given path.

## Hooks proof

Use hooks as a boundary proof in addition to sessions.

- [ ] keep generic hook points as ordinary core events/service layers rather than a hook-specific privileged subsystem;
- [ ] keep configurable hook definitions, conditions, actions, persistence, and domain-specific pre/post semantics in `phenix-plugin-hooks`;
- [ ] make `phenix-plugin-hooks` consume only public core event/layer/authority/provenance mechanisms;
- [ ] prove removing `phenix-plugin-hooks` removes configurable hook behavior while generic events and layers remain usable;
- [ ] prove a replacement hook plugin can implement equivalent interception without first-party-only APIs.

## Session proof

Use sessions as the first real layered service. Keep the generic mechanism small.

- [ ] establish or preserve the minimal session primitive in core: stable identity plus basic lifecycle/input/output operations required broadly by plugins;
- [ ] keep richer session semantics outside core;
- [ ] reduce `phenix-plugin-sessions` to any richer replaceable session behavior that remains above the minimal primitive;
- [ ] remove parent/child indexing and `Children` ownership from the base session state/API where they exist only for tree semantics;
- [ ] add a focused first-party `phenix-plugin-session-tree` package;
- [ ] store lineage in the tree plugin's durable namespace keyed by base session identity;
- [ ] expose tree-specific queries through `phenix.session-tree@1`;
- [ ] layer the relevant `phenix.sessions@1` create path so configured lineage behavior composes with the base session mechanism/provider;
- [ ] keep ordinary clients able to use basic sessions without understanding the tree contract;
- [ ] prove an optional tree layer can be omitted while basic sessions still work;
- [ ] prove Harness policy can require the tree layer and fail closed when it is unavailable;
- [ ] package the tree plugin through the same `phenixPlugins` and `mkPhenixPlugin` path as other first-party plugins.

## Atomicity

The layer chain itself is not a transaction.

- [ ] identify session-tree mutations that require base-session and lineage state to commit atomically;
- [ ] use the existing kernel transaction contract when the current implementation can span the required namespaces;
- [ ] if the implementation cannot yet execute the normative cross-owner transaction contract, implement that generic gap in core rather than adding session-specific rollback;
- [ ] add failure injection proving no half-created child/lineage state remains when atomic semantics are required.

## Compatibility and migration

This repository is prerelease. Do not preserve superseded plugin APIs only for compatibility.

- [ ] migrate first-party `ServiceContribution` construction to explicit roles;
- [ ] migrate fundamental agent primitive contracts to their canonical core ownership when they are currently owned by a first-party plugin or duplicate domain crate;
- [ ] update first-party plugins to consume those core primitives rather than shadowing them;
- [ ] update tests, fixtures, external protocol fixtures, and Nix plugin metadata to the canonical contract;
- [ ] update session durable schema through an explicit migration when moving lineage or primitive ownership changes canonical stored state;
- [ ] preserve existing basic sessions and input history across the migration;
- [ ] do not silently reinterpret old parent/child bytes under a new owner;
- [ ] remove stale documentation asserting that core must be entirely agent-domain-neutral.

## Required regressions

- [ ] zero layers behaves like current single-provider invocation;
- [ ] deterministic two-layer delegation reaches one terminal provider;
- [ ] layer handle short-circuits lower participants;
- [ ] layer deny short-circuits lower participants;
- [ ] layer failure short-circuits lower participants;
- [ ] layer can transform a delegated result;
- [ ] incompatible optional layer is skipped before invocation;
- [ ] missing required layer fails resolution;
- [ ] explicit terminal binding keeps configured layers;
- [ ] continuation is one-shot under sequential and concurrent attempts;
- [ ] stale continuation is rejected;
- [ ] same-service recursion fails while continuation succeeds;
- [ ] delegated authority never expands;
- [ ] unauthorized layer is excluded before execution;
- [ ] terminal failure never dispatches another provider;
- [ ] planned and executed chain provenance is exact;
- [ ] embedded and external layer semantics match;
- [ ] core model/tool/skill/session/context primitives work without first-party management plugins;
- [ ] provider, routing, skill-management, tool-suite, session-tree, and context-management behavior remains independently replaceable;
- [ ] configurable hooks disappear when the hooks plugin is omitted while generic interception remains available;
- [ ] a replacement hook plugin works through the same public mechanisms;
- [ ] basic sessions work without the tree plugin;
- [ ] tree plugin adds lineage without changing base session identity;
- [ ] required tree policy fails closed;
- [ ] restart restores both basic session and tree-owned durable state.

## Documentation updates

- [ ] update architecture docs to state the canonical boundary: primitives/simple mechanisms in core, non-trivial behavior/policy in plugins;
- [ ] remove statements that make agent-domain neutrality itself a core invariant;
- [ ] make `spec/plugin-resolution.md` distinguish terminal provider selection from layer-chain resolution and point here for layer semantics;
- [ ] make `spec/plugin-contributions.md` list terminal and layer service roles;
- [ ] make `spec/plugin-sessions.md` distinguish the minimal core session primitive from richer session plugins and tree semantics;
- [ ] make hook documentation distinguish generic core interception mechanics from `phenix-plugin-hooks` behavior;
- [ ] keep `spec/plugin-service-layering.md` as the single source of truth for layer semantics and this core/plugin boundary until the broader architecture docs are migrated.

## Validation

- [ ] formatting/static source checks;
- [ ] Clippy across the Rust workspace;
- [ ] all Rust unit tests in one run;
- [ ] doc tests;
- [ ] integration/system tests;
- [ ] Product session journeys with and without the tree layer;
- [ ] Product/core primitive journeys with first-party management plugins omitted;
- [ ] Nix default, omitted-tree, required-tree, alternate-layer, and omitted-hooks compositions;
- [ ] replacement hook plugin composition using only public interfaces;
- [ ] external plugin protocol tests;
- [ ] Maintenance checks;
- [ ] Maintenance autofix with a clean resulting tree.

## Completion rule

The PR is complete when the core/plugin boundary follows the primitives/simple-mechanisms rule, fundamental model/tool/skill/session/context primitives no longer require first-party behavior plugins, the service path no longer assumes one resolved provider, continuation is enforced by core rather than plugin convention, external and embedded layers match, configurable hooks are ordinary plugin behavior over generic core interception mechanisms, and the default session tree is a plugin layer/service over a working basic session primitive/provider. Checked items require code and test evidence on the final rebased head.
