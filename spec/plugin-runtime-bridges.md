# Plugin runtimes, hot loading, and build requests

## Status

Specification only.

This builds on the plugin ownership rules from #445 and the stable kernel interfaces from #444.

## Goal

Define one canonical Phenix Plugin API while allowing plugin implementations to arrive through different execution runtimes.

Core initially supports only the embedded runtime. Future runtimes such as WASM, TypeScript, Python, or external processes are provided by ordinary plugins that bridge those environments into the same Plugin API.

The kernel owns plugin management. Load, replace, unload, and reconcile requests are sent to the kernel and applied transactionally through graph generations.

## Terminology

**Plugin** is a unit of runtime behavior that participates in the Phenix component graph.

**Plugin API** is the single kernel-defined semantic contract every plugin implements.

**Plugin runtime** is an environment capable of executing a plugin implementation.

**Runtime bridge** is a provider that maps a plugin runtime onto the canonical Plugin API.

**Plugin artifact** is the executable implementation material consumed by a runtime. Examples include an embedded registration, a WASM component, or a JavaScript bundle.

**Plugin package** is the canonical manifest plus artifact inputs and resources needed to construct a plugin candidate.

**Artifact revision** is the immutable identity of the exact executable artifact used by a graph generation.

Execution runtime is packaging and execution metadata. It is not part of plugin capability or component identity.

## Ownership

### Kernel

The kernel owns:

- plugin identity and canonical manifests;
- plugin lifecycle semantics;
- plugin-management requests and authorization;
- runtime-provider registration and resolution;
- the built-in embedded runtime;
- component graph resolution;
- graph generations;
- artifact revision pinning;
- authority and capability attenuation;
- candidate preparation and rollback;
- activation, replacement, draining, and retirement;
- plugin persistence namespace ownership;
- build-request orchestration and provenance.

The kernel does not own TypeScript, WASM, Python, Node, V8, Wasmtime, language-specific values, or language-specific SDK ergonomics.

### Runtime bridges

Runtime bridges own translation between one execution environment and the canonical Plugin API.

A bridge may own:

- artifact validation specific to its runtime;
- runtime or interpreter setup;
- value conversion between `PhenixValue` and native values;
- lifecycle call translation;
- async and error translation;
- guest capability handles;
- runtime-specific artifact options.

A bridge must not define new plugin semantics, component semantics, authority rules, persistence ownership, or lifecycle rules.

### SDKs

Language SDKs provide authoring ergonomics only. They bind to the canonical Plugin API and standard component contracts.

Examples include Rust `phenix-sdk`, a future TypeScript `@phenix/sdk`, or future Python and Lua bindings.

Language SDKs must not define parallel plugin APIs.

## Canonical Plugin API

All plugin implementations normalize to one logical instance interface.

Conceptually:

```rust
trait PluginInstance: Send {
    fn prepare(&mut self, ctx: &PrepareContext<'_>) -> Result<(), PluginError>;
    fn start(&mut self, ctx: &StartContext<'_>) -> Result<(), PluginError>;

    fn invoke(
        &mut self,
        component: &ComponentId,
        service: &ServiceId,
        input: PhenixValue,
        ctx: &InvocationContext<'_>,
    ) -> Result<PhenixValue, PluginError>;

    fn quiesce(&mut self, ctx: &QuiesceContext<'_>) -> Result<(), PluginError>;
    fn stop(&mut self, ctx: &StopContext<'_>) -> Result<(), PluginError>;
}
```

The exact Rust surface may differ. The semantic phases do not.

Consumers cannot observe whether a provider is embedded, WASM, TypeScript, or another future runtime.

## Runtime-provider interface

Core exposes one kernel-domain interface for producing canonical plugin instances.

Conceptually:

```rust
trait PluginRuntimeProvider {
    fn id(&self) -> PluginRuntimeId;

    fn validate_artifact(
        &self,
        manifest: &PluginManifest,
        artifact: &PluginArtifact,
    ) -> Result<ValidatedArtifact, PluginRuntimeError>;

    fn instantiate(
        &self,
        manifest: &PluginManifest,
        artifact: ValidatedArtifact,
        guest: GuestPluginHost,
    ) -> Result<Box<dyn PluginInstance>, PluginRuntimeError>;
}
```

Core initially provides `embedded` directly.

Future runtime bridges are ordinary plugins that export the kernel runtime-provider interface. Example packages may include:

- `phenix-plugin-runtime-wasm`;
- `phenix-plugin-runtime-typescript`.

Do not add a closed Core enum containing every future runtime.

Prefer an open identifier:

```rust
struct PluginRuntimeId(String);
```

A plugin execution declaration references a runtime ID and runtime-specific artifact options.

## Bootstrap

`embedded` is the only runtime guaranteed by Core.

Every other runtime provider must itself be loadable by an already available runtime.

Initially:

```text
embedded
  -> WASM runtime bridge
      -> WASM plugins

embedded
  -> TypeScript runtime bridge
      -> TypeScript plugins
```

Runtime dependencies form an acyclic graph. Every runtime dependency chain must terminate at `embedded`.

A runtime bridge does not gain special plugin privileges merely because it provides another runtime.

## Equal plugin capabilities

Execution runtime must not change the observable Plugin API or grant additional Phenix authority.

Given the same manifest, configuration, bindings, and authority, implementations through different runtimes are semantically interchangeable.

The guest plugin receives its own attenuated host capabilities. It does not inherit the runtime bridge plugin's authority.

Conceptually:

```text
caller authority
  intersect plugin maximum authority
  intersect configured policy
  -> guest effective authority
  -> GuestPluginHost
  -> runtime translation
  -> guest plugin
```

The bridge's own authority is separate.

An in-process embedded implementation can physically call ambient Rust or OS APIs, but that must not become a supported Phenix capability path. Core APIs and architecture tests must treat ambient access as outside the Plugin API.

## Canonical manifest boundary

The kernel must be able to inspect a plugin candidate without executing plugin code.

The canonical manifest contains at least:

- plugin ID and package version;
- runtime ID;
- component imports and exports;
- stable interface IDs and schemas;
- requested authority;
- configuration schema;
- persistence schema metadata;
- dependencies;
- artifact input metadata.

Runtime-specific executable code cannot determine its own authority or component contract after execution begins.

## Plugin management interface

Plugin management is a kernel-domain capability.

Authorized callers may send typed management requests to the kernel through a stable kernel interface such as `phenix.kernel.plugins@1`.

The public operations are desired-state requests, not direct lifecycle hooks.

At minimum support the semantics of:

```rust
enum PluginManagementRequest {
    Load(PluginLoadRequest),
    Unload(PluginUnloadRequest),
    Reconcile(PluginSetRequest),
}
```

`Load` with an already-active `PluginId` is a replacement request. A separate semantic `reload` primitive is unnecessary.

Lifecycle methods such as `start` and `stop` are not public management operations.

### Load request

Conceptually:

```rust
struct PluginLoadRequest {
    manifest: PluginManifest,
    artifact: PluginArtifactInput,
    expected_active_revision: Option<ArtifactRevision>,
}
```

The optional expected revision provides compare-and-swap semantics for development tools and agents. A stale request must fail rather than replacing an unexpected newer artifact.

### Unload request

Conceptually:

```rust
struct PluginUnloadRequest {
    plugin: PluginId,
    expected_active_revision: Option<ArtifactRevision>,
}
```

Unload removes the plugin from the desired graph. It does not immediately destroy an instance that still belongs to a pinned old generation.

### Reconcile request

A reconcile request supplies the desired plugin set and lets the kernel compute additions, replacements, removals, and affected runtime dependents in one transaction.

This is the canonical internal model. `load` and `unload` are convenience mutations of desired state.

## Artifact inputs

A load request may reference either a ready artifact or a build plan that produces one.

Conceptually:

```rust
enum PluginArtifactInput {
    Ready(ArtifactRef),
    Build(PluginBuildPlan),
}
```

The runtime ID comes from the canonical manifest. Build machinery does not infer plugin semantics from the output file extension.

## Build plans

Build plans exist so compiled or bundled plugin languages fit the same load path.

Examples include:

- Rust source -> WASM component;
- TypeScript source -> JavaScript bundle;
- another compiled language -> runtime-specific artifact.

A build plan is part of candidate preparation. It is not plugin lifecycle execution.

Conceptually:

```rust
struct PluginBuildPlan {
    source: BuildSource,
    steps: Vec<BuildStep>,
    artifact_output: RelativePath,
}

struct BuildStep {
    program: String,
    args: Vec<String>,
    cwd: RelativePath,
    environment: BuildEnvironment,
}
```

Do not model a build step as one shell command string. Use executable plus arguments so quoting and shell injection are not implicit semantics.

The exact build-plan schema may evolve, but it must support deterministic ordered steps and one declared final artifact output.

### Build execution boundary

The kernel owns the build transaction, request authorization, output selection, provenance, and rollback behavior.

The kernel must not gain ambient shell access to implement builds.

Actual process execution runs through an explicit build executor or host process capability under kernel policy.

Build authority is separate from the authority later granted to the plugin artifact.

A build receives only explicitly granted capabilities such as:

- read access to its source tree;
- write access to an isolated build/output tree;
- execution of allowed toolchain programs;
- optional network access when policy permits dependency fetching;
- explicit environment values.

Secrets, arbitrary host filesystem access, and network access are not implicit.

For agent-authored build requests, effective build authority must be bounded by both kernel build policy and the requesting agent's delegated authority.

## Build result and artifact identity

A successful build produces a concrete artifact before runtime validation or graph activation.

The kernel computes or records an immutable `ArtifactRevision` from the exact output artifact and relevant immutable metadata.

Graph generations pin the exact artifact revision, not merely the plugin ID or package version.

Development builds therefore do not need fake semantic version bumps.

Example:

```text
generation 41
  example.search
  artifact 8d21...

generation 42
  example.search
  artifact c770...
```

Build logs and build-plan provenance should be attached to the candidate result so agents can diagnose failures.

## Load and hot-replace transaction

A load request follows this semantic sequence:

```text
receive request
  -> authorize plugin-management request
  -> validate canonical manifest
  -> execute optional build plan
  -> identify immutable artifact revision
  -> resolve runtime provider
  -> runtime validates artifact
  -> resolve candidate component graph
  -> validate contracts and authority
  -> instantiate candidate plugin
  -> prepare candidate
  -> start candidate
  -> atomically commit graph generation
  -> route new calls to new generation
  -> quiesce retired instances
  -> drain pinned old generation
  -> stop retired instances
  -> release retired artifacts
```

No step before graph commit may make the candidate visible to ordinary component invocations.

If build, runtime validation, graph resolution, prepare, or start fails, the active generation remains unchanged.

## Unload transaction

Unload follows the same reconciliation mechanism:

```text
receive unload request
  -> authorize
  -> resolve candidate graph without plugin
  -> reject if required imports become unsatisfied
  -> commit new generation
  -> prevent new calls from entering retired plugin
  -> quiesce old instance
  -> drain old generation
  -> stop instance
  -> release artifact
```

Removing one runtime bridge also invalidates guest instances that depend on that runtime. The candidate graph must either rebind them to another compatible runtime provider or remove/reject them as required by desired state.

## Generation pinning and draining

Hot replacement must not mutate the implementation under an active invocation.

New invocations bind to the newly committed generation. Existing invocations remain pinned to the generation in which they started until they complete or are cancelled by explicit drain policy.

This applies to component calls, tool calls, model calls, agent execution steps, background tasks, and any other runtime work that may retain plugin code or handles.

The kernel owns generation retirement and decides when a retired plugin instance is safe to stop.

## Persistence

Persistent state belongs to `PluginId`, not to runtime bridge or artifact revision.

Replacing:

```text
TypeScript implementation
```

with:

```text
WASM implementation
```

may retain the same plugin ID, component contracts, configuration, and persistent namespace.

Runtime bridges only translate guest persistence calls into the kernel-owned namespace capability.

Schema migrations are coordinated by the kernel before the candidate generation commits.

## Initial implementation scope

The first implementation supports only the built-in `embedded` runtime.

The management API, lifecycle, artifact identity, graph-generation semantics, and runtime-provider boundary must nevertheless be runtime-neutral from the start.

Initial embedded loading may only activate, replace, or unload implementations whose embedded factories are already available to the running process.

Do not add native dynamic-library loading as an implicit part of `embedded`.

Agent-authored arbitrary compiled code becomes live without restarting the kernel once a suitable future runtime bridge, such as WASM or a process-backed runtime, is installed.

Build-plan support may be implemented before non-embedded runtimes. A successful build whose declared runtime is unavailable must produce a clear `RuntimeUnavailable` result and must not alter the active graph.

## Error model

Plugin management failures must identify the failed phase and preserve the previous active generation.

At minimum distinguish:

- request authorization failure;
- manifest validation failure;
- build failure;
- artifact output missing or invalid;
- runtime unavailable;
- runtime artifact validation failure;
- graph resolution failure;
- contract incompatibility;
- authority incompatibility;
- prepare failure;
- start failure;
- stale expected artifact revision;
- drain or stop failure after commit.

Post-commit retirement failures are operational errors. They do not roll the graph pointer back to a generation that has already been replaced.

## Hard invariants

Implementation must preserve these invariants:

1. There is one canonical Plugin API.
2. Core initially supports only `embedded`.
3. Future plugin runtimes are bridges into the canonical API, not parallel plugin systems.
4. Runtime bridges are ordinary plugins except for the embedded bootstrap runtime.
5. Runtime choice does not alter component contracts or grant authority.
6. Guest authority is distinct from runtime-bridge authority.
7. Canonical manifests are inspectable before executing plugin code.
8. Load, unload, replace, and reconcile are kernel-domain operations.
9. Public management requests express desired state rather than raw lifecycle transitions.
10. Build steps may be included in load requests for compiled or bundled languages.
11. Build execution is explicitly sandboxed/capability-bound and is not ambient kernel shell access.
12. Build authority is distinct from plugin runtime authority.
13. Graph generations pin exact artifact revisions and runtime providers.
14. Candidate build, validation, prepare, or start failure leaves the active generation unchanged.
15. Existing calls remain pinned to their old generation during replacement.
16. Persistence belongs to plugin identity, not execution runtime or artifact revision.
17. Every runtime dependency chain is acyclic and terminates at `embedded`.
18. Consumers cannot observe a provider's execution runtime through the component contract.

## Validation

Implementation is complete when tests prove:

- an embedded plugin can be loaded and unloaded through the kernel management interface;
- replacing an active embedded plugin creates a new graph generation;
- stale expected-revision requests are rejected;
- a failed candidate start preserves the prior generation;
- an unload that would break a required import is rejected before commit;
- old invocations remain pinned while new invocations use the replacement generation;
- build requests use structured executable/argument steps rather than shell strings;
- build authority is bounded independently from plugin authority;
- a produced artifact gets an immutable revision;
- an unavailable future runtime fails without mutating the graph;
- runtime-provider dependency cycles are rejected;
- runtime-bridge authority cannot leak to a guest plugin;
- exact-head Source, Rust, Product, and Maintenance validation passes.

## Simplification audit

Before implementation completes, check whether any lifecycle state is stored when it can be derived from graph and instance ownership, whether separate reload logic duplicates reconcile, whether build and load use unnecessary intermediate representations, and whether custom failure handling can be replaced with ordinary propagation before the graph commit boundary.
