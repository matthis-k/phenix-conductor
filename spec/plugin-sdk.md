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

## Phenix SDK

The normal Phenix SDK can be an ordinary plugin that contributes the `phenix` namespace and wraps the standard Phenix plugin interfaces.

Core does not need session, agent, model, tool, or other Phenix SDK methods.

A testing plugin can independently contribute a `testing` namespace. It may expose typed testing interfaces, client helper resources, or both.

```text
phenix-sdk plugin
  -> namespace phenix
  -> standard Phenix interfaces/resources

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

## Required regressions

- two selected plugins contribute different namespaces;
- a resource-only plugin contributes client helper resources;
- a contribution cannot reference an unavailable interface;
- duplicate namespace ownership fails;
- an unselected provider fails;
- empty composition resolves to no SDK namespaces.
