# Phenix runtime architecture

Phenix is a persistent agent runtime assembled from a generic core, a generic conductor, selected plugins, client adapters, and product policy.

```text
frontends / protocol adapters
            |
            v
       phenix-client
 shared wire contracts
            |
            v
    phenix-conductor
 generic server process
            |
            v
       phenix-core
 generic mechanisms only
            |
            v
 selected plugin providers
            |
            v
 backend/provider adapters

phenix-harness
  product plugin selection
  grants and bindings
  runtime configuration
  skills and resources
```

The repository is prerelease. Replace obsolete contracts instead of preserving parallel compatibility APIs.

## Ownership

`phenix-core` owns generic mechanisms and trust boundaries:

- plugin identity, manifests, lifecycle, dependencies, activation, and shutdown;
- service registration and deterministic provider resolution;
- authority attenuation at every plugin boundary;
- namespaced plugin resources and durable schemas;
- persistence backend enforcement and transactions;
- generic events, subscriptions, blocking tasks, cancellation, and external-plugin isolation.

Core does not own sessions, context, skills, tools, executions, orchestrations, workers, objectives, plans, decisions, history, routing, models, language intelligence, frontend projections, hooks, jobs, or debugging semantics. Core must boot with no first-party services configured.

`phenix-client` owns the canonical client/server contract. It defines shared requests, responses, events, identifiers, capability descriptors, serialization, and protocol-version rules. Concrete adapters translate between this contract and external protocols.

`phenix-conductor` owns the generic server process. It owns process startup and shutdown, client transport, request dispatch, and hosting a configured core runtime. A zero-plugin conductor has no first-party fallback behavior.

First-party plugins own agent-domain behavior. Each plugin declares its manifest, services, authority ceiling, durable namespace, and migrations. First-party plugins use the same core contracts as alternate providers. Omitting a plugin removes its service unless another configured provider supplies that contract.

A thin plugin catalog may collect embedded factories. The catalog does not own plugin state, migrations, policy, or domain semantics.

`phenix-harness` owns supported product assembly. It selects the default plugins, grants, provider priorities, bindings, runtime configuration, skills, and resources. The `phenix` package is the supported Harness composition.

## Durable state

Durable agent-domain state belongs to the plugin that owns the semantic domain. Plugin schemas live in core-enforced namespaces. Core owns the persistence mechanism and transaction boundary but does not interpret first-party domain rows.

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

Plugins do not gain ambient filesystem, repository, network, IPC, secret, callable, or persistence authority. Resource-only plugins cannot execute code. External plugins receive only declared protocol and authority paths.

## Configuration and composition

Harness composition is explicit. Nix exposes independent first-party plugin packages through `phenixPlugins`, client adapters through `phenixClients`, and the public composition helpers through `wrappers.phenix.wrap`, `mkPhenix`, `mkPhenixPlugin`, and `mkPhenixClient`.

The default Harness uses those public package interfaces. Users may select a subset, add external or resource-only plugins, or replace a first-party service provider through the ordinary resolver.

Plugin configuration that changes semantics is immutable for the execution scope that uses it. Reload creates new semantic configuration rather than silently changing active execution meaning. Existing durable state keeps the exact identities needed to interpret prior work.

Omitting a provider removes its service. Core and conductor must not expose a hidden first-party fallback.

## Sessions and execution

Session trees, executions, orchestration, workers, objectives, plans, decisions, history, context, artifacts, tools, and related projections are plugin semantics. Their durable records and state transitions belong to their owning plugins.

The supported product reaches those domains through registered service contracts. It must not bypass provider resolution through compatibility registries or duplicate lookup tables.

Concurrency, retry lineage, authority attenuation, workspace consistency, exact references, context projection, verification, hooks, jobs, and worker handoff retain the behavioral contracts defined in the repository specs. The package split changes ownership and composition, not those semantics.

## Workspaces and language services

Workspace and language-intelligence behavior belongs to first-party plugins. Workspace operations still enforce explicit filesystem, repository, process, network, and IPC authority through core boundaries.

Language providers and frontend-linked services are adapters. Live process handles, connection identities, provider epochs, and notification state remain process-local. Durable observations are created only when an owning plugin records them as semantic state.

## Protocol and backend adapters

ACP is an interoperability adapter. `phenix-acp` translates ACP to the canonical `phenix-client` contract. ACP does not own Phenix sessions, routing, orchestration, tools, or durable state.

Backend/provider adapters translate between plugin-owned execution semantics and provider protocols. Provider conversation state is disposable. Phenix durable state remains canonical.

## Events, hooks, and jobs

Core supplies generic event delivery and blocking-task mechanisms. Hook definitions, hook policy, persistent jobs, execution lifecycle semantics, and their durable records belong to plugins.

Causal re-entry protection and authority attenuation apply when plugins handle events or schedule work. A hook, retry, or job cannot regain authority denied by its caller or configured maximum.

## Repository workers

Repository-driven worker selection and handoff are plugin semantics exposed through the ordinary service contract. The repository is the authoritative work queue. Worker state must not depend on chat history or a hidden core registry.

Worker tasks, results, verification, dependency ordering, and durable references use the same plugin-owned persistence and exact-reference rules as other agent-domain services.

## Migration rule

The package migration is complete only when one owner remains for each responsibility:

- `phenix-core` for generic runtime mechanisms;
- `phenix-client` for shared client/server contracts;
- `phenix-conductor` for the generic server process;
- one plugin for each agent-domain semantic area;
- `phenix-harness` for supported product policy and resources.

Remove old `phenix-kernel`, `phenix-protocol`, and `phenix-plugin-suite` package ownership once their responsibilities have moved. Remove transport-only artifacts before the merge gate.

Final verification must inspect the repository for duplicate registries, duplicate durable schemas, protocol bypasses, stale package aliases, stale architecture claims, and transitional APIs. Green CI alone does not prove ownership convergence.

## Specifications

Detailed contracts live under `spec/`. `spec/plugin-packages.md` defines the current package split and completion gate. Domain specs remain normative for session, context, execution, orchestration, worker, language, hook, job, and history behavior.
