# Plugin SDK contributions

Status: implementation contract.

## Purpose

Let a selected plugin extend an SDK without adding plugin-specific methods to core.

Core only resolves SDK contribution metadata. SDK behavior stays in plugins and uses the same typed component interfaces as plugin-to-plugin calls.

## Contribution

An SDK contribution declares:

- the selected plugin that provides it;
- one stable SDK namespace;
- typed component interfaces exposed through that namespace;
- optional opaque SDK resource identifiers for client helper code or generated bindings.

`SdkNamespace` and `SdkResourceId` are parsed identifiers. Invalid identifiers do not enter resolved state.

Resources are opaque to core. Packaging and SDK consumers decide how to materialize them.

## Resolution

SDK contributions resolve against an already resolved Harness composition.

Resolution requires:

1. the provider plugin is selected;
2. every referenced interface is exported by a selected component;
3. one provider owns each SDK namespace.

Source order does not affect the result. SDK metadata grants no authority and does not alter component or service provider selection.

The resolved SDK set is derived from the selected plugins, components, and contribution metadata. It is not stored as a second runtime registry.

## Default Phenix SDK

`phenix-plugin-sdk` is an ordinary userspace plugin that contributes the `phenix` namespace. It provides typed modules for:

```text
phenix.sessions
phenix.models
phenix.tools
phenix.skills
phenix.context
phenix.options
```

Models, context, and options expose their ordinary typed plugin interfaces directly. Tools wrap execution callables. Skills wrap context resources whose kind is `skill`. These wrappers add SDK ergonomics without creating parallel durable state or provider registries.

Sessions add the typed helper `phenix.sdk.sessions@1`. `Open` looks up the requested session and resolves scoped options before calling the ordinary session interface:

```text
session.reuse_existing
session.auto_create
```

The options context includes the requested session and optional agent. Normal option precedence therefore changes SDK behavior without adding session policy to the sessions plugin.

`phenix.sdk.tools@1` maps tool registration and invocation to the execution plugin's callable contract. `phenix.sdk.skills@1` maps skill registration, lookup, and listing to the context plugin's resource contract.

The SDK contribution also names opaque resources for each default module so language-specific bindings or helper packages can be attached without changing core.

A testing plugin can independently contribute a `testing` namespace with test interfaces and helper resources.

```text
phenix SDK plugin
  -> namespace phenix
  -> sessions -> options + sessions
  -> tools -> execution callables
  -> skills -> context resources
  -> models/context/options -> existing interfaces

testing plugin
  -> namespace testing
  -> testing interfaces/resources
```

Removing either plugin removes its SDK contribution. A compatible replacement can provide the same namespace when the previous provider is not selected.

## Invariants

- SDK helpers use ordinary typed plugin interfaces.
- SDK contribution metadata carries no authority.
- First-party and third-party plugins use the same contract.
- Duplicate namespace ownership fails resolution.
- Unknown providers fail resolution.
- References to unavailable interfaces fail resolution.
- Resource-only plugins may provide client-only SDK helpers.
- Zero-plugin composition has no SDK namespaces.
- The Phenix SDK can be replaced or omitted.
- SDK convenience behavior reads userspace options rather than adding hidden core policy.
- SDK wrappers reuse product-owned state instead of storing duplicate tool or skill state.

## Required regressions

- two selected plugins contribute different namespaces;
- a resource-only plugin contributes client helper resources;
- a contribution cannot reference an unavailable interface;
- duplicate namespace ownership fails;
- an unselected provider fails;
- empty composition resolves to no SDK namespaces;
- the default Phenix SDK advertises sessions, models, tools, skills, context, and options;
- the default SDK session helper honors scoped option resolution;
- SDK tools register through execution callables;
- SDK skills register and list through context resources.
