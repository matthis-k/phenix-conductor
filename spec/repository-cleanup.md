# Repository cleanup inventory

Status: implementation checklist. Delete this file when the checklist is complete.

## Rule

Keep a repository artifact only when it provides at least one of:

- runtime or product functionality;
- supported API or configuration ergonomics;
- developer workflow ergonomics;
- behavioral regression protection;
- reproducible builds or packaging;
- required legal or attribution metadata.

Git history is the archive. Completed migration plans, superseded architecture proposals, historical PR narratives, declaration mirrors, and checks that only prove old names stayed deleted should not remain in the working tree.

Do not remove executable behavior, public ergonomics, behavioral tests, lockfiles, required notices/licenses, active workflows, or active build/package definitions merely because they are old or small.

## Delete completed or superseded documentation

Delete these files outright unless implementation finds a current invariant that is not represented elsewhere. Move any such invariant into the current canonical owner before deleting the file.

- [ ] `docs/command-scoped-maintenance-adoption.md`
- [ ] `spec/plugin-implementation.md`
- [ ] `spec/plugin-kernel-migration.md`
- [ ] `spec/plugin-kernel-baseline.md`
- [ ] `spec/plugin-kernel-primitives.md`
- [ ] `spec/plugins.md`
- [ ] `spec/plugin-phenix-suite.md`
- [ ] `spec/plugin-packages.md`
- [ ] `spec/plugin-service-layering-implementation.md`
- [ ] `spec/plugin-nix-packaging.md`
- [ ] `spec/runtime-redesign-v1.md`
- [ ] `spec/runtime-r11-r13-plan.md`
- [ ] `spec/runtime-execution-providers.md`
- [ ] `spec/runtime-testing.md`
- [ ] `rust/ARCHITECTURE.md`, after folding any still-current unique invariant into `README.md`
- [ ] `spec/core-default-providers.md`, after the session-ownership migration in #431 no longer depends on it

Keep live feature specifications that define functionality which is still planned or implemented. Do not delete a spec solely because code has not landed yet.

## Prune stale repository metadata

- [ ] Remove unused `.gitignore` entries from previous repository incarnations, including `/node_modules/`, `/.pi-subagents`, `.phenix-agent-state/`, `/.phenix-qa/`, and `/qa-results/` when no current producer remains.
- [ ] Remove a redundant root `/target/` ignore if `rust/target/` is the only Cargo target directory used by this repository.
- [ ] Replace or clear the stale GitHub repository description that still describes the project as wrapped OpenCode with Phenix MCP configuration.

## Remove migration-only Nix checks

- [ ] Remove `pluginOwnershipCheck` from `modules/package-sets.nix`.
- [ ] Remove `checks.phenix-plugin-package-ownership`.
- [ ] Do not replace these with another check for historical package aliases. The dependency graph and build should fail when a real current invariant is violated.

The removed check currently only proves that old aliases such as `phenix-kernel`, `phenix-plugin-suite`, and `phenix-domain`-as-core have not reappeared.

## Remove declaration mirrors from product validation

Keep tests that execute installed behavior. Remove assertions that only restate the product declaration.

- [ ] In `modules/phenix-acp.nix`, keep the real packaged session Create/Get journey.
- [ ] Remove the hard-coded assertion that the product contains exactly 17 plugins.
- [ ] Remove the hard-coded assertion that no `phenix.basic-*` plugin is present.
- [ ] Remove hard-coded service-presence assertions when they only mirror the selected product composition.
- [ ] Where a service is product-critical, exercise its behavior instead of checking that its identifier appears in a list.

## Remove composition enumeration APIs and tests

- [ ] Remove `HarnessBuilder::basic_suite_plugin_ids()` if no production API consumer needs it.
- [ ] Remove `basic_suite_contains_only_independently_selected_basic_agent_plugins`.
- [ ] Keep `selected_suite_accepts_each_basic_plugin_id` while it proves selection behavior rather than a declaration fact.
- [ ] Keep the basic-suite persistence journey.
- [ ] Remove `HarnessBuilder::default_suite_plugin_ids()` if it remains test-only after consumers are adjusted.
- [ ] Remove the initial supported-product assertion that the configured plugin set equals a separately enumerated expected set.
- [ ] Keep supported-product calls that actually invoke sessions, session-tree, artifacts, context, execution, language, planning, models, jobs, hooks, workspace, debug, and other product behavior.
- [ ] In replacement tests, remove final plugin-name membership assertions when omission failure and successful replacement invocation already prove the behavior.

## Reduce documentation to current sources of truth

### `README.md`

Keep current architecture, ownership, public package/configuration API, product composition, and development entry points.

- [ ] Remove historical repository-renaming commentary.
- [ ] Remove migration instructions that only say obsolete contracts should be replaced.
- [ ] Remove the list of superseded package names once no active cleanup work depends on it.
- [ ] Remove duplicate frontend-repository ownership prose.
- [ ] Remove the `rust/ARCHITECTURE.md` pointer when that duplicate document is deleted.

### `AGENTS.md`

Keep instructions that materially change agent implementation behavior.

- [ ] Keep Rust design discipline, parse-don't-validate guidance, simplification rules, testing expectations, and verification requirements.
- [ ] Remove architecture prose duplicated by `README.md`.
- [ ] Remove detailed maintenance-command topology duplicated by `DEVELOPMENT.md` or Nix.
- [ ] Replace duplicated material with concise pointers to its canonical owner where a pointer improves ergonomics.
- [ ] Remove references to superseded package names when they no longer protect active work.

### `DEVELOPMENT.md`

Keep the human-facing development command surface and test-boundary guidance.

- [ ] Remove or correct stale leaf examples such as `maintenance test integration phenix-acp-repeated-prompts` when no such current target exists.
- [ ] Remove generated CI topology claims that can drift from `modules/development.nix`, including claims about Rust steps sharing one job when the current declaration gives them separate stages.
- [ ] Prefer stable guidance about how to run and classify checks over a prose mirror of generated CI implementation details.

## Remove stale ACP naming from non-ACP product infrastructure

Do not remove ACP functionality or the real `phenix-acp` adapter crate.

- [ ] Rename `modules/phenix-acp.nix` to a product or Harness-oriented name if the module primarily packages the supported Harness product.
- [ ] Rename local product-smoke paths such as `acp-smoke.sqlite` and `acp-smoke.jsonl` when they exercise the generic Harness service boundary rather than ACP.
- [ ] Replace descriptions such as `Phenix ACP maintenance` and `phenix-acp-dev` when the surrounding tooling is repository-wide.
- [ ] Rename product validation leaves that are labeled `phenix-acp` while actually invoking `phenix-product-smoke`.

## Keep unless a separate reachability audit proves otherwise

The initial scan found no clearly orphaned whole Rust implementation crate, active Nix module, or GitHub workflow. Do not delete these by category.

Keep:

- Rust implementations that are reachable from the workspace and provide runtime/API behavior;
- behavioral, authority, persistence, protocol, replacement, omission, recovery, and packaging tests;
- `.github/workflows/ci.yml`, `.github/workflows/sync-maintenance.yml`, and `.github/workflows/worker-executor.yml` while they remain active;
- all Nix modules imported by `flake.nix` while they expose current behavior or development ergonomics;
- `Cargo.lock` and `flake.lock`;
- legal notices and licenses;
- skill resources while they are part of the supported product;
- Stitch integration while it remains an exposed development/product tool;
- ergonomic API documentation such as plugin authoring documentation that explains supported usage rather than migration history.

## Follow-up reachability pass

After #431 and the deletions above:

- [ ] Re-run code search for newly unreferenced public helpers, types, test fixtures, package aliases, Nix outputs, environment variables, and documentation links.
- [ ] Remove anything that became unreachable and provides no public ergonomic contract.
- [ ] Prefer deleting an unused abstraction over retaining it for hypothetical future use.
- [ ] Do not preserve compatibility aliases in this prerelease repository without a current consumer.

## Acceptance

- [ ] Every remaining top-level file has a current functional, ergonomic, reproducibility, regression, or legal purpose.
- [ ] Every remaining spec is either a current normative contract or an active future-functionality contract.
- [ ] There is one current repository architecture source of truth.
- [ ] Documentation does not duplicate generated CI/package topology that can be derived from code.
- [ ] Tests assert behavior rather than mirroring declarations where practical.
- [ ] No migration-only grep/check remains solely to prevent already-removed names from returning.
- [ ] No stale naming describes a different architectural boundary than the code implements.
- [ ] `maintenance all` passes on the final exact head.
- [ ] Delete `spec/repository-cleanup.md` in the implementation PR that completes this checklist.
