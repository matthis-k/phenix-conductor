# Process confinement

Status: proposed

## Goal

Define the default security boundary for commands started by agents, tools, and workspace services.

Process execution must use an explicit confinement mode. A caller must not gain host access because a sandbox backend is missing or partially supported.

## Modes

| Mode | Workspace | Network | Host filesystem | Use |
| --- | --- | --- | --- | --- |
| `read_only` | Read-only | Denied unless granted | Denied | Default agent and inspection commands |
| `workspace_write` | Read-write | Denied unless granted | Denied | Commands expected to modify the workspace |
| `unrestricted` | Host policy | Host policy | Host policy | Explicit user-approved escape hatch |

`unrestricted` is never selected by fallback.

## Defaults

- New process requests use `read_only` unless the caller requests a narrower mode or has explicit authority for `workspace_write`.
- Network access is denied unless authority grants the required network capability.
- The workspace is the only project filesystem visible to confined commands.
- `.git` stays read-only unless the call has explicit Git write authority.
- Scratch storage is writable and private to the execution.
- Confined execution fails when the selected backend cannot enforce the requested mode.
- Process output is bounded. Truncation is explicit in the result.
- Cancellation terminates the process tree, not only the direct child.

## Authority

Confinement does not replace capability checks. Both must pass.

Examples:

- `workspace.read` permits a read operation. It does not permit a shell process to write the workspace.
- `workspace.write` permits writes only inside a `workspace_write` process or a direct workspace write operation.
- Network capabilities name the destinations or provider classes the process may reach. No network capability means no network.
- `unrestricted` requires a dedicated authority. Broad workspace or shell authority is insufficient.

Authority is attenuated when one execution starts another process. A child cannot regain capabilities removed by its parent.

## Backend contract

A confinement backend reports the guarantees it can enforce before process start. Resolution either selects a backend that satisfies the request or rejects the request.

Required guarantees:

- filesystem mode
- network mode
- process-tree isolation and termination
- workspace root
- scratch root

A backend may expose stronger guarantees. The caller must not depend on backend-specific behavior unless the contract names it.

## Environment

Confined commands receive a minimal runtime environment plus explicitly supplied values. Secrets are references resolved for the process, not copied into durable process metadata.

The environment must preserve enough platform configuration for normal command lookup and locale handling. The exact allowlist belongs to the platform backend.

## Result

Process results include:

- exit status or terminating signal
- bounded stdout and stderr
- whether either stream was truncated
- selected confinement mode
- backend identity
- start and finish timestamps

Sandbox diagnostics must not include secret values.

## Failure behavior

Failures are typed by stage:

1. authority denied
2. no backend satisfies the requested confinement
3. process spawn failed
4. process exceeded its execution limit
5. process cancelled
6. process exited

Only process exit is a normal completed process result. The other cases are execution failures.

## Non-goals

- Define a Linux-specific namespace or seccomp implementation.
- Define terminal persistence. See `persistent-terminals-jobs.md`.
- Define tool scheduling.
- Treat containers as the required sandbox mechanism.

## Implementation order

1. Add typed confinement request and result contracts.
2. Route workspace shell and Git process execution through the contract.
3. Add the Linux backend.
4. Add regression tests proving read-only commands cannot mutate the workspace or escape it.
5. Add explicit unrestricted execution only after confined execution is complete.
