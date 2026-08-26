# Phenix plugin package implementation

Status: implementation contract.

## Purpose

Finish the split started by #399. Keep the generic runtime small. Move first-party agent behavior into independently packaged plugins. Make the supported Harness a wrapped conductor configuration assembled through the same public package interfaces available to users.

This repository is the future `phenix-ai` repository. It owns the core runtime, conductor, shared client contracts, first-party plugins, client adapters, and the supported Harness configuration.

## Dependency

Requires:

- #399 merged into `main`.

Supersedes:

- #401, which closed with no implementation commits.
- #402, whose branch retained stale pre-squash #399 history.

Workers must implement this PR from current `main`. Do not rebuild the old #401 or #402 branch history.

## Target ownership

```text
phenix-core
  generic plugin runtime and trust-boundary mechanisms

phenix-client
  canonical client/server contracts and client-side interface library

phenix-conductor
  generic server process built on phenix-core + phenix-client contracts

phenixPlugins.<system>.*
  independently packaged first-party plugins

phenixClients.<system>.*
  independently packaged protocol/client adapters

phenix-harness
  wrapped phenix-conductor configuration + default plugins + product policy/resources
```

The normal `phenix` package is the supported Harness composition.

## Core invariants

- `phenix-core` owns mechanisms, not agent-domain behavior.
- First-party plugins use the same registration, authority, persistence, event, task, and service contracts as replacement plugins.
- First-party status grants no hidden authority, priority, persistence access, or fallback path.
- The conductor loads only configured plugins.
- A zero-plugin conductor exposes no first-party session, artifact, context, tool, routing, planning, worker, or model behavior.
- Plugin durable state belongs to the owning plugin namespace and migrations.
- Plugin omission removes the owned service unless another configured provider supplies the same contract.
- The Harness selects product defaults. The conductor and core do not.
- Actual clients and protocol adapters are separate packages. Shared client/server contracts live in `phenix-client` rather than in each adapter.
- The repository stays prerelease. Remove superseded APIs instead of maintaining parallel compatibility paths.

## `phenix-core`

Move the current `phenix-kernel` implementation into `phenix-core` and remove `phenix-kernel` as a competing public package.

`phenix-core` owns:

- plugin identity, manifests, lifecycle, dependencies, activation, and shutdown;
- authority attenuation and trust-boundary enforcement;
- generic service registration and deterministic provider resolution;
- events, subscriptions, causal re-entry protection, and failure policy;
- runtime tasks, cancellation, and blocking execution mechanisms;
- plugin persistence namespaces, schemas, migrations, transactions, and backend contracts;
- embedded, external executable, and resource-only plugin hosting;
- generic provenance and policy data required to enforce those mechanisms.

Move agent-domain types out of `phenix-core` unless a type is required by the generic plugin host itself.

Completion tests:

- core boots with zero first-party plugins;
- no first-party plugin crate is a dependency of core;
- denied authority cannot be regained through retries, delegation, events, tasks, or persistence;
- persistence never interprets plugin-domain rows.

## `phenix-client`

Create one canonical shared package for the Phenix client/server contract. This is not a frontend implementation.

It owns:

- client request and server response types;
- server events and notifications;
- shared protocol identifiers;
- capability and service descriptors visible to clients;
- serialization and protocol-version rules;
- client-side interfaces needed by concrete client packages.

Migrate the relevant parts of `phenix-protocol` and client-visible types currently living in other crates into this package. Remove `phenix-protocol` once ownership has moved.

The conductor may depend on these shared contract types. Concrete clients depend on the same package. `phenix-client` must not depend on the Harness or concrete first-party plugins.

## `phenix-conductor`

Restore `phenix-conductor` as the generic server process on top of `phenix-core` and `phenix-client`.

It owns:

- process startup and shutdown;
- configured plugin loading;
- client connection/session transport;
- dispatch between client contracts and configured services;
- generic runtime configuration needed to host the selected plugins.

It does not own product-default plugin selection or agent-domain fallback implementations.

Required regressions:

- conductor starts with zero plugins;
- conductor starts with exactly one configured plugin;
- omission removes that service;
- replacement uses the same service contract without conductor changes.

## Phenix Plugins package set

Delete `phenix-plugin-suite` as the implementation owner. Split its domains into focused plugin packages.

Expose each first-party plugin through:

```nix
phenixPlugins.${system}.<name>
```

Initial ownership should cover the current suite domains:

- session and session tree;
- artifact storage, readers, reuse, and invalidation;
- context;
- skills and resources;
- tools and callable execution;
- orchestration;
- workers and repository handoff;
- planning, objectives, decisions, and history;
- workspace and repository services;
- default CLI services;
- model, provider, auth, and routing services;
- language intelligence;
- frontend-facing projections/services where these are server-side plugin semantics;
- hooks;
- jobs;
- debugging and diagnostics.

Combine domains only when they have one real ownership boundary. Do not combine them only to reduce package count.

Each plugin must:

- have its own package or crate ownership;
- declare its own manifest and service contributions;
- own its durable namespace and migrations;
- build and test without loading unrelated first-party plugin implementations;
- be independently selectable, omittable, and replaceable;
- use ordinary core contracts without a first-party-only registration path.

A thin catalog may collect embedded factories. The catalog must not own plugin semantics, state, migrations, or policy.

## Client packages

Package concrete client/protocol adapters independently.

Expose repository-owned adapters through:

```nix
phenixClients.${system}.<name>
```

ACP should be one such adapter. Keep ACP translation separate from the canonical `phenix-client` contract.

A concrete client package may implement transport, protocol translation, UI integration, or adapter-specific behavior. It must not become the owner of conductor semantics.

## Harness

`phenix-harness` is a wrapped configuration of `phenix-conductor`. It is not a second server implementation.

It owns:

- the supported first-party plugin selection;
- grants, provider priorities, bindings, and product policy;
- default runtime configuration;
- first-party skills and product resources;
- supported client/adapter selection where the product needs it.

Build it through the public wrapper path:

```nix
phenix-harness = wrappers.phenix.wrap {
  conductor = packages.${system}.phenix-conductor;
  plugins = [
    phenixPlugins.${system}.session
    phenixPlugins.${system}.artifact
    # ... default product selection
  ];
};
```

The normal `phenix` package aliases this supported Harness composition.

## Consolidate the external Harness repository

Move all unique product configuration and resources from `matthis-k/phenix-harness` into this repository.

At minimum migrate:

```text
config/phenix/runtime.nix
config/phenix/skills/**
```

Preserve licenses and attribution files for imported skills and resources.

Completion requires:

- no build dependency on `matthis-k/phenix-harness`;
- no runtime dependency on it;
- no Product or integration test dependency on it;
- no unique product configuration or resource remains there;
- deleting the external repository would not change supported Phenix behavior.

## Nix contract

Expose at least:

```text
packages.<system>.phenix-core
packages.<system>.phenix-client
packages.<system>.phenix-conductor
packages.<system>.phenix-harness
packages.<system>.phenix
phenixPlugins.<system>.<plugin>
phenixClients.<system>.<client>
wrappers.phenix.wrap
lib.mkPhenixPlugin
lib.mkPhenixClient
lib.mkPhenix
```

Nix composition must prove:

- every default first-party plugin is independently buildable;
- a one-plugin conductor composition works;
- plugin omission removes its service;
- an alternate provider can replace a first-party plugin through the same contract;
- the default Harness uses the same public package-set and wrapper path;
- selecting a plugin does not require the monolithic first-party suite implementation.

## Migration order

1. Establish the final `phenix-core`, `phenix-client`, and `phenix-conductor` ownership boundaries.
2. Split `phenix-plugin-suite` into independently owned plugin packages.
3. Split concrete client/protocol adapters into independent client packages.
4. Expose `phenixPlugins` and `phenixClients` package sets.
5. Build the supported Harness only through the public wrapper/composition path.
6. Move the external Harness configuration, skills, licenses, and resources into this repository.
7. Route Product, ACP, integration, and system product tests through the in-repository Harness.
8. Remove old package aliases, duplicate registries, duplicate durable state, stale migration paths, and external Harness dependencies.
9. Review module/crate boundaries and split oversized files by one clear responsibility.
10. Run exact-head Source, Rust, Product, Nix composition, Maintenance, and Maintenance-autofix validation.

## Completion gate

The PR is complete only when:

- `phenix-core` is the single generic plugin-runtime owner;
- `phenix-client` is the single canonical shared client/server contract owner;
- `phenix-conductor` is a generic configured server with no hidden product defaults;
- `phenix-kernel`, `phenix-protocol`, and `phenix-plugin-suite` no longer compete as public implementation owners;
- each first-party plugin is independently packaged and selectable through `phenixPlugins`;
- each repository-owned client adapter is independently packaged through `phenixClients`;
- the supported Harness is a wrapped conductor assembled through the public package interfaces;
- the repository contains all product configuration/resources required by the supported Harness;
- the separate `phenix-harness` repository is safe to delete;
- full exact-head validation is green;
- the repository is ready to rename from `phenix-conductor` to `phenix-ai`.
