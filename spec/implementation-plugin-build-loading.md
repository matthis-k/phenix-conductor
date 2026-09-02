---
temporary: true
---

# Build-backed plugin loading

## Goal

Complete the artifact-build side of plugin management without changing the desired-state transaction model.

## Required changes

- Support ready artifacts and typed build plans as plugin artifact inputs.
- Model build steps as executable plus argument vector, working directory, and explicit environment. Do not use shell command strings.
- Run builds through an explicit bounded build executor or host process capability. The kernel owns orchestration, authorization, provenance, and rollback, not ambient process access.
- Keep build authority separate from resulting plugin authority and bounded by caller delegation when the request is agent-authored.
- Produce one declared artifact output and assign it an immutable `ArtifactRevision` before runtime validation.
- Preserve build provenance and diagnostics needed to explain failures.
- Resolve and validate the declared plugin runtime only after the artifact exists.
- Return `RuntimeUnavailable` without graph mutation when the artifact targets an unavailable runtime.
- Leave the active generation unchanged on build or artifact validation failure.

## Tests

Cover structured argv, authority separation, missing output, immutable revision, unavailable runtime, failed build rollback, and successful ready/build inputs sharing the same downstream activation path.

## Completion

- the build and artifact requirements in `plugin-runtime-bridges.md` are implemented and covered;
- no shell-string or second build/load lifecycle exists;
- Source, Rust, Product, and Maintenance pass on the exact head;
- delete this temporary implementation slice before merge.
