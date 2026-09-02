# Scheduled work

status: specification-only

## Goal

Define durable time-based execution for agents and orchestration callables.

Scheduling selects when to start work. Execution, retries, tools, models, authority, and persistence keep their existing owners.

## Schedule types

Portable schedule types:

- one-shot absolute time
- recurring wall-clock schedule

Recurring schedules use an explicit timezone and a recurrence rule. Prefer RFC 5545 recurrence semantics over cron-specific extensions so timezone and daylight-saving behavior are defined.

An absolute timestamp is stored as an instant. A wall-clock recurrence stores its timezone separately.

## Defaults

- A newly created schedule is enabled.
- Missed runs are skipped rather than backfilled.
- Only one run for one schedule may be active at a time.
- If the next occurrence arrives while the previous run is active, that occurrence is skipped.
- Scheduler-level retries are disabled. The invoked callable or execution policy owns retries.
- Disabling a schedule prevents new runs and does not cancel an already running execution.
- Schedule evaluation is durable. Restarting the process does not duplicate already claimed occurrences.

These defaults avoid catch-up storms, overlapping writers, and retry multiplication.

## Definition

A schedule definition contains:

- stable schedule id
- enabled state
- trigger
- callable id
- callable input
- requested authority
- optional session or objective association
- creation revision

Secret values are not stored in schedule definitions. Scheduled work stores secret references when its callable input requires them.

## Trigger semantics

### One-shot

A one-shot trigger has one absolute instant.

After the occurrence is claimed, the schedule becomes completed. A failed execution does not make the trigger eligible again unless an explicit retry policy creates another run.

### Recurring

A recurring trigger has:

- recurrence rule
- timezone
- optional start boundary
- optional end boundary

The scheduler derives occurrences from the rule. It persists the latest claimed occurrence so restart does not replay it.

Daylight-saving gaps skip nonexistent local times. Repeated local times execute once for each distinct instant produced by the recurrence calculation.

## Misfires

A misfire is an occurrence whose intended start passed while the scheduler was unavailable or unable to claim it.

Default policy: `skip`.

Future policy may support bounded catch-up, but catch-up must be explicit and capped. Unlimited replay of missed recurring work is invalid.

## Overlap

Default policy: `forbid`.

When an occurrence becomes due while a previous run from the same schedule is active, record the occurrence as skipped for overlap.

Future policies may support queued or concurrent runs. They are explicit schedule policy, not inferred from worker capacity.

## Authority

A schedule stores requested authority, not an immortal effective grant.

At each run:

1. resolve the callable against the current component graph
2. recompute effective authority against current policy and plugin ceilings
3. reject any authority expansion
4. create a normal execution with the resulting authority

Policy changes therefore apply to future scheduled runs. A schedule created under old policy cannot preserve removed capabilities.

## Graph changes

Schedules reference stable callable ids rather than resolved component handles.

Each occurrence resolves against the active graph generation. If the callable no longer exists or its contract is incompatible, the occurrence fails before execution starts.

Dynamic plugin reload does not mutate an already running execution.

## Claiming

Occurrence claiming must be atomic with durable scheduler state.

The scheduler records enough information to distinguish:

- pending future occurrence
- claimed occurrence
- started execution
- completed execution
- failed occurrence
- skipped misfire
- skipped overlap

A crash after claim but before execution creation must recover deterministically without starting the same occurrence twice.

## Run identity

Each claimed occurrence gets a stable run id derived from or linked to:

- schedule id
- occurrence instant
- claim sequence when needed for uniqueness

The run id is used for diagnostics and idempotent recovery. It is not a substitute for the execution id.

## Outputs

Scheduled work invokes ordinary callables. Results follow the callable's existing output contract.

A schedule may associate a run with a session, objective, or artifact policy, but the scheduler does not invent a second result store.

## Cancellation and deletion

Disabling prevents future claims.

Deleting a schedule removes future recurrence state after policy permits deletion. It does not erase execution history or domain records produced by past runs.

Cancelling an active run uses normal execution cancellation and requires its normal authority.

## Observability

Record:

- schedule created, updated, enabled, disabled, deleted
- occurrence due
- occurrence claimed
- execution started
- occurrence completed or failed
- occurrence skipped for misfire or overlap

Telemetry includes schedule and run ids. Callable inputs remain subject to normal content-capture policy.

## Condition-based work

Condition watches are not recurrence syntax.

A condition watch may use scheduled polling internally, but its contract is "run when condition becomes true" and belongs in a separate trigger plugin. The scheduler only needs to provide reliable time triggers.

## Non-goals

- Own execution retries.
- Preserve old authority after policy changes.
- Backfill every missed run by default.
- Allow overlapping recurring writes by default.
- Define event or webhook triggers.
- Store a second copy of callable outputs.

## Implementation order

1. Add schedule, trigger, and occurrence types.
2. Add durable schedule definitions and atomic occurrence claims.
3. Add one-shot execution.
4. Add recurrence and timezone handling.
5. Revalidate callable binding and authority for every run.
6. Add overlap and misfire records.
7. Add scheduler observability events.
8. Add optional condition-trigger plugins later.
