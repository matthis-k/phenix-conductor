# Plugin runtimes, hot loading, and build requests

status: partial
coverage:
  - rust/crates/phenix-core/src/plugin_management_regression.rs
  - rust/crates/phenix-core/src/runtime_provider_regression.rs

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

Core exposes the implemented subset through `GraphReconciler::manage`, which resolves one candidate generation from the active desired state and commits it atomically through the existing generation machinery:

```rust
enum PluginManagementRequest {
    Load(Box<PluginLoadRequest>),
    Unload(PluginUnloadRequest),
    Reconcile(PluginSetRequest),
}
```

`Load` with an already-active `PluginId` is a replacement request. A separate semantic `reload` primitive is unnecessary.

Lifecycle methods such as `start` and `stop` are not public management operations.

### Load request

The load manifest uses the generic artifact slot to carry a ready artifact or build plan. Management normalizes it to the default concrete `PluginManifest<PluginArtifact>` before candidate resolution:

```rust
struct PluginLoadRequest {
    manifest: PluginManifest<PluginArtifactInput>,
    components: Vec<ComponentManifest>,
    expected_active_revision: Option<ArtifactRevision>,
}
```

The optional expected revision provides compare-and-swap semantics for development tools and agents. A stale request must fail rather than replacing an unexpected newer artifact.

### Unload request

```rust
struct PluginUnloadRequest {
    plugin: PluginId,
    expected_active_revision: Option<ArtifactRevision>,
}
```

Unload removes the plugin from the desired graph. It does not immediately destroy an instance that still belongs to a pinned old generation.

### Reconcile request

```rust
struct PluginSetRequest {
    plugins: Vec<PluginManifest>,
    components: Vec<ComponentManifest>,
}
```

A reconcile request supplies the desired plugin set and lets the kernel compute additions, replacements, removals, and affected runtime dependents in one transaction.

This is the canonical internal model. `load` and `unload` are convenience mutations of desired state.

## Artifact inputs

The implemented load request supports this split:

```rust
enum PluginArtifactInput {
    Ready(PluginArtifact),
    Build(PluginBuildPlan),
}
```

The runtime ID comes from the canonical manifest. Build machinery does not infer plugin semantics from the output file extension.

## Build plans

Build plans let compiled or bundled plugin languages enter the same load path.

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
    configuration: Map<String, PhenixValue>,
    requested_authority: Authority,
}

struct BuildStep {
    executable: BuildExecutable,
    argv: Vec<BuildArgument>,
    working_directory: RelativePath,
    environment: BuildEnvironment,
}
```

Do not model a build step as one shell command string. Use executable plus arguments so quoting and shell injection are not implicit semantics.

Executables, arguments, source identity/revision, environment names and values, working directories, and the single artifact output are parsed into validated types. Working directories and output paths are relative, and plans contain at least one deterministic ordered step.

### Build execution boundary

Core injects a narrow `PluginBuildExecutor` into trusted management context. The executor contract requires isolated staging, explicit environment, ordered steps, and return of only the one declared output locator and its exact bytes. It returns bounded provenance and diagnostics on success or typed failure.

The kernel must not gain ambient shell access to implement builds.

Actual process execution is outside Core and runs through that explicit executor under host policy. Core does not use a workspace shell, task runtime, backend process configuration, or artifact plugin as a build executor.

Build authority is separate from the authority later granted to the plugin artifact.

A build receives only explicitly granted capabilities such as:

- read access to its source tree;
- write access to an isolated build/output tree;
- execution of allowed toolchain programs;
- optional network access when policy permits dependency fetching;
- explicit environment values.

Secrets, arbitrary host filesystem access, and network access are not implicit.

Effective build authority is the intersection of kernel build policy, requesting caller authority, and plan-requested authority. It is separate from guest/plugin authority and runtime-provider authority. The caller authority, policy, CAS, and executor are trusted out-of-band context and are never serialized in `PluginManagementRequest`.

## Build result and artifact identity

A successful build produces a concrete artifact before runtime validation or graph activation.

Core computes `ArtifactRevision` from the exact output bytes. Its only accepted form is `sha256:<64 lowercase hexadecimal digits>`; parsing and deserialization reject every non-canonical value.

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

Bounded build provenance and diagnostics are returned in successful management results and retained in later candidate/runtime errors so callers can diagnose failures after build completion.

## Load and hot-replace transaction

The implemented load request follows this semantic sequence:

```text
receive request
  -> preflight live reconciler/kernel agreement
  -> authorize through trusted caller/policy context
  -> preflight content-addressed storage
  -> verify a ready artifact or execute the typed build plan
  -> hash and store the exact declared build output
  -> normalize to a concrete PluginArtifact
  -> resolve runtime provider and dependency order
  -> reject cycles and unavailable runtimes
  -> resolve candidate component graph
  -> validate contracts, authority, and resources
  -> instantiate and prepare candidate
  -> start candidate
  -> atomically commit graph generation
  -> stop retired instances
```

Core enforces the trusted policy and caller authority supplied out of band. Product policy chooses those values and supplies concrete CAS/executor implementations.

No step before graph commit may make the candidate visible to ordinary component invocations.

If resolution, runtime-provider validation, graph compilation, prepare, or start fails, the active generation remains unchanged.

Explicit quiesce and asynchronous drain phases are future work. The current subset stops retired instances synchronously after the new generation commits.

## Unload transaction

Unload follows the same reconciliation mechanism:

```text
receive unload request
  -> resolve candidate graph without plugin
  -> reject if required imports become unsatisfied
  -> reject if a retained guest's runtime provider is removed
  -> commit new generation
  -> stop retired instances
```

Removing one runtime bridge also invalidates guest instances that depend on that runtime. The candidate graph must either rebind them to another compatible runtime provider or remove/reject them as required by desired state.

## Generation pinning and draining

Hot replacement must not mutate the implementation under an active invocation.

New invocations bind to the newly committed generation. Existing invocations remain pinned to the generation in which they started until they complete or are cancelled by explicit drain policy.

Invocations record their starting generation in service provenance, and spawned tasks carry their starting generation in their cancellation token. The kernel owns generation retirement and stops a retired instance after the new generation commits; an explicit asynchronous drain registry is not yet implemented.

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

The implemented subset covers kernel-owned desired-state plugin management through `GraphReconciler::manage`:

- typed `PluginManagementRequest` over load, unload, and desired-set reconcile;
- `expected_active_revision` compare-and-swap on load and unload;
- ready artifacts and validated structured build plans normalized before resolution;
- management authorization and effective build authority attenuation through trusted context;
- injected CAS and isolated build-executor contracts with bounded evidence;
- core-computed immutable artifact revisions from exact output bytes;
- runtime-provider resolution with cycle and unavailable-runtime rejection before commit;
- atomic generation commit through the existing resolved-generation reconciliation path;
- synchronous retirement of stopped instances after commit;
- generation-pinned invocation provenance and task scopes.

Runtime providers are ordinary embedded plugins exporting the kernel runtime-provider service. No WASM, TypeScript, or process-backed bridge package is implemented.

Initial embedded loading activates, replaces, or unloads implementations whose embedded factories are already available to the running process.

Do not add native dynamic-library loading as an implicit part of `embedded`.

Explicit quiesce/drain retirement, concrete production build executors, and non-embedded bridge packages remain future work. A successful build whose declared runtime is unavailable produces a direct `RuntimeUnavailable` result with build evidence and does not alter the active graph.

## Error model

Plugin management failures must identify the failed phase and preserve the previous active generation.

The implemented subset distinguishes direct management phases for:

- authorization denial;
- build failure;
- missing declared output;
- invalid or unavailable CAS artifact;
- runtime unavailable;
- runtime-provider dependency cycle;
- graph resolution failure, including unsatisfied required imports;
- prepare failure;
- start failure;
- stale expected artifact revision;
- unknown unload target.

Build reports are also retained on later candidate resolution, runtime prepare, and activation failures.

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
11. Build execution is explicitly isolated/capability-bound and is not ambient kernel shell access.
12. Build authority is distinct from plugin and runtime-provider authority.
13. Graph generations pin exact artifact revisions and runtime providers.
14. Candidate validation, prepare, or start failure leaves the active generation unchanged.
15. Existing calls remain pinned to their old generation during replacement.
16. Persistence belongs to plugin identity, not execution runtime or artifact revision.
17. Every runtime dependency chain is acyclic and terminates at `embedded`.
18. Consumers cannot observe a provider's execution runtime through the component contract.

## Validation

The implemented subset is covered by `rust/crates/phenix-core/src/plugin_build_loading_regression.rs`, `rust/crates/phenix-core/src/plugin_management_regression.rs`, and `rust/crates/phenix-core/src/runtime_provider_regression.rs`, which prove:

- a runtime plugin can be loaded and unloaded through the kernel management interface;
- replacing an active plugin with a new artifact creates a new graph generation;
- stale expected-revision requests are rejected;
- a failed candidate start preserves the prior generation;
- an unload that would break a required import is rejected before commit;
- an unavailable runtime fails without mutating the graph;
- runtime-provider dependency cycles are rejected;
- removing a runtime provider with dependents is rejected;
- old invocations remain pinned while new invocations use the replacement generation;
- runtime-bridge authority cannot leak to a guest plugin;
- structured argv preserves shell metacharacters as literal arguments and cwd/environment are explicit;
- build authority is the policy/caller/request intersection and remains separate from guest authority;
- CAS preflight precedes build execution;
- missing output and failed builds preserve the active generation;
- exact output bytes deterministically produce an immutable revision;
- unavailable runtimes and runtime rejection preserve build evidence without graph mutation;
- ready and built artifacts enter the identical concrete downstream generation.

## Simplification audit

Before implementation completes, check whether any lifecycle state is stored when it can be derived from graph and instance ownership, whether separate reload logic duplicates reconcile, whether build and load use unnecessary intermediate representations, and whether custom failure handling can be replaced with ordinary propagation before the graph commit boundary.
