# Structural plugin contracts

status: implemented

Plugins may use native language types internally. Public plugin boundaries lower those types into a small structural representation owned by `phenix-core`.

The ABI value is `PhenixValue`. Native Rust types are local views over that value; they are not part of the cross-plugin ABI.

```text
provider-native type
        |
        | PhenixValue
        v
      Value
        |
        | kernel / IPC
        v
      Value
        |
        | consumer-owned TryFrom
        v
consumer-native type
```

Provider and consumer types may differ. The provider can still use a strongly typed local representation for implementation safety and documentation, while the consumer independently defines the shape it needs.

## Core vocabulary

`PhenixSchema` contains only portable runtime shapes:

- any structural value, unit, boolean, signed and unsigned 64-bit integers, 64-bit float, string, bytes
- option, fixed-length array, homogeneous dynamic list, and homogeneous string-keyed map
- exact keyed table
- tagged variant
- callable reference with input/output types
- object reference

Architecture-sized integers are not ABI types.

`PhenixValue` is the corresponding runtime representation. Every value converts to its concrete `PhenixSchema` (`value.schema()` or `PhenixSchema::from(&value)`), so structural satisfaction is the reverse schema comparison `expected.accepts(&value.schema())`. `Never` is the uninhabited schema used only where a concrete value provides no inner example, such as `None` or an empty collection. Fixed arrays and dynamic lists share the sequence value representation; a concrete sequence schema retains its observed length so fixed-array checks remain possible. Maps have arbitrary string keys and a homogeneous value type. `Any` is a schema-level wildcard, not an opaque value variant: deliberately dynamic data such as JSON still lowers recursively into ordinary structural values. Tables use parsed non-empty structural keys. Callable and object values are opaque references that bind a contract, provider plugin, graph generation, and reference identity. They never expose Rust function pointers, trait objects, or shared mutable implementation state.

## Schema compatibility

Every component import and export carries request and response `PhenixSchema` metadata. The resolver compares them before activation.

Compatibility is directional. A provider request schema must accept every request the consumer may send. A consumer response schema must accept every response the provider may return. Extra provider table fields are compatible with a smaller consumer view. Extra provider variants are incompatible with a consumer that does not know those variants. Callable inputs are contravariant and callable outputs are covariant.

The resolver classifies a pair as exact, compatible, or incompatible. It skips incompatible providers and may select a lower-priority compatible provider. A required import with only incompatible providers fails graph construction with the structural mismatch path. Interface identity remains nominal: matching schemas do not make different interface IDs interchangeable.

This check proves structural compatibility only. Local semantic refinements may still reject structurally valid values at runtime, so conversions remain fallible.

## Consumer-owned decoding

A consumer may define a native view independently of the provider and choose its shape policy at the call site:

```rust
#[derive(PhenixValue)]
struct CoverageSummary {
    covered: u64,
    label: String,
}

let exact = CoverageSummary::try_from(Exact(&value))?;
let projected = CoverageSummary::try_from(Project(&value))?;
```

`Exact` requires the complete derived shape and rejects unexpected fields. `Project` requires every declared field and its declared type but ignores unused table fields. Projection applies recursively through derived structs, enum payloads, options, lists, and boxes.

The same derived type supports both conversions. No derive attribute changes its decoding policy. Missing fields, wrong types, and unknown variants are ordinary errors; structural mismatches must never panic.

A caller that wants the match classification and decoded value can use normal conversion syntax:

```rust
let matched: ValueMatch<CoverageSummary> = value.into();
match matched {
    ValueMatch::Exact(summary) => use_exact(summary),
    ValueMatch::Compatible(summary) => use_projected(summary),
    ValueMatch::Incompatible(error) => return Err(error.into()),
}
```

`Exact` is determined by the derived schema. Conversion errors do not get reclassified as relaxed compatibility.

Kernel-mediated SDK clients expose the same policy directly:

```rust
let summary: CoverageSummary = client.invoke_projected(&request)?;
```

The request is lowered to `PhenixValue`, the provider returns `PhenixValue`, and the response is projected into the consumer's local type. `invoke_exact` is available when the consumer intentionally requires exact structural equality. `invoke_value` exposes the raw ABI for dynamic consumers.

A failed provider request decode or consumer response decode returns the conversion error and also emits `kernel.structural_value_mismatch` as a diagnostic event. Event reporting is secondary: failure to deliver the diagnostic must not replace, panic on, or otherwise hide the original structural error.

## Optional contracts

`Contract` and `ContractValue` describe an exact named structural contract when a plugin wants one. They are stronger metadata and invariant-bearing values, not a requirement for ordinary plugin calls.

A `ContractValue` is created by parsing a `PhenixValue` through a `Contract`:

```rust
let value = contract.parse(raw_value)?;
```

After that operation the value is known to have the contract's exact shape. Runtime code must not carry a separate validity flag or repeatedly validate it.

Tables reject missing and unexpected fields. Variants reject unknown tags. Object and callable references reject mismatched contracts. Contract identifiers use the same explicit positive `@version` form as runtime interfaces.

`Contract::parse` remains exact regardless of native conversion mode. A `ContractValue` therefore proves the full named contract shape without making named contracts the general ABI.

## Dynamic access

Dynamic consumers can stay dependent only on `phenix-core`:

```rust
let percent: f64 = coverage
    .get("percent")?
    .value()?;
```

Both key lookup and value conversion are fallible. Wrong shapes do not panic.

## Native derives

Rust plugins may derive the structural adapter:

```rust
#[derive(PhenixValue)]
struct Coverage {
    covered: u64,
    total: u64,
    percent: f64,
}
```

A named contract can additionally derive `PhenixContract` with `#[phenix(id = "testing.coverage@1")]`.

The generated implementation targets `phenix-core` types. `phenix-sdk` re-exports the derive macros, conversion wrappers, and `PhenixSchema`. Plugin code uses `From`/`TryFrom` for values; the codec trait remains internal derive/runtime glue.

Named structs become tables. Rust enums become tagged variants. Newtypes delegate to their inner structural type. Unit types remain unit. Multi-field tuple structs and tuple variants are rejected because their positional ABI would be ambiguous; authors should use a named payload type instead.

## Ownership and authority

Structural values do not grant authority.

A callable or object reference records where the capability came from, but every use must still be mediated by the kernel under the current caller's effective authority and current runtime generation. A stale or unauthorized reference therefore cannot become ambient cross-plugin access.

Plugin-owned state and implementation objects remain private. Cross-plugin sharing occurs only through declared interfaces and structural values.

## Kernel boundary

The kernel knows structural values, types, references, and interface identities, but no userspace concepts such as session, model, agent, skill, test run, or provider semantics. Those remain plugin-defined.

This representation is also the intended basis for injected Bevy-style plugin parameters. Parameter types can eventually derive interface requirements and authority without introducing source dependencies between provider and consumer plugins.
