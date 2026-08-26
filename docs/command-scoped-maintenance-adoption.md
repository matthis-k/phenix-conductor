# Adopt command-scoped maintenance execution

## Dependency

Requires `matthis-k/phenix-flake-ci#1`.

The final branch must use the merged `phenix-flake-ci` revision. Temporary testing against the dependency PR head is fine while implementing, but do not leave a mutable branch reference in `flake.lock` or `flake.nix`.

## Goal

Adopt the command-scoped maintenance interface from `phenix-flake-ci#1` so narrow CI jobs realize only the tooling they can execute.

The motivating failure is the current `fix` job. A recent run requested 199 store paths, about 856 MiB downloaded and 2.8 GiB unpacked, while `fix` itself only needs formatting and static-fix tools.

## Requirements

1. Update the `phenix-flake-ci` lock to the merged implementation from the dependency PR.
2. Adapt `modules/development.nix` to any explicit command-dependency metadata introduced upstream. Existing cross-command relationships such as `rust-ci clippy -> check rust`, `rust-ci unit -> test unit`, and `rust-ci doc -> test doc` must remain explicit and correct.
3. Regenerate `.github/workflows/ci.yml` from the Nix declaration. Do not hand-maintain a divergent workflow.
4. Use command-scoped execution for individual CI jobs, including Maintenance autofix.
5. Preserve the current semantic test boundaries and the behavior of `maintenance all`, `maintenance check`, `maintenance test`, and `maintenance fix` in the development shell.
6. Keep one definition of each command. Do not copy command bodies to obtain smaller Nix closures.
7. Preserve the root `phenix-maintenance` package/app as the compatibility dispatcher while adding command-scoped package/app outputs.

## Closure validation

Validate the scoped `fix` package/app with Nix closure inspection or a clean dry run.

The result does not need to meet a fixed byte threshold. nixpkgs changes make exact sizes unstable. Instead prove that `fix` does not realize packages introduced only by other boundaries. In particular, test-only sandbox/network tools such as Bubblewrap, Slirp4netns, and Socat must not enter the `fix` closure through the maintenance dispatcher.

Observed on the implemented tree:

- scoped `fix`: 104 store paths, about 2.6 GiB;
- compatibility dispatcher: about 2.8 GiB;
- Bubblewrap, Slirp4netns, and Socat are absent from the scoped `fix` closure.

## Acceptance criteria

- [x] `flake.lock` pins the merged `phenix-flake-ci#1` implementation.
- [x] Maintenance command dependencies use the upstream explicit dependency mechanism where needed.
- [x] Generated workflow uses command-scoped execution for narrow CI jobs.
- [x] Maintenance autofix no longer realizes unrelated test/product tooling.
- [x] The root `phenix-maintenance` compatibility package/app remains available.
- [x] Closure inspection proves unrelated sandbox/test packages are absent from scoped `fix` execution.
- [ ] Source checks pass on the final exact head.
- [ ] Rust CI passes on the final exact head.
- [ ] Product checks pass on the final exact head.
- [ ] Maintenance validation passes on the final exact head.
- [x] No temporary input override or dependency-PR branch reference remains.

## Worker checklist

- [x] Read and verify the final interface implemented by `phenix-flake-ci#1` before editing this consumer.
- [x] Update the input lock and maintenance declaration.
- [x] Regenerate CI.
- [x] Inspect the scoped `fix` closure.
- [ ] Run the full validation graph on the final exact head.
- [x] Record closure evidence in the PR description and this document; record final validation when it completes.
