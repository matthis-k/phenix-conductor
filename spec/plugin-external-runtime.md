# External plugin runtime

status: implemented

## Purpose

Run executable plugins outside the Phenix process when they need independent distribution or enforceable isolation, while preserving the same logical plugin contracts used by embedded providers.

Requires `spec/plugin-host.md`.

## When to use external hosting

External hosting is a distribution and security boundary, not the default concurrency model.

Use it when an executable plugin needs one or more of:

- independent installation or upgrade without rebuilding the Phenix product;
- implementation in another language;
- enforceable filesystem, network, secret, IPC, crash, or memory isolation;
- an ownership/release lifecycle that should not be linked into the normal product.

First-party status does not force embedded hosting. Third-party status does not prevent a custom trusted product from embedding source at build time. The normal distribution path for independently supplied executable plugins is external hosting.

## Transport

The first external hosting mode is a local subprocess protocol over stdio or a Unix-domain socket. The transport is an adapter under `PluginHost`; it does not define alternate plugin semantics.

The host transport uses blocking reads/writes on dedicated threads. It does not require an async executor.

The protocol is typed and versioned. It carries only transport-safe values defined by the logical plugin contracts.

Rust dynamic libraries are not a supported plugin ABI. Independently distributed Rust executable plugins use this process protocol rather than `.so`, `.dylib`, or `.dll` loading. WASM is not required for the first implementation. A later WASM adapter may implement the same logical contract if a concrete need justifies it.

## Handshake

Before a plugin becomes `ready`, host and plugin exchange and validate:

- `PluginId`;
- plugin package/version identity;
- plugin protocol version;
- supported contribution/capability contract versions;
- declared capability implementations;
- runtime feature support such as cancellation or streaming;
- health/startup result.

A protocol or contract mismatch fails before contribution activation.

The runtime handshake cannot add contributions, permissions, or priority beyond the pinned configuration and validated manifest.

## Process ownership

The kernel owns external plugin process lifecycle through `PluginHost`.

Each start creates a new plugin runtime generation. Process PID, transport handles, pending requests, and cancellation handles are process-local state and are not persisted.

Unexpected exit:

- marks the generation unavailable;
- fails in-flight requests with a typed disconnect/crash error;
- releases live scopes;
- does not crash the Phenix process;
- does not automatically repeat ambiguous mutating requests through another provider.

Restart creates a new generation and re-runs handshake.

## Permission enforcement

External plugin permissions are enforced by the OS sandbox where the host claims enforcement.

At minimum map applicable plugin grants to existing Phenix isolation mechanisms for:

- filesystem access;
- repository metadata access;
- network access;
- explicit IPC endpoints;
- secrets;
- writable temporary/scratch state.

If a requested restriction cannot be enforced for the selected external hosting mode, startup fails closed instead of silently broadening host access.

Host API permission checks remain mandatory even when the OS sandbox also restricts the process.

## Environment

The plugin process receives only declared non-secret environment plus explicitly granted secrets and runtime metadata required by the protocol.

Host credentials, arbitrary HOME state, ambient agent credentials, frontend sockets, workspace write access, or network access are not inherited unless explicitly granted by the resolved plugin permission set.

## Requests

Each request carries:

- request ID;
- plugin runtime generation;
- capability/operation identity;
- contract version;
- normalized input;
- scoped host-operation token/handle identity if needed;
- cancellation correlation.

Responses echo request identity and generation. A response from an ended or mismatched generation is rejected as stale.

## Concurrency and cancellation

The transport may handle concurrent requests when the plugin declares support. Correlation is explicit; response ordering is not semantic.

Each blocked connection/request worker follows the ordinary threaded runtime model. Streaming is a sequence of correlated protocol events, not an async Rust stream.

Cancellation uses the ordinary `PluginHost` live scope. If the transport/plugin supports cancellation, the host sends a correlated cancellation request. Otherwise the host may terminate the plugin generation when policy requires hard cancellation.

A cancelled request cannot later commit a result into canonical state.

## Security boundary

External plugins are not trusted merely because they are installed or first-party. Permission grants are explicit product/user policy.

The external process can affect canonical Phenix state only through allowed protocol operations. Direct workspace, network, IPC, and secret access is additionally bounded by its sandbox.

Embedded native plugins do not provide this OS-isolation guarantee. A plugin requiring enforceable isolation must remain external.

## Invariants

- External and embedded executable plugins implement the same logical plugin contracts.
- External hosting is chosen for distribution or isolation, not ordinary concurrency.
- Protocol handshake cannot expand configuration semantics.
- External permissions are enforced both at host-operation boundaries and by OS isolation where applicable.
- Unsupported isolation fails closed.
- Plugin crash/disconnect cannot crash Phenix.
- Runtime generations make stale responses detectable.
- Restart does not restore process-local handles.
- Ambient host credentials and IPC are absent without explicit grants.
- Ambiguous mutating requests are never automatically replayed through another provider.
- External host transport does not introduce an async runtime.
- Rust dynamic libraries are not a supported external plugin format.

## Required regressions

- compatible plugin completes handshake and becomes ready;
- incompatible protocol version is rejected before contribution activation;
- handshake that advertises an undeclared capability cannot expand the pinned manifest;
- plugin process crash fails in-flight calls and leaves Phenix alive;
- restart increments generation and stale old-generation response is rejected;
- no-filesystem plugin cannot read workspace source;
- read-only plugin cannot write workspace source;
- no-network plugin cannot reach host/outbound network;
- ungranted secret is absent from plugin environment;
- explicit allowed IPC endpoint works while undeclared host IPC is unavailable;
- cancellation prevents a late external result from entering canonical state;
- external and embedded adapters pass a shared conformance fixture for the same logical capability contract;
- blocking external transport supports incremental streaming without an async host runtime.

## PR boundary

This slice adds subprocess transport, handshake, lifecycle integration, sandbox mapping, cancellation correlation, and external conformance tests. It does not add remote-network plugins, runtime package downloads, Rust dynamic-library loading, WASM, marketplace semantics, or automatic plugin updates.
