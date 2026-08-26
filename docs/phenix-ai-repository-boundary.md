# Phenix AI repository and package boundary

## Target

The current `matthis-k/phenix-conductor` repository becomes the complete Phenix AI source repository. Rename the GitHub repository to `phenix-ai` only after the migration stack is merged and validated.

The final repository owns four distinct roles:

```text
phenix-core
  generic plugin runtime and trust-boundary mechanisms

phenix-api
  shared client/server contracts

phenix-conductor
  generic server process built on phenix-core + phenix-api

phenix-harness
  wrapped configuration of phenix-conductor + selected plugins + product policy
```

First-party agent behavior lives in independently packaged plugins collected by the Phenix Plugins package set.

The separate `matthis-k/phenix-harness` repository is migration input only. After this work lands, deleting that repository must not remove unique product configuration, skills, licenses, or behavior.

## Final repository model

```text
phenix-ai
├── phenix-core
├── phenix-api
├── phenix-conductor
├── first-party plugin packages
├── Phenix Plugins package set
├── phenix-harness configuration/wrapper
├── ACP and other protocol adapters
└── product tests and documentation
```

The repository boundary does not weaken package boundaries. `phenix-core` and `phenix-api` must not depend on concrete first-party plugins.

## `phenix-core`

`phenix-core` owns generic mechanisms only:

- plugin identity, manifests, lifecycle, activation, and dependencies;
- authority enforcement and isolation;
- generic service registration and provider resolution;
- events, subscriptions, tasks, and cancellation;
- persistence backend contracts, namespaces, schemas, migrations, and transactions;
- embedded, external, and resource-only plugin hosting.

The current `phenix-kernel` implementation migrates into this role. Remove `phenix-kernel` as a second public package when that move is complete.

The current `phenix-core` crate contains agent-domain types. Reassign them by ownership:

- client-visible request, response, event, descriptor, and identifier contracts move to `phenix-api`;
- plugin-owned domain types move to the plugin that owns the domain;
- types required only by generic plugin/runtime mechanisms stay in `phenix-core`.

`phenix-core` must boot without first-party agent plugins. It must not provide session, artifact, context, routing, tool, worker, planning, model, or other agent-domain fallbacks.

## `phenix-api`

`phenix-api` defines the contract clients use and the server implements. It is not a client implementation.

It owns:

- client/server request and response types;
- server event and notification types;
- client-visible capability and service descriptors;
- shared identifiers that are part of the client contract;
- serialization rules and canonical Phenix API versioning.

The current `phenix-protocol` crate migrates into `phenix-api`. Move types from the old `phenix-core` here only when clients need those types.

`phenix-api` must not depend on `phenix-harness` or concrete first-party plugins.

ACP is an adapter to the canonical API, not the canonical API itself. Keep ACP-specific translation in its own package. The same rule applies to future transport adapters.

## `phenix-conductor`

`phenix-conductor` is the generic server process. It depends on `phenix-core` and `phenix-api`, hosts configured plugins, and exposes the configured API to clients.

The conductor must not encode the Phenix product's default plugin selection. Starting it with zero configured first-party plugins must not silently restore product behavior.

The conductor may host embedded, external, and resource-only plugins. Hosting form does not change plugin ownership.

The package remains named `phenix-conductor` after the GitHub repository is renamed to `phenix-ai`.

## Phenix Plugins package set

Every first-party plugin is independently buildable and selectable. The package set groups those packages; it does not replace them with one implementation package.

Expose a Nix package set such as:

```nix
phenixPlugins.${system} = {
  session = ...;
  artifact = ...;
  context = ...;
  skills = ...;
  tools = ...;
  orchestration = ...;
  workers = ...;
  planning = ...;
  workspace = ...;
  cli = ...;
  models = ...;
  language = ...;
  frontend = ...;
  hooks = ...;
  jobs = ...;
  debug = ...;
};
```

The exact split may change when two domains prove to have one ownership boundary. Independent packaging and selection are mandatory.

Cargo package names may use names such as `phenix-plugin-session`. The primary Nix interface is the package set, so users select `phenixPlugins.${system}.session` instead of depending on flat public package names.

The current `phenix-plugin-suite` crate is transitional. Split its modules into independent plugin packages. A thin internal factory catalog may exist for an embedded build, but it must not own plugin semantics, durable state, or configuration.

Each plugin must:

- declare its own manifest and service contributions;
- own its durable namespace and migrations;
- build and test without loading unrelated first-party plugin implementations;
- be omittable without exposing a conductor fallback;
- be replaceable through the same service contract.

## `phenix-harness`

`phenix-harness` is a wrapped configuration of `phenix-conductor`. It is not a second server/runtime implementation.

Conceptually:

```nix
phenix-harness = wrappers.phenix.wrap {
  conductor = packages.${system}.phenix-conductor;
  plugins = [
    phenixPlugins.${system}.session
    phenixPlugins.${system}.artifact
    phenixPlugins.${system}.context
    # default first-party selection
  ];
  # default grants, bindings, settings, and resources
};
```

Harness owns product defaults:

- the default first-party plugin selection;
- grants, priorities, bindings, settings, and product policy;
- the default runtime configuration;
- first-party skills and other resources shipped by the supported product.

The public `packages.${system}.phenix-harness` result must come from the same wrapper/configuration path available to users.

The normal user-facing `phenix` package may alias the Harness result.

The current Rust `phenix-harness` crate must not remain a second owner of server semantics. Remove it if the wrapper no longer needs it. If embedded factory linking needs a small build-time helper, keep that helper narrow and do not make it the semantic meaning of Harness.

## Nix outputs

The future `phenix-ai` flake should expose at least:

```text
packages.<system>.phenix-core
packages.<system>.phenix-conductor
packages.<system>.phenix-harness
packages.<system>.phenix
phenixPlugins.<system>.<plugin>
wrappers.phenix.wrap
lib.mkPhenixPlugin
lib.mkPhenix
```

`phenixPlugins.<system>.<plugin>` values are independent plugin derivations.

`wrappers.phenix.wrap` accepts those derivations directly and produces a configured conductor. Product defaults use the same interface.

A core-only or zero-plugin test configuration is explicit. Do not retain `phenix-kernel` as a competing public name.

## Current-to-final mapping

| Current owner | Final owner |
| --- | --- |
| `phenix-kernel` | `phenix-core` generic mechanisms |
| domain-heavy parts of current `phenix-core` | `phenix-api` or owning plugin |
| `phenix-protocol` | `phenix-api` |
| `phenix-conductor` | generic server process |
| `phenix-plugin-suite` | independent `phenixPlugins.*` packages |
| public Rust Harness runtime ownership | remove or downscope; Harness is wrapper/configuration |
| external `matthis-k/phenix-harness` config/resources | in-repository Harness configuration/resources |

Do not preserve old names through long-lived compatibility packages. This repository is prerelease and should converge on one ownership model.

## Consolidate the external Harness repository

The current `matthis-k/phenix-harness` repository already wraps `phenix-conductor`. Its unique product data must move here.

Migrate at least:

```text
config/phenix/runtime.nix
config/phenix/skills/**
```

Preserve all licenses and attribution associated with imported skills and resources.

After migration:

- the supported Harness package comes from this repository;
- the default runtime configuration comes from this repository;
- first-party skills used by the Harness come from this repository or from first-party resource plugin packages in this repository;
- product tests exercise this in-repository Harness;
- documentation points to this repository;
- no build, runtime, test, or release path depends on `matthis-k/phenix-harness`;
- deleting `matthis-k/phenix-harness` loses no unique behavior, configuration, skills, or attribution.

The old repository may remain briefly as a redirect or archive notice while consumers move. It is not part of the final dependency graph.

## Repository rename

Keep the GitHub repository named `phenix-conductor` while #399 and this migration PR are active. Stacked branches and worker instructions currently depend on that identity.

After this migration is merged and validated, rename:

```text
matthis-k/phenix-conductor -> matthis-k/phenix-ai
```

New documentation and Nix examples should use the final input name when they describe the post-migration state:

```nix
inputs.phenix-ai.url = "github:matthis-k/phenix-ai";
```

The repository rename is the final naming step, not a substitute for the package migration.

## Dependency rules

Required dependency direction:

```text
phenix-core
    ^
    |
phenix-api <---- protocol/client adapters
    ^
    |
phenix-conductor
    ^
    |
configured plugin packages through declared contracts
    ^
    |
phenix-harness wrapper/configuration
```

Concrete plugins may depend on `phenix-core` plugin contracts and the API contracts they implement. `phenix-core` and `phenix-api` must not depend on concrete first-party plugins.

Harness configuration may select plugin packages. It must not duplicate their registries, durable state, or execution semantics.

## Worker tasks

- [ ] Rebase this PR onto the completed #399 semantic head before implementation work continues.
- [ ] Reassign current `phenix-core`, `phenix-kernel`, and `phenix-protocol` code to the final ownership model.
- [ ] Remove `phenix-kernel` as a duplicate public package after its mechanisms become `phenix-core`.
- [ ] Add `phenix-api` and move canonical client/server contracts into it.
- [ ] Keep ACP and other protocol adapters separate from `phenix-api`.
- [ ] Split `phenix-plugin-suite` into independent plugin packages.
- [ ] Expose those packages through `phenixPlugins.${system}`.
- [ ] Make `wrappers.phenix.wrap` consume independent plugin derivations.
- [ ] Make `phenix-conductor` generic and free of hidden first-party defaults.
- [ ] Make `phenix-harness` a wrapped configuration of `phenix-conductor`.
- [ ] Move the external Harness runtime configuration into this repository.
- [ ] Move the external Harness skills/resources and required licenses into this repository or first-party resource plugins here.
- [ ] Route Product, ACP, integration, and system product tests through the in-repository Harness.
- [ ] Remove old package aliases, duplicate registries/state, and stale external-Harness dependency paths.
- [ ] Update architecture and packaging documentation to the final model.
- [ ] Review crate/module boundaries after the split and keep files focused.
- [ ] Leave repository-rename-only changes recorded for the final GitHub rename.

## Required regressions

- `phenix-core` boots without first-party agent services.
- `phenix-conductor` boots with zero configured first-party plugins and exposes no product fallback.
- every default first-party plugin has an independently buildable package.
- a test composes the conductor with exactly one plugin from `phenixPlugins.${system}`.
- omitting that plugin removes its service.
- replacing it with an alternate provider uses the same conductor/plugin contracts.
- building one plugin does not require unrelated first-party implementation packages except shared dependencies.
- `phenix-harness` is produced through the public wrapper path.
- the Harness contains the migrated default runtime configuration and skill resources.
- Product smoke uses the in-repository Harness and does not fetch `matthis-k/phenix-harness`.
- ACP and canonical integration/system product journeys use the same Harness composition.
- repository checks find no build/runtime/test dependency on the external Harness repository.
- Source, Rust, Product, Nix composition, Maintenance, and Maintenance-autofix pass on the final exact head.

## Completion criteria

This PR is complete when:

- `phenix-core`, `phenix-api`, `phenix-conductor`, Phenix Plugins, and `phenix-harness` have distinct ownership;
- `phenix-kernel` no longer competes with `phenix-core`;
- `phenix-protocol` no longer competes with `phenix-api`;
- `phenix-plugin-suite` no longer owns first-party plugin implementations;
- every first-party plugin selected by the supported Harness is independently packageable and selectable;
- the supported Harness is a configured conductor assembled through the public wrapper path;
- this repository contains every unique Harness configuration/resource required by the product;
- deleting the separate `phenix-harness` repository is safe;
- the repository is ready to rename from `phenix-conductor` to `phenix-ai`;
- full exact-head validation passes.
