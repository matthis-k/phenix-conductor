# Phenix runtime architecture

Phenix is a persistent agent runtime. The conductor owns application semantics. Frontends and backends are adapters around that runtime.

```text
frontend(s) <-> Phenix conductor <-> backend adapters <-> providers/agents
```

ACP is interoperability below the conductor. `phenix-acp` may own ACP wire types and translation. It does not own Phenix sessions, routing, orchestrations, callables, tools, persistence, workspaces, or lifecycle.

The repository is prerelease. Replace obsolete contracts instead of preserving parallel compatibility APIs.

## Process ownership

One long-lived conductor is shared by all local frontends.

Frontends own interaction state such as active session, layout, scroll position, and draft input. The conductor owns the sessions themselves. A frontend does not lock or own a Phenix session.

Multiple frontends may submit to the same session. The conductor assigns durable ingress order when it accepts a root submission. Root submissions for one session execute in that order. Different sessions may execute concurrently.

Child executions inside one root execution may execute concurrently when their dependencies and workspace authority permit it.

## Frontend service providers

A live frontend may advertise service providers that the conductor can use over that frontend's existing connection. Provider advertisements, source connection identity, pending calls, notification subscriptions, and execution-to-frontend routes are process-local state.

A frontend connection owns only the service providers it advertises. It does not gain ownership of a Phenix session. When a frontend submits a root execution, that root binds to the submitting connection for execution-scoped frontend-service routing. Descendants use the same route. Another frontend may submit to the same session, but it cannot answer service calls for that root.

Conductor-owned workspace services may instead inspect the live provider catalog, select one connection whose descriptor satisfies their required capabilities, and address that connection directly. They do not create synthetic execution ownership to reach a frontend service.

Frontend service requests use conductor-assigned correlation IDs and receive one typed success or error response. Notifications are one-way in both directions. Frontend-to-conductor notifications are accepted only from a connection that currently advertises the named provider, then carry that source connection identity to the conductor-owned subscriber. The generic transport does not persist notifications.

A frontend may replace its advertised provider set while connected. Disconnect removes its provider set, execution routes, and pending calls. Process restart removes all frontend-service state even when durable executions are restored.

Frontend services are adapters for capabilities that live in a frontend process. They do not add durable configuration, callable ownership, session ownership, or ambient IPC authority. Executions do not receive arbitrary frontend IPC; conductor-owned code decides when and how a provider is used.

## Workspace language services

The conductor owns one active language-service provider for each workspace and language-service kind. A provider may be frontend-linked or conductor-managed. The conductor selects one provider that satisfies the complete required capability set and never combines state from multiple providers.

Managed provider definitions, capability requirements, and configured preferences are immutable configuration semantics. They contribute to the compiled configuration fingerprint. Live frontend registrations, selected connection identity, managed process handles, and provider epochs are process-local workspace state.

A provider lifetime has a monotonically increasing epoch. Disconnect, replacement, capability loss, or managed process restart ends the old epoch. Work dispatched under an ended epoch fails instead of being replayed against the replacement provider.

Frontend-linked providers use the generic frontend-service catalog. Their advertised capabilities do not grant execution authority or callable delegation. Executions borrow typed conductor-owned language tools through the normal callable policy and filesystem-read authority checks; they never receive the frontend connection or raw language-server transport.

Managed language-server processes live at workspace scope and survive individual executions. A consumed language result becomes a durable observation bound to the consuming execution, provider epoch, typed operation, and exact document provenance. Diagnostics notifications update process-local current state; they become durable only when an execution consumes a diagnostic result. An ended provider epoch invalidates in-flight work instead of replaying it against a replacement provider.

`spec/language-service.md` defines provider identity, capability negotiation, selection, epochs, and failover. `spec/language-intelligence.md` defines execution operations, document synchronization, diagnostics, and consumed observations.

## Session and conversation identity

A `Session` is the long-lived Phenix conversation. Model and backend conversations are implementation details.

Changing a model or routing profile between turns keeps the Phenix conversation. The conductor reconstructs or reuses backend context without making backend session IDs part of Phenix identity.

Session lineage and execution parentage remain separate concepts.

## Workspaces

A workspace is a first-class conductor entity. A session binds to one `WorkspaceId`; forks inherit that workspace. A Git worktree is a distinct workspace even when it shares Git object storage with another worktree.

The real worktree is the source of truth.

Each execution has explicit authority. Authority dimensions are independent:

```text
filesystem: read_only | write
network: none | outbound
repository: read | write
ipc: explicit endpoint set
secrets: explicit grant set
callables: explicit allowed set
```

A child receives at most its parent's authority:

```text
child authority
  = parent delegated authority
  ∩ child configured maximum authority
  ∩ invocation restrictions
```

Runtime approval may authorize an operation within that bound. It does not increase the bound.

Worker profiles are immutable configuration metadata in `CompiledConfiguration`. A profile names one canonical agent by `CallableId` and adds an authority maximum; it does not copy the agent or create another agent registry. Worker creation resolves the profile from the parent execution's pinned configuration, uses the normal child-execution path for attenuation, and records the selected profile ID as a durable execution binding. Journal replay and SQLite persistence preserve that binding independently of the canonical agent definition.

Provider credentials are not ambient shell credentials. Execution environments contain only declared non-secret environment, execution-local temporary state, and explicitly granted secrets. Host IPC is denied unless explicitly granted.

Working-tree write authority and Git metadata write authority are separate. A normal implementer may edit files while `.git` remains read-only. Remote Git operations additionally require network and credential authority.

## Read-only execution and scratch data

Read-only agents read the real worktree. Source paths are read-only to the sandbox.

Read-only execution still needs writable temporary state. `/tmp`, execution-local HOME/XDG state, and declared repository scratch paths are writable. Repository scratch is shared by executions in one workspace and is excluded from source consistency tracking.

Projects may mark ignored build/cache paths as Phenix scratch with a namespaced Gitignore directive:

```gitignore
# phenix:scratch
/target/
```

A plain ignored path does not automatically become scratch.

Capturing attempted source writes from a read-only agent is an explicit diagnostic mode. Captured writes form an audit patch and never change the real worktree automatically. The user applies such a patch; a model may assess it only when explicitly asked.

## Workspace consistency

Workspace concurrency follows reader/writer semantics:

- any number of read-only executions may hold read leases;
- exactly one writable execution may hold the write lease;
- a workspace does not run readers and a writer at the same time.

The lease is held for the execution, not one tool call. Orchestrations should therefore consolidate useful read and write phases instead of repeatedly alternating tiny readers and writers.

Executions record file observations for authoritative source files. Observations are file-scoped, not repository-scoped. A changed `src/a.rs` invalidates claims that depended on `src/a.rs`; it does not invalidate unrelated observations of `src/b.rs`.

Native writes carry the expected file version. A stale expected version produces a typed workspace conflict instead of overwriting newer data.

Writable shell commands use a command transaction so modifications can be checked against the relevant pre-command versions before commit. Shell read tracing may remain incomplete until the filesystem runtime can report exact reads; the runtime must not claim a stronger guarantee than it enforces.

External changes are allowed. The conductor detects changed observations and invalidates affected results when needed.

## Write phases and checkpoints

Writable agents modify the real worktree. Their changes are implementation work, including when the attempt later fails. Phenix does not silently roll those changes back.

Before a contiguous write phase, the conductor records a lightweight recovery checkpoint without committing, stashing, resetting, or otherwise changing Git history. The checkpoint distinguishes pre-existing user changes from changes made by the phase.

Authority transitions normally define phase boundaries:

```text
scout/read -> plan/read -> checkpoint -> implement/write -> fixup/write -> test/read
```

An orchestration may request another checkpoint when consecutive writers represent materially different approaches.

Verification applies to the exact source versions it observed. Later changes invalidate only the affected verification dependencies.

## Delegation

Delegation is a directed allow graph over callable IDs. The compiled configuration contains concrete edges.

Typical policy:

```text
root -> any configured callable
implementer -> selected scouts/reviewers/tools
scout -> read-oriented tools and permitted read-only children
```

Tool visibility and delegation policy are separate concerns.

`ExecutionAuthority.callables` is the execution's effective delegation ceiling. Creating a child requires the child's callable ID in that set. The child then receives the intersection of that ceiling and its own configured callable authority, so no descendant can regain a callable removed by an ancestor. Root executions include every configured executable callable in their delegation ceiling. An orchestration includes its declared node callables plus the callable authority needed by those nodes' configured maxima.

A child uses its own configured maximum authority, further restricted by the parent and invocation. Authority never expands from parent to child. A read-only parent therefore cannot create a writable child.

## Orchestrations

Orchestration authoring formats are source adapters:

```text
source (Markdown / Lua object / JSON / RON)
    -> parse
OrchestrationDefinition
    -> instantiate
OrchestrationExecution
    -> NodeExecution(...)
```

The canonical orchestration runtime is a dependency graph. Sequential execution is one graph shape, not a separate runtime model.

An orchestration has typed input and output schemas. Values inside declared fields may contain prose; the structure is fixed and validated.

Data dependencies are explicit. Node transcripts are not the orchestration data model.

An orchestration may declare an interface agent. The interface agent receives typed orchestration context and outcomes, handles intelligent recovery or final synthesis, and returns data matching the orchestration output schema. It does not hide the orchestration execution tree from frontends or debug tooling.

Without an interface agent, output comes from explicit deterministic bindings and node failure follows deterministic orchestration policy.

## Failure and retry

A child failure is reported to its parent. The parent decides whether to retry, choose another child, continue where permitted, or fail itself. When a parent fails, the conductor cancels active descendants and propagates the failure upward.

The frontend records both the failure and the parent's resulting decision.

A deterministic orchestration has no hidden intelligence. A failed node fails the orchestration to its caller unless its configured interface agent handles the failure.

The first retry creates an `AttemptGroup`. The original failed execution is attempt 1. All attempts in the group share one immutable goal. A materially changed goal is a new invocation, not a retry.

Each failed attempt contributes a short summary:

```text
approach
failure_at
reason
completed_work
```

`failure_at` preserves the fact that work before the failure may still be valid. The next attempt automatically receives the invariant goal and short failure summaries, not every previous transcript. Attempts may use different models or inference settings.

Every retry is a new `ExecutionId`. Attempt identity is separate so frontends may group or expand retries as they choose.

## Routing

Routing is a deterministic conductor policy engine. Backends receive only concrete `ModelTarget` values.

Routing context may include callable, role, difficulty, required capabilities, model availability, and explicit policy constraints. The selected target is captured once for an invocation and is not recomputed by downstream layers.

## Configuration

Configuration compiles to immutable revisions. Sessions pin a revision.

Reload creates a new revision. Existing sessions do not silently change meaning. New sessions use the current revision. An explicit rebase operation may move an existing session to a newer compatible revision. Existing executions keep the revision they started with.

## Lifecycle hooks

Lifecycle hooks are immutable `CompiledConfiguration` semantics. An execution resolves them only from the configuration revision it started with. Each event uses one explicit dependency DAG; registration order is not runtime order.

The conductor owns dispatch and causal recursion guards. Hook actions request canonical conductor operations: context loading uses exact-revision injection, while callable and orchestration actions use normal policy, authority, lease, schema, sandbox, child-execution, and persistence paths. Hooks do not gain hidden side-effect authority or a second scheduler. A hook identity is executed at most once in one synchronous causal dispatch chain.

Hook metadata and warnings use canonical durable execution events. Runtime frontend connections and process handles are excluded from hook configuration identity.

## Context catalog

The conductor owns context resource identity, revision history, discovery, loading, and injection history. Skills, discoverable project documents, objectives, and plans use one catalog. Backends receive projected context bytes; backend conversation state never becomes the authoritative context store.

Each execution has one conductor-owned `ExecutionContextProjection`. `ContextManager` is the canonical projection, model-prompt rendering, inspection, and accounting boundary. `ContextRegistry` and `SkillRegistry` are immutable configuration inputs to that projection; model and provider dispatch must not independently reconstruct semantic context or maintain a parallel prompt registry.

Exact injected content is materialized from its durable resource revision, never from whatever bytes happen to be current on disk. Projection accounting is observational: it reports deterministic catalog cost, injected-content bytes, and rendered-prompt bytes without pruning, mutating durable state, or granting authority.

Durable artifacts use the same exact context-resource registry and persistence path. Promotion records immutable content under the producing execution. The execution projection exposes a compact artifact descriptor instead of copying the artifact body into model context. Deterministic pruning may remove repeated injected bytes or artifact bodies from the projection, but each omission records a typed reason, original byte count, content identity, and exact recovery reference. Pruning never mutates the journal, artifact record, or referenced content. Small tool output stays inline unless a caller explicitly promotes it.

Scoped `AGENTS.md` and `AGENTS.override.md` remain mandatory instructions. `CONTRIBUTING.md`, `DEVELOPMENT.md`, skills, objectives, and plans are discoverable resources. Loading a discoverable resource uses the canonical conductor injection operation and records the requester, exact source revision, reason, lifetime, and content identity.

Executions resolve configuration-backed resources through the immutable configuration revision they started with. Historical context requests name an exact resource revision and never fall through to current content. Discoverable document and skill bytes therefore belong to catalog revisions, not the compiled configuration fingerprint.

Context loading applies the execution's existing scope and authority. It cannot create filesystem, repository, network, IPC, secret, or callable authority. Context policy is not a second permission system.

Durable exact references identify immutable facts. A context reference must name the logical resource and immutable revision. File and language observations receive durable identity only when the conductor records them; workspace and language-service adapters supply observation data but do not mint durable IDs.

## Persistence and debug export

Each discovered workspace or worktree uses one canonical SQLite database in WAL mode. The default path is `$XDG_STATE_HOME/phenix/workspaces/<workspace-key>/workspace.db`, with the XDG user-state fallback when `XDG_STATE_HOME` is unset. The workspace key derives from the canonical `WorkspaceId`. An explicit state path changes storage location only.

The database records the canonical `WorkspaceId` and canonical root. Startup validates both before configuration binding, backend registration, or frontend service. Schema migrations are ordered and transactional. A failed migration prevents the conductor from serving the workspace. `spec/workspace-persistence.md` defines this contract.

Durable state includes sessions, lineage, configuration revisions, accepted submission order, execution metadata, canonical events, attempt groups, and other conductor-owned state required for recovery.

Live backend handles, cancellation handles, streams, sandboxes, frontend connections, and frontend service providers are process-local.

The conductor builds one canonical `SessionDebugBundle` on demand. Serializers encode that bundle; serializers do not independently query runtime state. JSON is the first encoding, not the persistence model.

The debug bundle includes session metadata, conversation, execution tree, attempts, orchestration topology, events, resolved routing, tool activity, failure decisions, workspace authority/observations, and diagnostic read-only patches. Credentials and other secrets are redacted or omitted.

## Implementation order

The runtime migration follows these dependency boundaries:

1. domain contracts for workspace authority, file observations, and retry identity;
2. first-class workspace runtime, reader/writer scheduling, guards, and checkpoints;
3. orchestration DAG, delegation, failure propagation, and retry execution;
4. sandbox enforcement for filesystem, network, IPC, repository metadata, and secrets;
5. SQLite persistence, durable multi-client ingress, configuration rebasing, and debug export.

Each slice must leave one canonical contract. Transitional duplicate APIs must not become public compatibility layers.

## Durable objectives

The conductor owns workspace objective semantics. Root objectives originate only at the explicit user-input boundary. Derived objectives may be created by conductor-mediated agent work. A derived objective is mutable only while it is a draft; activation freezes its statement and criteria. Later semantic changes create new objectives and explicit supersession rather than rewriting enacted history.

Objective creation, draft revision, criterion evidence, lifecycle transitions, and execution assignments are immutable domain events. Objective replay is validated before the aggregate runtime is restored. Completion is valid only when every required criterion has durable evidence. Lifecycle cause provenance is validated at write and replay time: execution-backed causes name recorded executions, and evidence or policy causes carry non-empty material.

Every execution created after objective semantics activate has exactly one primary objective and may have supporting objectives. Child executions inherit their parent's assignment unless an explicit conductor operation creates and assigns a more specific derived objective. On the first objective-capable operation against a pre-objective workspace journal, the conductor records the activation boundary and durably derives missing assignments from the recorded root user input and parent execution lineage. No backend conversation state is used for recovery.

SQLite schema migrations store objective facts relationally. The journal remains the ordered semantic history; the workspace database remains the authoritative durable representation and reconstructs the same typed events without JSON event replay.

## Durable plans

The conductor owns workspace plan semantics. A plan describes intended strategy and never carries model targets, callable bindings, authority, retries, timeouts, or scheduling policy. Those remain execution and orchestration concerns.

Plan drafts are prospective and revisioned. Draft updates use optimistic concurrency. The first execution-to-step assignment enacts the plan and freezes that revision. Later strategy changes create a successor plan instead of rewriting enacted steps.

Plan steps form an acyclic dependency graph and may reference objectives. Plan failure means the strategy was attempted and did not succeed. Plan invalidation means later evidence disproved an assumption. Backtracking records the old plan outcome, its typed cause, and a successor plan; it does not restore workspace files.

Plan creation, draft revision, enactment, lifecycle transitions, and execution-step assignments are durable domain events. Replay must reject stale draft revisions, dependency cycles, mutation after enactment, invalid step links, execution reassignment, invalid successor state, and invalid transition causes. Live mutations enforce the same invariants before recording an event. SQLite stores the same facts relationally so restart reconstructs the same plan history without backend conversation state.

`spec/plans.md` is the normative plan lifecycle contract.

### Context budgeting and compaction

The conductor owns per-execution context budgeting, pressure decisions, and durable checkpoints. Category demand comes from the canonical execution context projection. Deterministic pruning runs before model-backed compaction. The compactor receives a typed immutable request and returns a typed summary; it receives no runtime, workspace, tool, or semantic mutation authority. Checkpoints persist exact raw journal ranges and retained exact references. Repeated compaction may consume the previous summary, but it carries the union of raw covered ranges forward. Backend context overflow enters the same explicit management path before a bounded retry. Token pressure never creates child executions by itself.


## Durable decisions and searchable history

The conductor owns semantic decisions as durable workspace state. A draft may be revised, but recording freezes its historical identity. Later changes use a new decision with an explicit supersedes or reverts relation. Decision dependencies use a validated acyclic graph. Evidence uses typed exact references.

Decision events are canonical. SQLite stores normalized event data and rebuildable FTS search data. The FTS index is derived and may be rebuilt without changing journal or relational decision state. History search defaults to the current objective and its ancestors. Whole-workspace search is an explicit scope.

Recorded decisions are discoverable context resources. The existing context catalog exposes descriptors first, and the canonical context load path resolves the exact decision body. No separate decision prompt registry exists.

## Persistent process resources

The conductor owns terminal and job identity, durable lifecycle metadata, output references, promotion, and authority provenance. Raw process and PTY handles remain runtime-local and are never reconstructed from durable state. Resources start execution-owned. Workspace lifetime requires an explicit durable promotion event. Current execution authority is always an upper bound; narrowing authority revokes a resource whose creation authority is no longer permitted. Managed language services stay in the workspace language-service subsystem rather than entering the execution job registry.
