# Structural plugin contracts

Status: normative architecture and implementation contract.

Plugins may use native language types internally. Public plugin boundaries lower those types into the structural representation owned by `phenix-core`.

`PhenixValue` is the dynamic ABI value. Native Rust, TypeScript, Python, Lua, or other language types are local views over that value. They are not the cross-plugin ABI.

```text
provider native type
  -> PhenixValue
  -> kernel or runtime bridge
  -> PhenixValue
  -> consumer-owned typed parse
  -> consumer native type
```

Provider and consumer native types may differ. They need compatible structural schemas, not shared implementation types.

## Core vocabulary

`PhenixSchema` contains portable runtime shapes:

- any structural value;
- unit, boolean, signed and unsigned 64-bit integer, 64-bit float, string, bytes;
- option;
- fixed array and homogeneous dynamic list;
- homogeneous string-keyed map;
- keyed table;
- tagged variant;
- callable reference with input and output schemas;
- object reference.

Architecture-sized integers are not ABI types.

`PhenixValue` is the corresponding runtime representation. A concrete value has a concrete schema. Structural satisfaction compares the expected schema with the value schema.

`Any` is a schema wildcard, not an opaque runtime value. Dynamic JSON, when JSON is the external format, lowers recursively into ordinary `PhenixValue` data.

Callable and object references are opaque kernel-mediated references. They bind semantic contract identity, provider identity, graph generation, and reference identity. They never expose Rust function pointers, trait objects, or shared mutable implementation state.

## Interface compatibility

Every import and export carries request and response `PhenixSchema` metadata. The resolver checks compatibility before activation.

Compatibility is directional:

- a provider request schema must accept every request the consumer may send;
- a consumer response schema must accept every response the provider may return;
- callable inputs are contravariant;
- callable outputs are covariant.

Extra provider table fields are compatible with a consumer that needs only a smaller projected view. Extra provider variants are incompatible when the consumer cannot represent them.

The resolver classifies a provider and consumer pair as exact, compatible, or incompatible. Incompatible providers are excluded before activation. A required import with no compatible provider fails graph construction.

Interface identity remains nominal. Matching schemas do not make different interface IDs interchangeable.

Structural compatibility does not prove local semantic refinements. Typed conversion therefore remains fallible.

## Canonical matching wrappers

The authoring model uses one matching vocabulary across all typed boundaries:

```text
T           projected structural matching
Project<T>  explicit projected structural matching
Exact<T>    exact structural matching
```

`T` is the common case. It requires all fields and variants needed by `T` and permits producer fields that the local view does not use.

`Project<T>` states the same projected policy explicitly. Use it where the distinction itself is useful to the reader or type system.

`Exact<T>` requires the complete exact structural shape and rejects unexpected fields or variants.

These wrappers apply consistently to:

- call requests and responses;
- export inputs and outputs;
- listener payloads;
- layer inputs and outputs;
- hook inputs and outputs;
- public callable boundaries;
- runtime-bridge typed boundaries.

Do not add separate method families such as `invoke_projected` or `invoke_exact` when the type already expresses the matching policy.

A deliberately dynamic consumer may request raw `PhenixValue`.

## Consumer-owned decoding

A consumer defines the native shape it needs independently of the provider:

```rust
#[derive(PhenixValue)]
struct CoverageSummary {
    covered: u64,
    label: String,
}
```

Projected decoding accepts compatible extra producer fields:

```rust
let summary: CoverageSummary = CoverageSummary::try_from(&value)?;
```

The explicit projected form has the same structural policy:

```rust
let summary: Project<CoverageSummary> = Project::try_from(&value)?;
```

Exact decoding rejects producer data outside the complete local shape:

```rust
let summary: Exact<CoverageSummary> = Exact::try_from(&value)?;
```

The exact conversion API may use equivalent constructors or `TryFrom` implementations. The semantic distinction belongs to the wrapper type, not to a parallel SDK method name.

Missing required fields, wrong types, and unknown required variants are ordinary errors. Structural mismatches never panic.

## Typed SDK calls

Kernel-mediated clients use the return type to select matching policy.

Conceptually:

```rust
let summary: CoverageSummary = client.invoke(&request)?;
let summary: Project<CoverageSummary> = client.invoke(&request)?;
let summary: Exact<CoverageSummary> = client.invoke(&request)?;
let raw: PhenixValue = client.invoke_value(&request)?;
```

`invoke_value` is reserved for genuinely dynamic consumers. There is no separate `invoke_projected` or `invoke_exact` API.

The request lowers to `PhenixValue`, the kernel dispatches through the resolved graph, and the response is parsed into the consumer-owned local type.

A provider request decode failure or consumer response decode failure returns the conversion error and emits the canonical structural-mismatch diagnostic. Diagnostic delivery failure cannot replace or hide the original error.

## Named contracts

`Contract` and `ContractValue` describe an exact named structural contract when a shared semantic contract needs one.

A `ContractValue` is produced by parsing raw structural data through the named contract:

```rust
let value = contract.parse(raw_value)?;
```

After parsing, the value carries the contract invariant. Runtime code should not keep a second validity flag or repeatedly validate the same value.

Named contracts are stronger metadata, not a requirement for every plugin call.

Shared named contract identity belongs to a neutral passive owner when providers and consumers need to name it independently. It does not belong to a default provider implementation crate.

## Dynamic access

Dynamic consumers can depend only on structural vocabulary and inspect values through fallible access:

```rust
let percent: f64 = coverage
    .get("percent")?
    .value()?;
```

Wrong shapes return errors.

## Native derives

Rust plugins may derive structural adapters:

```rust
#[derive(PhenixValue)]
struct Coverage {
    covered: u64,
    total: u64,
    percent: f64,
}
```

A named shared contract may additionally derive or declare a stable interface/contract identity through the neutral SDK authoring API.

The generated implementation targets Core structural types. `phenix-sdk` re-exports authoring macros, wrappers, and schemas so plugin authors do not need provider implementation crates for contract vocabulary.

Named structs become tables. Rust enums become tagged variants. Newtypes delegate to their inner structural type. Unit remains unit.

Ambiguous positional public ABI shapes should be rejected in favor of named payload types.

## Runtime bridges

Foreign runtimes convert native values to `PhenixValue` immediately at the Phenix boundary.

```text
foreign value
  -> PhenixValue
  -> resolve semantic target
  -> parse local target type
```

A runtime bridge may translate native values and errors. It does not create language-specific matching or contract semantics.

## Ownership and authority

Structural values do not grant authority.

Callable and object references record provenance, but every use remains kernel-mediated under the caller's effective authority and pinned graph generation.

Plugin state and implementation objects remain private. Cross-plugin sharing occurs through declared interfaces, resource contracts, and structural values.

## Kernel boundary

The kernel knows structural values, schemas, references, interface identities, and compatibility rules.

It does not know userspace concepts such as sessions, models, agents, skills, tools, memory entries, test runs, or provider-specific semantics. Shared semantic contracts for those concepts live in neutral passive owners; implementations live in plugins.

## Invariants

- `PhenixValue` is the canonical dynamic Phenix representation.
- Provider and consumer native types may differ structurally.
- Shared implementation Rust types are not required across plugin boundaries.
- `T`, `Project<T>`, and `Exact<T>` are the canonical matching vocabulary.
- Matching policy belongs in types rather than parallel invoke method names.
- Raw `PhenixValue` remains available for genuinely dynamic code.
- Interface compatibility is checked before activation.
- Local typed conversion remains fallible.
- Structural values never grant authority.
- Foreign values normalize to `PhenixValue` immediately at the boundary.
- The kernel remains product-domain neutral.

## Required regressions

- structurally compatible provider and consumer local types interoperate without depending on each other's crates;
- projected `T` accepts producer fields the consumer does not use;
- `Project<T>` has the same projected compatibility semantics as `T`;
- `Exact<T>` rejects unexpected fields and variants;
- the same wrappers work for calls, listeners, layers, and public callables;
- no `invoke_projected` or `invoke_exact` API is needed;
- raw `PhenixValue` remains usable by dynamic consumers;
- incompatible schemas fail graph construction before provider execution;
- conversion errors return normally and emit diagnostics without panicking;
- callable and object references cannot bypass graph-generation or authority checks;
- consumers can use shared contract identities without importing default provider implementation crates.
