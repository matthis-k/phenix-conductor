---
temporary: true
---

# Canonical architecture language

## Goal

Make normative architectural wording precise, consistent, and owner-specific without changing behavior.

## Required changes

- Use exact cross-cutting terms consistently: Plugin, Component, Interface, Import, Export, Provider, Provider Binding, Language Binding, Layer, Event, Listener, Hook, Runtime Provider, Plugin Runtime, PluginHost, Host Capability, Graph Generation, Artifact Revision, Plugin Resource, Application, Adapter, Client SDK, and Transport.
- Resolve overloaded bare terms such as `binding`, `runtime`, `client`, `artifact`, `resource`, `execution`, `generation`, `host`, `capability`, and `state` by qualifying them when context is not singular.
- Reserve normative `must`, `must not`, `may`, and `should` for requirements. Remove fuzzy requirement wording when an exact rule exists.
- Name the responsible owner in requirements: kernel, resolver, plugin, adapter, application, runtime provider, persistence provider, or product composition.
- Keep each semantic definition in one owning specification. Add references instead of duplicate definitions.
- Remove duplicate status prose such as `Status: implementation contract` when lifecycle metadata already carries that meaning.
- Record any sentence whose rewrite exposes a real semantic ambiguity as an architecture finding for the following implementation PRs rather than choosing behavior silently.

## Boundary

No semantic behavior change belongs in this PR.

## Completion

- normative architecture text has one precise vocabulary;
- ambiguous overloaded terms are qualified where needed;
- architecture findings are assigned to a later PR in this stack;
- Source, Docs, and Maintenance pass on the exact head;
- delete this temporary implementation slice before merge.
