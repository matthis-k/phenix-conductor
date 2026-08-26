# Nix-native kernel, Harness, and plugin packaging

Status: implementation and packaging contract.

## Purpose

Make `wrappers.phenix` the primary Nix configuration interface for composing the Phenix Harness while keeping the kernel independently available and every userspace plugin replaceable.

Kernel, Harness, and most first-party Phenix Plugin Suite sources may live together in `phenix-ai` without weakening their architectural boundary.

## Primary Nix interface

Normal use builds the Harness:

```nix
inputs.phenix-ai.wrappers.phenix.wrap {
  inherit pkgs;
}
```

This composes the Phenix Kernel with the selected Phenix Plugin Suite and product policy.

The wrapper is the single Nix configuration model for suite enablement, alternate providers, external/resource packages, grants, bindings, priorities, settings, generated runtime configuration, and final derivation.

## Kernel-only profile

An explicit kernel-only profile is available for infrastructure testing, debugging, and custom userspaces.

It disables the Phenix Plugin Suite. It does not expose intrinsic session/artifact/context/tool fallbacks because those are not kernel features.

Conceptually:

```nix
inputs.phenix-ai.wrappers.phenix.wrap {
  inherit pkgs;
  kernelOnly = true;
}
```

The exact option name may be refined during implementation.

## Distribution forms

Plugin semantics do not imply one package form.

```text
embedded executable
  Rust crate linked into product
  manifest embedded in product
  activated by PluginId

external executable
  independent immutable package
  manifest + executable
  activated from exact store path

resource-only
  independent immutable package
  manifest + static resources
  activated from exact store path
```

First-party status is independent of hosting form.

## Embedded suite plugins

Most trusted first-party executable suite plugins may be embedded in the normal `phenix` product for efficiency.

Harness/wrapper policy selects which linked factories are enabled and supplies settings, grants, priority, and bindings. Changing only policy does not require relinking the underlying executable.

Embedding does not make a plugin part of the kernel. The kernel crate never depends on concrete suite crates.

## Alternate and external plugins

Users may replace suite services or add new services through the same wrapper.

```nix
inputs.phenix-ai.wrappers.phenix.wrap {
  inherit pkgs;
  plugins = [ inputs.some-plugin.packages.${pkgs.system}.default ];
}
```

Structured entries may add product policy such as permissions, priority, bindings, and settings.

Packages carry implementation/resources and manifests. They do not grant themselves effective authority or priority.

## Product assembly

Dependency direction:

```text
phenix-kernel
      ^
      |
plugin crates
      ^
      |
phenix-harness / product assembly
```

Product assembly owns the linked factory catalog. Harness owns the supported composition and runtime policy.

A custom distribution may assemble a different set of embedded plugins without modifying the kernel.

## `mkPhenixPlugin`

`phenix-ai.lib.mkPhenixPlugin` packages independently supplied external executable or resource-only plugins and validates their manifests/resources/entrypoints.

It is not required for statically linked Rust crates and is not the normal product configuration interface.

## Flake outputs

`phenix-ai` should expose at least:

```text
wrappers.phenix
packages.<system>.phenix-kernel
packages.<system>.phenix-harness
packages.<system>.phenix
lib.mkPhenixPlugin
lib.mkPhenix
```

`phenix-kernel` is infrastructure only.

`phenix-harness` is the supported kernel + Phenix Plugin Suite composition.

`phenix` may be the normal user-facing package/alias for the Harness composition.

Optional `phenix-plugin-*` package outputs are build/test conveniences for independently packaged external/resource plugins. Embedded first-party Rust crates do not need fake runtime-loadable package outputs.

## Wrapper-module reuse

NixOS, Home Manager, nix-darwin, devenv, and flake-parts integrations adapt the same `wrappers.phenix` configuration rather than defining parallel Phenix option semantics.

No overlay is required.

## Reproducibility

A wrapped Harness derivation pins:

- the exact kernel/product build;
- enabled embedded plugin IDs/manifests;
- exact external/resource plugin store paths/manifests;
- settings, grants, priorities, and bindings;
- the selected userspace composition.

A Nix-built product performs no runtime plugin download or Rust dynamic-library loading.

## Invariants

- `.wrap { inherit pkgs; }` produces the normal Phenix Harness.
- Kernel-only mode is explicit and contains no miniature agent-harness fallbacks.
- Phenix Plugin Suite components can be disabled or replaced through ordinary composition.
- Embedded plugins remain userspace architecturally.
- Kernel crate does not depend on concrete suite crates.
- External/resource plugins are pinned immutable packages.
- Package/manifest contents cannot self-grant authority or effective priority.
- Platform modules adapt the wrapper instead of creating a second configuration model.
- Consuming Phenix does not require an overlay.

## Required regressions

- zero-config wrapper builds the normal Harness composition;
- explicit kernel-only wrapper builds infrastructure without suite services;
- wrapper can enable/disable a linked first-party service by `PluginId`;
- wrapper can replace a first-party provider with an alternate provider;
- enabling an unavailable embedded `PluginId` fails validation;
- changing only activation policy reuses the same underlying binary package when implementation bytes are unchanged;
- external executable and resource-only packages compose through the same wrapper;
- equivalent wrapper configuration produces deterministic runtime configuration;
- malformed manifests or missing required entrypoints fail at build/check time;
- no runtime plugin download or Rust dylib loading occurs.