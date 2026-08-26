# Phenix AI package layout and repository consolidation

Status: migration contract.

## Purpose

Make the current `phenix-conductor` repository the complete Phenix AI source repository. After this migration it can be renamed to `phenix-ai`, and the separate `phenix-harness` repository contains no unique product configuration or resources and can be deleted.

The repository must keep four roles distinct:

```text
phenix-core
  generic runtime mechanisms and plugin hosting

phenix-api
  shared client/server contracts

phenix-conductor
  generic server/runtime built on phenix-core + phenix-api

phenix-harness
  wrapped configuration of phenix-conductor + selected plugins + product policy
```

First-party agent behavior belongs to individually packaged plugins collected by the Phenix Plugins package set.

## Package ownership

### `phenix-core`

`phenix-core` owns generic mechanisms only:

- plugin identity, manifests, lifecycle, activation, and dependency handling;
- authority enforcement and isolation;
- generic service registration and provider resolution;
- event delivery and task/cancellation mechanisms;
- persistence backend contracts, namespaces, schemas, migrations, and transactions;
- embedded, external, and resource-only plugin hosting.

The current `phenix-kernel` implementation migrates into this role. `phenix-kernel` must not remain as a second public package with overlapping ownership.

The current `phenix-core` crate contains agent-domain types. Move each type according to ownership before `phenix-core` takes the kernel role:

- client-visible request, response, event, and identifier contracts move to `phenix-api`;
- plugin-owned domain types move to the plugin that owns that domain;
- generic runtime types required by the plugin host stay in `phenix-core`.

`phenix-core` must boot without first-party agent plugins and must not provide session, artifact, context, routing, tool, worker, planning, or model fallbacks.

### `phenix-api`

`phenix-api` defines the contracts between Phenix clients and the server side. It does not implement a client.

It owns:

- client/server request and response types;
- server event and notification types;
- capability and service descriptors required by clients;
- shared identifiers that are part of the client contract;
- serialization rules and protocol versioning for the canonical Phenix API.

The current `phenix-protocol` crate should migrate into `phenix-api`. Types currently in `phenix-core` move here only when they are part of the client/server contract.

`phenix-api` must not depend on `phenix-harness` or concrete first-party plugins. External adapters such as ACP may translate to and from `phenix-api`, but ACP is not the definition of the canonical Phenix API.

### `phenix-conductor`

`phenix-conductor` is the generic server/runtime. It depends on `phenix-core` and `phenix-api`, hosts configured plugins, and exposes the configured API to clients.

It must not encode the Phenix product's default plugin selection. Starting the conductor without product configuration must not silently load first-party agent services.

The conductor may support embedded plugin factories, external plugins, and resource-only plugins. Those hosting forms do not change plugin ownership.

### Phenix Plugins package set

Every first-party plugin is an independently buildable and selectable package. The package set groups those packages; it is not one implementation monolith.

The Nix interface should expose a package set such as:

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

The exact plugin split may change when two domains have one real ownership boundary. The required property is independent packaging and selection. Choosing one plugin must not require a monolithic first-party suite package as the implementation owner.

Cargo package names may use names such as `phenix-plugin-session`. The primary Nix interface is the `phenixPlugins.${system}` package set, so users select `phenixPlugins.${system}.session` rather than relying on flat package names.

The current `phenix-plugin-suite` crate is transitional. Split its domain modules into independently owned plugin packages. A thin internal catalog may collect factories when needed for an embedded build, but it must not own plugin semantics or durable state.

Each plugin:

- declares its own manifest and service contributions;
- owns its durable namespace and migrations;
- can be omitted without exposing a conductor fallback;
- can be replaced through the same service contract;
- can be tested without loading unrelated first-party plugins.

### `phenix-harness`

`phenix-harness` is a wrapped configuration of `phenix-conductor`. It is not a second conductor implementation.

Conceptually:

```nix
phenix-harness = wrappers.phenix.wrap {
  conductor = packages.${system}.phenix-conductor;
  plugins = [
    phenixPlugins.${system}.session
    phenixPlugins.${system}.artifact
    # default first-party selection
  ];
  # default policy and resources
};
```

`phenix-harness` owns product defaults:

- the default first-party plugin selection;
- grants, priorities, bindings, and product policy;
- the default runtime configuration;
- first-party skills and other product resources that are part of the supported Phenix configuration.

Changing Harness policy must not require copying conductor semantics into a Harness runtime crate. If embedded plugins require a configured build, `wrappers.phenix.wrap` may build that configured conductor derivation from the selected plugin packages. The Harness role remains configuration and composition.

The normal user-facing `phenix` package may alias the Harness result.

## Nix outputs

The future `phenix-ai` repository should expose at least:

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

`phenixPlugins.<system>.<plugin>` values are independently selectable plugin derivations.

`packages.<system>.phenix-harness` must be produced through the same wrapper/configuration path available to users. It must not use a private product-only composition path.

Kernel-only testing becomes a `phenix-core` or explicit conductor-with-zero-plugins configuration. Do not retain `phenix-kernel` as a competing public name.

## Consolidate the current Harness repository

The current `matthis-k/phenix-harness` repository is already a wrapper around `phenix-conductor`. Its unique product data must move into this repository before the old repository is removed.

Migrate at least:

```text
config/phenix/runtime.nix
config/phenix/skills/**
```

Preserve licenses and attribution files associated with imported skills and resources.

The in-repository Harness wrapper must reproduce the current supported configuration without a Git or flake dependency on `matthis-k/phenix-harness`.

After migration:

- the supported Harness package comes from this repository;
- tests use the in-repository Harness configuration;
- documentation points to the in-repository configuration;
- no build, runtime, test, or release path needs the external Harness repository;
- deleting `matthis-k/phenix-harness` does not remove unique configuration, skills, licenses, or product behavior.

The old repository may remain temporarily as a redirect or archival notice while consumers move. It is not part of the final dependency graph.

## Repository rename

Keep the GitHub repository named `phenix-conductor` while the migration PR stack is active. Workers and stacked branches depend on the current repository identity.

After the package split, Harness consolidation, old-path cleanup, and final validation are merged, rename the repository to `phenix-ai`.

New documentation and Nix examples should use the final input name:

```nix
inputs.phenix-ai.url = "github:matthis-k/phenix-ai";
```

The package remains `phenix-conductor`; only the repository becomes `phenix-ai`.

## Dependency direction

The intended dependency direction is:

```text
phenix-core
    ^
    |
phenix-api  <---- client/protocol adapters
    ^
    |
phenix-conductor
    ^
    |
selected phenixPlugins.* through declared plugin contracts
    ^
    |
phenix-harness configuration
```

Concrete plugin packages may depend on `phenix-core` plugin contracts and on the API contracts they implement. `phenix-core` and `phenix-api` must not depend on concrete first-party plugins.

## Migration requirements

1. Reassign the current `phenix-core`, `phenix-kernel`, and `phenix-protocol` contents to the ownership model above.
2. Remove `phenix-kernel` as a duplicate public package after its mechanisms become `phenix-core`.
3. Replace the monolithic `phenix-plugin-suite` implementation with independently packaged first-party plugins.
4. Expose those plugin packages through `phenixPlugins.${system}` and make the wrapper accept those derivations directly.
5. Make `phenix-conductor` the generic configured server/runtime, with no hidden first-party defaults.
6. Make `phenix-harness` a wrapper/configuration of `phenix-conductor` using the default plugin set and product policy.
7. Move the current external Harness configuration, skills, and required attribution files into this repository.
8. Route Product, ACP, integration, and system tests through the in-repository Harness where they test the supported product.
9. Remove obsolete package aliases, duplicate registries, duplicate durable state, and old Harness dependency paths.
10. Leave the merged repository self-contained and ready for the GitHub rename to `phenix-ai`.

## Required regressions

- `phenix-core` boots without first-party agent services.
- `phenix-conductor` boots with zero configured first-party plugins and exposes no product fallback.
- every first-party plugin in the default Harness has an independently buildable package.
- a test can compose the conductor with exactly one first-party plugin from `phenixPlugins.${system}`.
- omitting that plugin removes its service.
- replacing it with an alternate provider uses the same conductor/plugin contracts.
- building one plugin does not require building the implementation code for unrelated first-party plugins, except shared dependencies.
- `phenix-harness` is produced from the public wrapper/configuration path.
- the Harness package contains the migrated default runtime configuration and skill resources.
- Product smoke uses the in-repository Harness and does not fetch `matthis-k/phenix-harness`.
- ACP and remaining integration/system product journeys execute through the same Harness composition.
- repository checks find no runtime/build/test dependency on the external Harness repository.
- full Source, Rust, Product, Nix composition, Maintenance, and Maintenance-autofix validation passes on the final exact head.

## Completion gate

The migration is complete only when all of these statements are true:

- `phenix-core`, `phenix-api`, `phenix-conductor`, the Phenix Plugins package set, and `phenix-harness` have non-overlapping ownership;
- `phenix-plugin-suite`, `phenix-kernel`, and the old domain-heavy meaning of `phenix-core` no longer exist as competing implementation owners;
- the supported Harness is a configured conductor assembled from independently selectable plugin packages;
- this repository contains all product configuration and resources needed by the supported Harness;
- the separate `phenix-harness` repository can be deleted without losing unique product behavior or data;
- the repository is ready to be renamed from `phenix-conductor` to `phenix-ai`.
