# Kernel and Phenix userspace architecture

Status: normative architecture contract.

## Purpose

Phenix follows a kernel/userspace model similar to Linux:

```text
Phenix Kernel
    +
Phenix Plugin Suite
    =
Phenix Harness
```

The kernel provides infrastructure and mechanisms. The Phenix Plugin Suite provides the agent-harness services and programs that motivated Phenix. The Harness is the opinionated product composition of both.

The Phenix Plugin Suite is first-party and canonical for the normal Phenix product, but it has no architectural privilege. Every suite component, and the entire suite, is replaceable.

A third party may run the kernel with a subset of the suite, replace individual services, or provide a completely different userspace.

Kernel-only operation is a bootable and testable infrastructure profile. It is not required to be a useful agent harness by itself.

## Kernel ownership

The kernel owns mechanisms that must remain valid regardless of which userspace is installed:

- plugin identity, registration, lifecycle, health, and host contracts;
- generic resource and runtime identity needed by kernel mechanisms;
- authority, capability grants, attenuation, and isolation enforcement;
- generic service/capability registration and deterministic provider resolution;
- blocking task execution, cancellation, worker-thread scheduling, and bounded resource ownership;
- IPC and local transport primitives;
- generic event delivery, subscriptions, ordering, recursion protection, and dispatch provenance;
- immutable kernel configuration snapshots needed to pin runtime policy;
- durable namespaces, schema registration, transactions, migrations, recovery gating, and persistence backend abstraction;
- generic provenance for kernel-mediated operations.

The kernel must not own agent-product semantics merely because Phenix ships them by default.

In particular, session, artifact, context, skill, tool, callable, orchestration, worker, objective, plan, decision, model, provider, routing, language-service, frontend-service, job, and repository-workflow semantics belong in userspace plugins.

The kernel may expose generic mechanisms used to implement those concepts. It does not define their domain models.

## Phenix Plugin Suite

The Phenix Plugin Suite is the first-party userspace that implements the normal Phenix agent harness.

It includes focused services and programs for areas such as:

- durable sessions and session trees;
- artifacts, reads, reuse, provenance, and invalidation;
- context discovery, projection, compaction, and recovery;
- skills and tool catalogs;
- callables, orchestration, workers, verification, and recovery;
- objectives, plans, decisions, and searchable history;
- filesystem, repository, shell, search, Git, and CLI services;
- model/provider integrations, authentication, routing, and backend adapters;
- language intelligence;
- frontend services;
- lifecycle automation and hooks;
- persistent terminals/jobs;
- debug and export services;
- repository-driven worker handoffs.

These are not kernel fallbacks or privileged built-ins. They are ordinary first-party plugins using the same kernel contracts available to alternate implementations.

## Harness

The Phenix Harness is the supported product composition:

```text
phenix-harness
  kernel
  + selected Phenix Plugin Suite services/programs
  + product policy
```

Harness policy decides which first-party plugins are enabled, their settings, grants, priorities, bindings, and required-service relationships.

Product assembly decides which embedded plugin factories are available in a given binary. Availability does not imply enablement or authority.

## Contribution model

The kernel contribution vocabulary stays generic:

```text
services
capability providers
resources
subscriptions/event handlers
durable schemas
persistence providers
```

A userspace service may define higher-level contracts such as skills, tools, sessions, callables, context, or artifacts. Those contracts belong to that userspace layer, not to the kernel.

A plugin may own canonical durable state for its service through a namespaced schema. The kernel persists and transacts that state without understanding its domain meaning.

## Replaceability

Replaceability is a hard architecture requirement.

- No Phenix Plugin Suite implementation receives special kernel APIs unavailable to alternatives.
- No suite plugin may depend on being named `default` to gain authority or provider priority.
- Kernel policy selects eligible providers through generic contracts.
- Replacing one service must not require kernel changes when the replacement implements the same declared contract.
- Replacing the entire Phenix Plugin Suite must leave the kernel usable as infrastructure for another suite.

First-party status affects packaging and support, not semantics or privilege.

## Durable plugin state

Plugins register namespaced durable schemas with the kernel persistence mechanism.

The kernel owns namespace isolation, transactions, migration ordering, backend dispatch, and recovery gating. Plugins own field meaning, validation, and service semantics.

Multiple plugin services may join one atomic transaction when a declared operation requires it. Atomicity does not make any participating service a kernel concept.

## Capabilities and resolution

Capabilities are versioned service contracts:

```text
artifact.read@1
git.status@1
code.symbols@1
```

The contract namespace does not imply kernel ownership of the domain.

Provider selection is deterministic:

```text
requested contract/version
  -> compatible providers
  -> available providers
  -> permission-eligible providers
  -> explicit binding, if any
  -> configured priority
  -> stable provider-ID tie-break
```

Permissions determine eligibility. Priority chooses among eligible providers. Neither manifest status nor first-party status grants authority.

## Hosting

Hosting, distribution, and product policy are separate axes.

### Embedded executable

Trusted Rust plugins may be statically linked into the assembled product as `PluginFactory` implementations. They remain userspace components architecturally even when they share a process and use direct Rust calls.

The kernel crate never depends on concrete embedded plugin crates.

### External executable

Independently supplied or security-isolated plugins use a versioned blocking local process protocol.

### Resource-only

Static skills, templates, schemas, context resources, and similar content may use manifest-backed resource packages without a fake executable.

Rust dynamic-library loading through the Rust ABI is not a plugin format.

## Repository boundary

Repository layout does not define architecture.

The target may be:

```text
phenix-ai/
  kernel/
  harness/
  plugins/
    sessions/
    artifacts/
    context/
    orchestration/
    models/
    ...
```

Kernel, Harness, and most first-party plugins may live in one repository while preserving strict dependency direction.

## Nix composition

The normal Nix interface builds the Harness:

```nix
inputs.phenix-ai.wrappers.phenix.wrap {
  inherit pkgs;
}
```

The flake should expose at least:

```text
wrappers.phenix
packages.<system>.phenix-kernel
packages.<system>.phenix-harness
packages.<system>.phenix
lib.mkPhenixPlugin
lib.mkPhenix
```

External and resource-only plugins are immutable packages. Embedded plugins are linked by product assembly and activated by `PluginId`.

## Invariants

- Kernel contains mechanisms, not Phenix agent-product policy.
- Kernel-only mode is infrastructure, not a miniature Harness.
- Phenix Harness equals kernel plus Phenix Plugin Suite plus product policy.
- The Phenix Plugin Suite is fully replaceable.
- First-party/default status grants no authority or kernel privilege.
- Agent-domain identities and semantics are owned by userspace services.
- Plugins persist service state through namespaced kernel-mediated durable data.
- Kernel never depends on concrete embedded plugin crates.
- One-process embedding does not collapse the kernel/userspace boundary.
- Repository co-location does not weaken the boundary.
- No compatibility path may reintroduce agent-domain semantics into the kernel during migration.

## Migration rule

Phenix is prerelease. Move existing agent-harness semantics out of the kernel boundary, including flat sessions, basic artifacts, explicit context, skills, tools, callables, orchestration, and model/provider behavior. Retain only the generic mechanisms those services require. Implement the normal product through focused Phenix Plugin Suite services, remove obsolete direct paths, and test alternate/mock implementations through the same contracts.