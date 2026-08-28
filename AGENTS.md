# Phenix AI repository instructions

This repository contains the generic Phenix runtime, conductor, shared client contracts, independently packaged first-party plugins, Harness product assembly, ACP boundary, and backend adapters. Treat executable Rust and deterministic tests as authoritative. Do not restore deleted UI crates, Neovim plugin code, Pi extensions, TypeScript runtime paths, or compatibility owners.

## Source of truth

Use this order:

1. Executable Rust code and deterministic tests.
2. `README.md` for current architecture and subsystem boundaries.
3. `config/phenix/` for supported product configuration and skills.
4. This file for repository working rules.

When documentation and code disagree, fix or remove the stale documentation in the same change.

## Architecture discipline

- `phenix-core` owns generic plugin mechanisms, hosting, persistence enforcement, authority attenuation, events, tasks, and service resolution. It owns no first-party agent fallback.
- `phenix-client` owns canonical client/server request, response, event, capability, and serialization contracts.
- `phenix-conductor` owns the generic configured server process and client transport. A zero-plugin conductor exposes no first-party services.
- Focused `phenix-plugin-*` crates own first-party session, context, execution, planning, workspace, routing, language, frontend-service, hook, job, debug, and repository-worker semantics.
- `phenix-plugin-catalog` is only a thin embedded-factory catalog. It owns no durable state or product policy.
- `phenix-harness` owns supported product assembly, plugin selection, grants, persistence location, provider policy, runtime configuration, skills, and resources.
- `phenix-acp` is an adapter to `phenix-client`. ACP types do not own application semantics or durable state.
- Frontends remain clients. Rendering, input handling, editor integration, and frontend packaging belong in frontend repositories.

The supported product path is a configured conductor plus selected plugins through `phenix-harness`. Alternate or omitted services use the ordinary plugin resolver. No core or conductor compatibility registry may restore a missing first-party service.

`phenix-kernel`, `phenix-protocol`, and `phenix-plugin-suite` are superseded package names. Remove remaining aliases instead of preserving them as compatibility surfaces.

## Change discipline

- Remove superseded APIs and compatibility paths instead of maintaining parallel versions.
- Prefer an existing platform or library abstraction when it expresses the required semantics.
- Keep semantic names even when using a library type internally.
- Make invalid states unrepresentable. Prefer enums, newtypes, non-zero types, ownership, and constructors over correlated strings, booleans, options, or later checks.
- Parse, don't validate. Convert external, configuration, and wire inputs into invariant-bearing types at the boundary. Once parsed, internal code assumes local invariants. Reserve resolution checks for facts that depend on other runtime state.
- Use guard clauses and early returns for errors, unsupported cases, and exceptional branches. Keep the success path shallow and linear.
- Keep the common success path short. Derive state where possible; remove intermediate representations or branches that exist only to support validation.
- Preserve typed errors at subsystem boundaries. Do not collapse actionable failures into generic exit states.
- Add focused regression tests for behavioral fixes and integration tests for cross-boundary behavior.
- Keep implementation-specific invariants close to the code and tests that enforce them instead of creating speculative documents.
- Keep plugin dependencies explicit. A plugin may depend on another plugin when the service dependency is real, but it must not load unrelated first-party implementations.

## Rust implementation discipline

Use Canonical's Rust best practices as the default style reference when they do not conflict with repository-specific architecture or established local conventions.

- Use exhaustive pattern matching on internal enums and relevant structs when new variants or fields must force a compiler error at each decision point.
- Keep foreign serialization shapes at the boundary. Deserialize into boundary types, then parse or convert into internal domain types. External formats must not dictate core representation.
- Keep production code panic-free for user, configuration, network, persistence, and runtime failures. Prefer invariant-bearing types, `?`, and typed errors. Use `expect` only for programmer-proven invariants and state the invariant in the message.
- Scope mutability to construction or the smallest block that needs it. Prefer expressions and block return values over unassigned `let` declarations and mutation used only to shuttle a value between branches.
- Keep generic bounds and lifetime parameters no broader than required. Hide incidental generic parameters with ordinary Rust API patterns when that makes the call site simpler.
- For `Result<()>` success paths, propagate fallible work with `?` and use an explicit `Ok(())` when it makes the no-information success case clear.
- Keep helper and boundary-only types in the narrowest useful scope. Promote them only when multiple callers share the same concept.

## Testing discipline

Tests validate behavior, not declarations.

- Ordinary Nix configuration is allowed to be misconfigured. The build or run that consumes it is the meaningful validation boundary.
- Do not mirror Nix options, package selections, file declarations, or literal configuration values into tests merely to assert that the source says the same thing twice.
- Direct Nix tests are appropriate when the Nix expression itself is nontrivial reusable program logic such as composition, transformation, ordering, or precedence.
- Product checks should build, start, execute, or otherwise exercise realized outputs.
- Keep a behavior in one canonical execution layer. Product derivations must not rerun the Cargo behavioral suites.
- Frontend-specific product tests belong in the frontend repository.

## Maintenance

The flake owns the development shell, product packages, package smoke checks, and one declarative maintenance provider. Do not add a second development-environment lock or task graph.

The provider is exposed as `packages.<system>.phenix-maintenance`; its generated executable is `maintenance`. The Nix command tree is authoritative for command behavior and CI topology. The committed GitHub workflow is generated from that declaration and must stay synchronized with it.

```sh
nix develop
maintenance fix
maintenance all
```

Validation is separated by boundary:

- `maintenance check source`: formatting, Nix static analysis, workflow syntax and synchronization, and Cargo test-target classification;
- `maintenance check rust`: Clippy and static Rust validation;
- `maintenance test unit`: in-crate tests;
- `maintenance test doc`: Rust documentation tests;
- `maintenance test integration`: crate and API integration targets;
- `maintenance test system`: black-box conductor, Harness, process, and protocol targets;
- `maintenance test product`: realized conductor, Harness, client, and package behavior.

CI granularity is declarative. A CI-enabled maintenance command is a visible step. Commands with the same `ci.stage` share a GitHub job, while distinct stages become distinct jobs. Prefer leaf commands when individual failure attribution is useful.

Every Cargo integration-test target must be explicitly classified under integration or system maintenance commands. Compiler errors, judgment-bearing lint findings, test failures, runtime failures, and Nix build failures are never auto-repaired.

## Required verification

Before considering a change complete, run the relevant focused layer while iterating and `maintenance all` before final handoff. Do not weaken a check to make transitional code pass. Fix the current implementation or remove the obsolete surface that the check protected.
