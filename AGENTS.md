# Phenix ACP repository instructions

This repository contains the Phenix kernel, replaceable Plugin Suite, Harness product assembly, ACP protocol boundary, and backend adapters. Treat the current Rust/ACP implementation as authoritative; do not restore deleted Ratatui UI crates, Neovim plugin code, Pi-extension, TypeScript, JSONL-process, or compatibility paths.

## Source of truth

Use this order:

1. Executable Rust code and deterministic tests.
2. `README.md` for the intended architecture and subsystem boundaries.
3. `config/phenix-harness/` for the explicit example application configuration.
4. This file for repository working rules.

When documentation and code disagree, fix or remove the stale documentation in the same change.

## Architecture discipline

- `phenix-kernel` owns generic mechanisms, plugin hosting, persistence enforcement, authority attenuation, events, and tasks. It has no first-party agent-domain fallback.
- `phenix-plugin-suite` owns Phenix-specific session, context, execution, planning, workspace, routing, language, frontend, hook, job, debug, and repository-worker semantics.
- `phenix-harness` is the supported product assembly. It selects and configures plugins through ordinary kernel contracts.
- `phenix-acp` is a wire/adaptation boundary. ACP types do not own application semantics or durable state.
- The superseded `phenix-conductor` runtime is removed. Do not restore a parallel agent-domain runtime, registry, or durable schema.
- Frontends remain clients. Rendering, input handling, editor integration, and frontend packaging belong in frontend repositories.

The supported product path is kernel plus selected plugins through `phenix-harness`. Alternate or omitted services use the same plugin resolver; no kernel or compatibility registry may restore a missing first-party service.

## Change discipline

- Remove superseded APIs and compatibility paths instead of maintaining parallel versions.
- Prefer an existing platform/library abstraction when it expresses the required semantics.
- Keep semantic names even when using a library type internally.
- Make invalid runtime states difficult or impossible to represent.
- Preserve typed errors at subsystem boundaries; do not collapse actionable failures into generic exit states.
- Add focused regression tests for behavioral fixes and integration tests for cross-boundary behavior.
- Keep implementation-specific invariants close to the code/tests that enforce them instead of creating speculative design documents.

## Testing discipline

Tests validate behavior, not declarations.

- Ordinary Nix configuration is allowed to be misconfigured; the build/run that consumes it is the meaningful validation boundary.
- Do not mirror Nix options, package selections, file declarations, or literal configuration values into tests merely to assert that the source says the same thing twice.
- Direct Nix tests are appropriate when the Nix expression itself is nontrivial reusable program logic: composition libraries, transformations, generated aggregates, ordering/precedence rules, or similar machinery.
- Product checks should build, start, execute, or otherwise exercise realized outputs.
- Keep a behavior in one canonical execution layer; product derivations must not rerun the Cargo behavioral suites.
- Frontend-specific product tests belong in the frontend repository.

## Maintenance

The flake owns the development shell, Harness product packages, package smoke checks, and one declarative maintenance provider. Do not add a second development-environment lock or task graph.

The provider is exposed as `packages.<system>.phenix-maintenance`; its generated executable is `maintenance`. The Nix command tree is authoritative for command behavior and CI topology. The committed GitHub workflow is generated from that declaration and must stay synchronized with it.

```sh
nix develop
maintenance fix
maintenance all
```

Validation is separated by boundary:

- `maintenance check source`: formatting, Nix static analysis, workflow syntax/synchronization, and Cargo test-target classification;
- `maintenance check rust`: Clippy/static Rust gate;
- `maintenance test unit`: in-crate tests;
- `maintenance test doc`: Rust documentation tests;
- `maintenance test integration`: crate/API integration targets;
- `maintenance test system`: black-box Harness/process/protocol tests;
- `maintenance test product`: realized Harness and package behavior.

CI granularity is declarative. A CI-enabled maintenance command is a visible step; commands with the same `ci.stage` share a GitHub job, while distinct stages become distinct jobs. Prefer leaf commands when individual failure attribution is useful.

Every Cargo integration-test target must be explicitly classified under integration or system maintenance commands. Compiler errors, judgment-bearing lint findings, test failures, runtime failures, and Nix build failures are never auto-repaired.

## Required verification

Before considering a change complete, run the relevant focused layer while iterating and `maintenance all` before final handoff. Do not weaken a check to make transitional code pass; either fix the current implementation or remove the obsolete surface that the check was protecting.
