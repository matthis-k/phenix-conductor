# Typed structural value boundaries

status: partial
coverage:
  - scripts/check-structural-boundaries.sh
  - rust/crates/phenix-sdk/tests/incompatible_schema.rs
  - rust/crates/phenix-sdk/tests/plugin_authoring.rs
  - rust/crates/phenix-harness/src/basic_suite.rs
  - rust/crates/phenix-provider-sdk/src/lib.rs

## Status

Implementation slice for converging Phenix dynamic values, plugin calls, language bindings, and generic invocation surfaces onto one structural boundary model.

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

## Current migration inventory

The implementation must review and converge at least these current surfaces.

### Core configuration

`phenix-core/src/configuration.rs` currently carries configuration contributions and resolved contributions as `serde_json::Value`.

Foreign configuration frontends should lower values into `PhenixValue`. A configuration consumer parses its known namespace into its local typed configuration as early as possible.

### Frontend services

`phenix-plugin-frontend` currently carries generic method parameters and results as `serde_json::Value`.

Generic frontend invocation data should use `PhenixValue`. Once the target method or action is resolved, the receiver parses its local request type.

### Execution configuration

`phenix-plugin-execution/src/configuration.rs` currently deserializes service bytes directly into Rust commands and exposes callable schemas as `serde_json::Value`.

Move the service onto the normal structural component boundary. Decode to `PhenixValue`, parse the typed command, execute typed logic, and encode the typed response. Callable schemas use `PhenixSchema` or another canonical neutral schema type.

### Model and tool contracts

Model options, provider metadata, and tool schemas currently contain `serde_json::Value` in canonical contracts.

Use typed values where semantics are known. Use `PhenixValue` for intentionally open provider metadata and options. Use `PhenixSchema` for structural tool and callable schemas.

Provider-specific options are parsed into provider-local invariant-bearing types at the provider boundary.

### Language plugin

Language operation and diagnostic payloads currently retain `serde_json::Value` inside domain records.

Prefer operation-specific typed variants. When a payload is intentionally open because an external language protocol owns the shape, lower it to `PhenixValue` before it enters generic Phenix state.

### Hooks

Hook metadata currently uses `serde_json::Value`, and a statically known context call uses raw `invoke_value`.

Use `PhenixValue` for intentionally open metadata. Use typed invocation for statically known plugin contracts.

### Debug surfaces

Debug services and canonical debug bundles currently convert generic results or execution outputs into `serde_json::Value` before serialization.

Keep canonical debug data typed or structural. JSON serializers perform JSON conversion at the output boundary.

Debug probing may remain a genuinely dynamic consumer and may use raw structural invocation when the probed response type is intentionally unknown.

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

This slice is complete when:

- `PhenixValue` is the only canonical dynamic Phenix data representation;
- foreign values terminate at their adapters;
- known semantic targets parse structural data into local invariant-bearing types immediately;
- normal statically known plugin calls are typed;
- raw structural invocation is limited to genuine dynamic consumers;
- JSON values remain only where JSON itself is the external format or serializer boundary;
- plugin consumers do not depend on provider implementation crates for shared contracts;
- tool and callable structural schemas use canonical Phenix schema vocabulary;
- the current migration inventory is converged or intentionally documented as an explicit external-format exception;
- deterministic source checks and behavioral regressions prevent recurrence;
- exact-head Source, Rust, Product, and Maintenance validation is green.
