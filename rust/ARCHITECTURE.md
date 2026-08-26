# Phenix runtime architecture

Phenix is a persistent agent runtime assembled by `phenix-harness` from `phenix-kernel` and a selected plugin set.

```text
frontends / protocol adapters
            |
            v
      phenix-harness
      product policy
            |
            v
      phenix-kernel
 generic mechanisms only
            |
            v
 selected plugin providers
 Phenix Plugin Suite or alternatives
            |
            v
 backend/provider adapters
```

The repository is prerelease. Replace obsolete contracts instead of preserving parallel compatibility APIs.

## Ownership

`phenix-kernel` owns generic mechanisms and trust boundaries only:

- plugin identity, manifests, lifecycle, dependencies, and activation;
- service and capability registration plus deterministic provider resolution;
- authority attenuation at every plugin boundary;
- namespaced plugin resources and durable schemas;
- persistence backend enforcement and transactions;
- generic events, subscriptions, blocking tasks, cancellation, and external-plugin isolation.

The kernel does not own sessions, context, skills, tools, executions, orchestrations, workers, objectives, plans, decisions, history, routing, models, language intelligence, frontend projections, hooks, jobs, or debugging semantics. Kernel-only mode must boot without those services.

`phenix-plugin-suite` supplies the first-party implementations of those agent-domain services. First-party services use the same contribution, resolution, authority, persistence, event, and lifecycle contracts available to alternate plugins. A first-party service may be omitted or replaced without a kernel change.

`phenix-harness` owns product assembly and policy. It selects plugins, provider preferences, plugin configuration, and supported composition. The `phenix` package is the supported Harness composition.

`phenix-conductor` remains only as migration source and compatibility/test coverage while equivalent Harness/plugin paths are completed. It is not the supported product and must not acquire new domain ownership, persistence, or lookup paths.

## Durable state

Durable agent-domain state belongs to the plugin that owns the semantic domain. Plugin schemas live in kernel-enforced namespaces. The kernel owns the persistence mechanism and transaction boundary but does not interpret first-party domain rows.

A plugin restart reconstructs canonical state from its durable schema. Process-local handles, connections, caches, and provider generations are disposable and must not become durable identity.

Exact durable references retain their semantic identity across restart. A plugin may project or compact context, but compaction cannot replace or erase the underlying durable record.

## Authority

Authority only attenuates across boundaries.

```text
effective authority
  = caller authority
  ∩ provider maximum authority
  ∩ invocation restrictions
```

The same rule applies to direct service calls, plugin-to-plugin calls, retries, event handlers, blocking tasks, persistence operations, workspace operations, and external plugins. Runtime approval may authorize an operation inside the effective bound. It cannot expand that bound.

Plugins do not gain ambient filesystem, repository, network, IPC, secret, callable, or persistence authority. Resource-only plugins cannot execute code. External plugins receive only declared protocol and authority surfaces.

## Configuration and composition

Harness composition is explicit and inspectable. Nix composition may select the default Plugin Suite, select a subset, add external or resource-only plugins, or replace a first-party provider through the ordinary resolver.

Plugin configuration that affects semantics is immutable for the execution scope that uses it. Reload creates new semantic configuration rather than silently changing active execution meaning. Existing durable state keeps the exact identities needed to interpret prior work.

Omitting a provider removes its service. The kernel must not expose a hidden first-party fallback.

## Sessions and execution

Session trees, executions, orchestration, workers, objectives, plans, decisions, history, context, artifacts, tools, and related projections are Plugin Suite semantics. Their durable records and state transitions belong to their owning plugins.

The supported product reaches those domains through kernel service contracts. It must not bypass provider resolution through conductor registries or compatibility lookup tables.

Concurrency, retry lineage, authority attenuation, workspace consistency, exact references, context projection, verification, hooks, jobs, and worker handoff retain the behavioral contracts defined in the repository specs. The plugin migration changes ownership and composition, not those semantics.

## Workspaces and language services

Workspace and language-intelligence behavior belongs to first-party plugins. Workspace operations still enforce explicit filesystem, repository, process, network, and IPC authority through kernel boundaries.

Language providers and frontend-linked services are adapters. Live process handles, connection identities, provider epochs, and notification state remain process-local. Durable observations are created only when an owning plugin records them as semantic state.

## Protocol and backend adapters

ACP is an interoperability boundary, not an application runtime. `phenix-acp` owns ACP wire types and translation. ACP frontends reach the supported Harness composition; they do not own Phenix sessions, routing, orchestration, tools, or durable state.

Backend/provider adapters translate between plugin-owned execution semantics and provider protocols. Provider conversation state is disposable. Phenix durable state remains canonical.

Thin compatibility adapters may remain while an external protocol surface is migrated, but they must call the supported Harness/plugin path rather than maintain a second semantic runtime.

## Events, hooks, and jobs

The kernel supplies generic event delivery and blocking-task mechanisms. Hook definitions, hook policy, persistent jobs, execution lifecycle semantics, and their durable records belong to plugins.

Causal re-entry protection and authority attenuation apply when plugins handle events or schedule work. A hook, retry, or job cannot regain authority denied by its caller or configured maximum.

## Repository workers

Repository-driven worker selection and handoff are Plugin Suite semantics exposed through the ordinary kernel service contract. The repository is the authoritative work queue. Worker state must not depend on chat history or a hidden kernel registry.

Worker tasks, results, verification, dependency ordering, and durable references use the same plugin-owned persistence and exact-reference rules as other agent-domain services.

## Migration rule

The migration is complete only when the supported Harness path owns every product journey and no replaced conductor registry, durable table, product package, or compatibility lookup path remains authoritative.

During migration:

1. prove the Harness/plugin path with regressions;
2. move the supported product path to it;
3. remove or downscope the replaced conductor path;
4. keep one canonical API and one durable owner per semantic domain;
5. remove transport-only artifacts before the merge gate.

Green CI on an intermediate migration head does not prove ownership convergence. Final verification must inspect the repository for duplicate registries, duplicate durable schemas, protocol bypasses, stale architecture claims, and transitional APIs.

## Specifications

The detailed domain contracts live under `spec/`. In particular:

- `spec/plugin-implementation.md` tracks the kernel/plugin migration acceptance criteria;
- the architecture specs introduced by #398 define plugin hosting, authority, persistence, external plugins, and Harness composition;
- domain specs remain normative for session, context, execution, orchestration, worker, language, hook, job, and history behavior.

When this document and an older conductor-oriented description disagree, the kernel/userspace plugin ownership model above is authoritative for the current architecture.
