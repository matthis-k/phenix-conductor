# Typed structural value boundaries

status: enforced
coverage:
  - scripts/check-structural-boundaries.sh
  - rust/crates/phenix-sdk/tests/incompatible_schema.rs
  - rust/crates/phenix-sdk/tests/plugin_authoring.rs
  - rust/crates/phenix-harness/src/basic_suite.rs
  - rust/crates/phenix-provider-sdk/src/lib.rs

## Status

Current contract for Phenix dynamic values, plugin calls, language bindings, and generic invocation surfaces.

## Goal

Use `PhenixValue` as the canonical dynamic data representation at Phenix boundaries, then convert to an invariant-bearing native type as soon as the receiving side knows the semantic type it expects.

Keep plugins independent. A consumer must not depend on a provider implementation crate merely to decode the provider's value.

## Core rule

```text
foreign or provider-local value
        |
        | adapter or derived conversion
        v
   PhenixValue
        |
        | kernel, IPC, dynamic dispatch
        v
   PhenixValue
        |
        | resolve semantic target
        | consumer-owned parse
        v
consumer-local typed value
```

`PhenixValue` is a boundary representation, not the default representation for typed userspace logic.

`std::any::Any` is not a Phenix interchange format or plugin ABI. Local Rust implementation code may use type erasure when it materially simplifies an internal implementation, but public kernel, SDK, plugin, IPC, persistence, and language contracts must not depend on `Any`.

## Plugin independence

Provider and consumer native Rust types may differ.

A consumer may define its own compatible projected view of a provider response. It must not import the provider implementation crate solely to name request, response, configuration, or state types.

Shared Rust types belong in `phenix-core` only when they are kernel vocabulary, or in a passive neutral contract or SDK crate when their semantics are intentionally shared across plugins.

Implementation crates own implementation types and behavior.

Structural compatibility remains directional and is checked through `PhenixSchema`. Local conversion remains fallible because structural compatibility does not prove semantic refinements.

## Boundary conversion

Foreign representations terminate at their adapter boundary.

Examples:

```text
LuaValue -> PhenixValue
JSON protocol value -> PhenixValue
Python object -> PhenixValue
ACP value -> PhenixValue
```

The kernel and unrelated plugins must not need to know the foreign representation exists.

Outbound values follow the inverse path only at the corresponding adapter.

## Invocation

A generic invocation carries Phenix-native structural data and provenance.

Conceptually:

```text
Invocation
  action
  input: PhenixValue
  context
```

The kernel resolves the target before the payload can be interpreted as a concrete userspace type.

After target resolution, the receiving handler parses the `PhenixValue` into its local request type before business logic runs.

```text
Invocation<PhenixValue>
  -> resolve target
  -> parse local Request
  -> authorize
  -> execute typed handler
  -> local Response
  -> PhenixValue at boundary
```

Parsing is side-effect free. Authorization may depend on the parsed request. Invalid typed state does not survive as a separate validity flag.

UI input, language APIs, hooks, listeners, adapters, and CLI surfaces may all produce invocations. They do not bypass kernel dispatch by directly calling implementation code when the operation is kernel-mediated.

A language binding such as `Phenix.bench(...)` converts the language value to `PhenixValue` and emits the same invocation shape used by other producers. Benchmark semantics remain owned by the benchmark capability, not the Lua binding.

## Typed calls

For a statically known interface, normal plugin code uses typed invocation helpers such as projected or exact calls.

Raw `invoke_value` remains available only for genuinely dynamic consumers that intentionally do not know the response type at compile time.

A typed provider decodes the structural request immediately, runs typed logic, then encodes the typed response at the boundary.

```text
bytes -> PhenixValue -> Request -> handler -> Response -> PhenixValue -> bytes
```

## Dynamic data

`serde_json::Value` is not a second Phenix dynamic representation.

JSON adapters and serializers may use `serde_json::Value` while translating JSON. Canonical kernel state, SDK contracts, plugin contracts, invocation payloads, and domain state use native types or `PhenixValue` when the data is intentionally open-ended.

A value whose semantics are explicitly JSON may remain JSON only when JSON itself is part of the external contract. That exception must not be used for ordinary heterogeneous Phenix data.

`PhenixSchema` represents Phenix structural schemas. Tool input and output schemas, callable schemas, and compatible plugin interface schemas must not use `serde_json::Value` merely because JSON Schema was previously convenient.

## Persistence

Persistence format is independent from domain representation.

A plugin may serialize typed state as JSON bytes inside its private durable namespace. After loading, it parses directly into its invariant-bearing native state.

Persisted domain state must not become `serde_json::Value` merely because JSON is the storage codec.

## Source enforcement

Add deterministic source checks for the mechanically enforceable subset of this rule.

The checks must reject:

- `std::any::Any` in public Phenix ABI, SDK, plugin contract, or kernel boundary definitions;
- new `serde_json::Value` fields in neutral Phenix contracts unless the source location is an explicit JSON adapter or serializer exception;
- callable or tool structural schemas represented as `serde_json::Value` when `PhenixSchema` is the canonical shape;
- provider implementation crate imports used only to name shared consumer contracts where a neutral contract owner exists.

Do not add a broad allowlist that makes the check advisory. Keep exceptions narrow and source-local.

Semantic regressions remain covered by Rust or Product tests rather than inferred from grep alone.

## Required regressions

Add coverage proving:

- a language or protocol adapter lowers its foreign value to `PhenixValue` before kernel dispatch;
- a typed provider receives a native request type, not raw `PhenixValue`, in its business handler;
- a typed consumer receives a native projected or exact response;
- provider and consumer can use different compatible native structs without importing each other's implementation crates;
- incompatible schemas fail before execution;
- semantic refinement failure is returned during consumer-owned parsing;
- a genuinely dynamic consumer can still use raw `PhenixValue`;
- plugin replacement requires no consumer source dependency on the provider implementation;
- JSON serialization remains available at explicit JSON output boundaries;
- no public plugin ABI depends on `std::any::Any`.

## Completion

This contract is complete when:

- `PhenixValue` is the only canonical dynamic Phenix data representation;
- foreign values terminate at their adapters;
- known semantic targets parse structural data into local invariant-bearing types immediately;
- normal statically known plugin calls are typed;
- raw structural invocation is limited to genuine dynamic consumers;
- JSON values remain only where JSON itself is the external format or serializer boundary;
- plugin consumers do not depend on provider implementation crates for shared contracts;
- tool and callable structural schemas use canonical Phenix schema vocabulary;
- deterministic source checks and behavioral regressions prevent recurrence;
- exact-head Source, Rust, Product, and Maintenance validation is green.

## Structural domain failures and runtime failures

Implementation is in progress. `InterfaceSchema` now carries a directional error schema. Infallible interfaces use `Never`, and typed `Call` imports may declare a domain-error type. Runtime outcomes, provider lowering, typed consumer errors, and adapter mappings remain pending. The baseline status above does not certify this boundary.

PluginInstance invocation returns Result<Vec<u8>, String>, and generated handlers call error.to_string(). Domain variants and details disappear before SDK consumers or protocol adapters can classify them.

1. Define one canonical invocation outcome with success and declared domain-error payloads represented by PhenixValue. Keep a separate typed runtime failure enum for resolution, authority, conversion, cancellation, host, bridge, and execution failures. Core owns runtime classifications; plugins own domain error contracts.

2. Add error schema metadata to interface descriptors with the same directional compatibility rules used for responses. Infallible operations use an uninhabited error schema. Generated Result<Response, DomainError> handlers encode the error contract rather than Display text.

3. Typed consumers receive a generic CallError<DomainError> preserving the domain value or runtime classification. Dynamic callers receive the same structural outcome. Consumer-owned projected error views remain supported; unmatched variants yield a typed conversion failure rather than guessed classification.

4. Migrate embedded dispatch, layers, runtime bridges, first-party implementations, and internal wire decoding together. Remove string-only invocation errors and to_string at semantic boundaries; render human messages only at diagnostics and external protocol presentation.

5. Map declared application failures explicitly in the application bridge. ACP, generated Rust clients, and later Lua bindings retain machine-readable classification through their supported error data. Retry behavior belongs to caller policy and declared contracts, not message matching.

Acceptance requires:

- Distinct domain errors with identical Display text stay distinguishable and preserve structured fields.
- Domain errors survive a layer, nested import, and runtime bridge without importing provider implementation types.
- Incompatible error schemas reject before execution; infallible and projected consumers behave deterministically.
- Cancellation, denied authority, malformed payload, bridge disconnect, and domain conflict remain distinct through application and ACP mappings.
