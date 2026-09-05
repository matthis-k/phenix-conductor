# Workspace execution

## Goal

The workspace API should expose only operations that still benefit from typed tool boundaries.

The target agent-facing contract is:

- generic process execution in the selected workspace;
- conflict-checked patch application.

Common CLIs such as `git`, `gh`, `jj`, `rg`, `fd`, `jq`, `cargo`, `nix`, Python, Node, and package managers execute through the generic process path. The command toolbelt remains discovery metadata that tells callers which commands are available and, where useful, whether they are authenticated.

## Target contract

`WorkspaceInterface` converges on two operations.

### Exec

`Exec` runs a command in the workspace provider's execution environment.

The request carries the command plus execution metadata needed for deterministic placement, cancellation, and bounded output. Relative working directories resolve inside the selected workspace. Environment injection is explicit. The provider owns process spawning and transport.

`Exec` is location-independent. The provider may execute locally, over SSH, inside a container, VM, pod, or another process bridge. Callers do not change tools when the provider changes.

### Patch

`Patch` applies one or more file edits against explicitly observed versions.

Each affected file carries its expected version. The provider validates all expected versions before mutation. A stale version rejects the patch before any file changes. Multi-file patches use all-or-nothing validation and should commit atomically when the provider can guarantee it.

Patch responses return the resulting versions for changed files.

## Authority

Generic process execution must preserve filesystem authority.

A read-authority invocation receives a read-only workspace execution environment. A write-authority invocation may receive the writable workspace or the existing transactional overlay. The provider enforces this at the filesystem or process boundary.

Do not classify shell strings as read-only or mutating. A command such as `python`, `git`, or `sh` can perform arbitrary writes, so command inspection cannot enforce the policy.

`Patch` requires write authority.

Removing dedicated read and write operations is gated on this enforcement. Until `Exec` receives a read-only filesystem view for read authority, shell execution can bypass the existing `workspace.write` distinction.

## Placement

`Exec` and `Patch` resolve through the same workspace provider and therefore the same workspace identity.

A remote provider owns both file mutation and process execution. It must not expose remote files while spawning workspace-sensitive commands on the conductor host.

Workspace paths exposed through the contract remain relative. Provider-specific roots, mount paths, remote directories, container paths, and transport identifiers are implementation details.

## Common CLI discovery

The command toolbelt answers availability questions such as:

- whether `git` is installed;
- which version of `cargo` is available;
- whether `gh` is authenticated.

It does not provide separate execution wrappers for those commands. Probes execute through `WorkspaceInterface::Exec` so their result describes the selected workspace rather than the conductor host.

## Transitional operations

The current workspace contract also contains `Read`, `Write`, `Search`, `Shell`, and dedicated `Git` operations.

They migrate in this order:

1. Add `Exec` with explicit placement, cancellation, and bounded output semantics.
2. Route current shell behavior and command-toolbelt probes through `Exec`.
3. Remove the dedicated `Git` operation and `workspace.git` capability.
4. Add conflict-checked `Patch` while retaining the existing exact-version guarantee.
5. Make local execution enforce read-only and writable workspace views from effective authority.
6. Prove the same contract through a second provider that does not use the local filesystem and local process namespace.
7. Remove agent-facing `Read`, `Write`, and `Search` unless benchmarks show a concrete token, latency, reliability, or policy advantage.

The migration keeps compatibility only while required to land the authority boundary safely. The final contract should not retain duplicate ways to execute the same common CLI operation.

## Output and cancellation

`Exec` output is bounded. Responses report exit status, stdout, stderr, and whether either stream was truncated. Streaming transports may additionally emit ordered output updates, but the final result retains the same semantic fields.

Cancellation terminates the workspace-side process or process group. Remote providers must propagate cancellation across their transport rather than only abandoning the local request.

## Invariants

- Workspace-sensitive commands execute through the selected workspace provider.
- Read authority cannot mutate the workspace through `Exec`.
- Write authority is explicit and attenuated before provider invocation.
- `Patch` rejects stale observed versions before mutation.
- Common CLI availability metadata describes the selected workspace.
- Provider replacement can relocate execution without changing agent-facing tools.
- No dedicated CLI wrapper is added when `Exec` provides the same capability without losing structure, policy, or reliability.
