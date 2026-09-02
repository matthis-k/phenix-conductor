# Frontier model profile

## Purpose

Give strong frontier models a small coding environment. Prefer one general shell over overlapping file, search, git, and package tools. Keep context management, memory, authority, and verification owned by the conductor.

Profile ID: `frontier`.

## Model-visible tools

The model sees one tool:

```text
bash
  input:  { command: string }
  output: { exit_code: i32, stdout: string, stderr: string }
```

`bash` invokes the conductor-owned workspace shell. It does not bypass execution authority, workspace leases, sandboxing, or network policy.

Do not expose separate read, write, edit, glob, grep, git, package-install, or `nix shell` tools in this profile. The shell already composes those operations without spending model context on overlapping schemas.

Each call starts a fresh shell in the workspace root. Shell process state does not persist between calls.

## Nix environment

The shell environment provides:

- `bash`
- `nix`
- `phenix-nix-find`

Use ephemeral dependencies through `nix shell`:

```sh
nix shell nixpkgs#jq nixpkgs#ripgrep -c bash -lc 'rg TODO . | jq -R .'
```

`phenix-nix-find <query>` returns likely `nixpkgs` packages for a command or package query. The wrapper may use `nix-index`/`nix-locate` and `nix search`, but that implementation detail is not part of the model contract.

A separate model-visible Nix shell tool is unnecessary. It would duplicate shell semantics without creating a distinct authority boundary.

## Context anchors

Long-running executions keep a small pinned anchor outside compaction. It contains only state that must survive every context rewrite:

```text
objective
acceptance criteria / normative spec refs
active authority and hard constraints
current execution identity
```

Steering creates a new explicit anchor revision. Compaction never edits an anchor in place.

Place the objective and hard constraints near the start of the model context. Place the latest verified execution state near the end. This avoids burying the two most important forms of state in the middle of a long context.

## Working state

Maintain current work state as structured derived data, not as an accumulating free-form narrative:

```text
current head / source revision
completed requirements + evidence
remaining requirements
known failures / blockers
latest verification results
exact refs to relevant artifacts, logs, and decisions
```

Rebuild this state from canonical repository/execution records when compacting. Do not recursively treat the previous prose summary as the source of truth.

## Compaction

Use the order in `context-compaction.md`:

1. avoid unnecessary injection;
2. deterministic pruning;
3. structured result collapse;
4. model-backed compaction only when required.

A compacted checkpoint is derived data. It retains exact covered source ranges and references. Later compaction may reuse a checkpoint for efficiency, but summary-only ancestry is insufficient.

Prefer hierarchical progressive disclosure:

```text
pointer
  -> task/event summary
     -> selected detailed records
        -> exact durable source
```

Retrieve high-level nodes first and expand only the branches needed for the current task.

## Memory

Memory is a derived index over canonical durable sources. It is not a replacement for session, execution, repository, tool, or artifact state.

Default retrieval order:

1. scope and authority filter;
2. exact IDs, explicit links, and source refs;
3. temporal validity and supersession;
4. lexical candidates;
5. hierarchy traversal;
6. optional semantic candidates/reranking;
7. bounded expansion to exact evidence.

Semantic similarity never decides authority, validity, or truth.

Do not automatically promote model reflections or prior worker narratives into trusted long-term memory. Promotion requires evidence from canonical state or a successful external evaluation. Later tests and verifier results may label stored experiences as successful, stale, superseded, or misleading.

## Drift control

The conductor, not the model, owns drift control.

Re-ground the model from the pinned anchor and canonical working state after compaction and before a new major work phase. Do not resume from an inherited free-form trajectory alone.

Use an independent verifier at meaningful checkpoints and before completion. The verifier receives the original objective/spec, current repository state, changes, and verification output. It returns findings but cannot silently rewrite the objective or implementation.

Completion depends on external evidence such as tests, source checks, exact spec coverage, and repository state. The worker's self-report is never sufficient evidence of completion.

Track incremental progress against acceptance criteria. If progress stalls or reverses, refresh the working state from canonical sources rather than adding more reflection to the same context.

## Research basis

These rules are informed by the following results:

- Liu et al., *Lost in the Middle* (TACL 2024): relevant information is used less reliably when buried in long contexts. Keep goal/constraints and current verified state in privileged positions. https://aclanthology.org/2024.tacl-1.9/
- Packer et al., *MemGPT* (2023): hierarchical memory tiers can extend effective context by moving data between active and archival memory. https://arxiv.org/abs/2310.08560
- Sarthi et al., *RAPTOR* (ICLR 2024): recursive, tree-organized summaries improve retrieval across long material. https://proceedings.iclr.cc/paper_files/paper/2024/hash/8a2acd174940dbca361a6398a4f9df91-Abstract-Conference.html
- Wu et al., *LongMemEval* (2024): long-term memory benefits from session decomposition, richer keys, temporal query expansion, and explicit tests for knowledge updates and abstention. https://arxiv.org/abs/2410.10813
- Arike et al., *Evaluating Goal Drift in Language Model Agents* (AIES 2025): all evaluated agents showed some drift; the best scaffolded agent maintained near-perfect adherence for more than 100k tokens. Drift increased with long-context pattern matching. https://doi.org/10.1609/aies.v8i1.36541
- Menon et al., *Inherited Goal Drift* (2026): strong models can inherit drift from trajectories produced by weaker agents. Do not treat an inherited transcript as authoritative state. https://arxiv.org/abs/2603.03258
- Xiong et al., *How Memory Management Impacts LLM Agents* (ACL 2026): retrieved experiences can cause error propagation and misaligned replay; later task evaluations provide useful quality labels for memory. https://aclanthology.org/2026.acl-long.27/
- Cao et al., *HiGMem* (Findings ACL 2026): hierarchical event-first retrieval improved adversarial memory F1 while retrieving an order of magnitude fewer turns than a strong memory baseline. https://aclanthology.org/2026.findings-acl.1690/
- Ma et al., *AgentBoard* (2024): incremental progress metrics reveal long-horizon stalls that final success alone hides. https://arxiv.org/abs/2401.13178
- Shinn et al., *Reflexion* (NeurIPS 2023): external feedback stored as episodic experience can improve later attempts. In Phenix, keep that feedback evidence-backed and verifier-labeled rather than trusting self-reflection alone. https://papers.nips.cc/paper_files/paper/2023/hash/1b44b878bb782e6954cd888628510e90-Abstract-Conference.html

## Invariants

1. The frontier profile exposes one model-visible coding tool: `bash`.
2. `nix shell` and package discovery are shell capabilities, not parallel tool protocols.
3. Context compaction cannot rewrite the objective, acceptance criteria, authority, or canonical durable state.
4. Every derived summary can resolve to exact durable evidence.
5. Memory retrieval filters authority, scope, validity, and exact links before optional semantic ranking.
6. Prior trajectories are advisory evidence, never canonical state.
7. Completion requires independent evidence rather than worker self-assessment.
