# Phenix CLI service plugin

Status: implementation contract and plugin pressure test.

## Purpose

Provide baseline CLI discovery and state inspection as a Phenix Plugin Suite service. Use it to prove that normal Harness behavior lives behind ordinary replaceable service contracts without CLI-specific kernel logic.

Requires `spec/plugin-host.md` and `spec/plugin-resolution.md`.

## Ownership

The kernel owns generic capability/service registration, authority enforcement, provider resolution, host-call lifecycle, and provenance.

The Phenix CLI plugin owns CLI discovery/probing semantics and any user/agent-facing CLI tools it exposes.

Harness policy decides whether this provider is enabled, its grants, bindings, settings, and priority.

## Initial capabilities

Define narrow userspace contracts:

```text
cli.discover@1
cli.version@1
cli.auth-state@1
```

`cli.discover@1` reports whether a named executable is available and its resolved executable identity/path subject to authority policy.

`cli.version@1` reports normalized version information when a safe probe exists.

`cli.auth-state@1` reports typed authentication state for approved non-mutating probes without exposing credentials.

Initial targets may include:

```text
git
gh
jj
rg
fd
jq
nix
cargo
```

The kernel does not enumerate these tools or understand CLI semantics.

## Result model

The plugin may define:

```text
CliDescriptor
  name
  availability
  executable_identity
  version?
  auth_state?
  supported_probe_capabilities
  observation_provenance
```

Authentication state is typed: `unsupported`, `unknown`, `authenticated`, `unauthenticated`, or `error`.

Do not infer authentication from environment-variable presence when a safe authoritative status probe exists.

## Permissions

Discovery/probing uses the minimum authority required for each operation.

The plugin does not gain arbitrary shell authority merely because it runs approved probes. Probe execution must constrain executable, arguments, environment, filesystem view, network, secrets, timeout, and output.

Missing authority returns a typed limited result rather than broadening access.

## Replacement

The Phenix CLI implementation receives no default kernel priority or privilege.

An alternate provider may replace one or all CLI capabilities through Harness configuration. The kernel resolver treats first-party and alternate providers identically.

## Invariants

- No CLI-specific semantics enter the kernel.
- Normal CLI behavior is supplied by a replaceable Phenix Plugin Suite service.
- First-party status grants no authority or provider priority.
- Discovery does not grant execution authority for discovered programs.
- Authentication inspection never returns secret material.
- Missing permissions cannot become ambient host access.

## Required regressions

- CLI plugin registers through ordinary `PluginHost`;
- baseline provider is selected only because Harness policy configures it;
- alternate provider can replace one capability without kernel changes;
- discovered executable does not become invokable without explicit authority;
- version/auth probes cannot run arbitrary commands;
- no-network/no-secret grants cannot be bypassed;
- auth results contain no credential bytes;
- plugin removal removes CLI behavior without revealing a kernel fallback.