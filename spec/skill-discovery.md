# Skill discovery

status: specification-only

## Goal

Define how Phenix discovers, validates, selects, and activates Agent Skills without loading every skill body into model context.

Phenix follows the Agent Skills `SKILL.md` format. Discovery policy is Phenix-specific because the open format does not define installation roots or collision handling.

Canonical format: <https://agentskills.io/specification>

## Skill package

A skill is a directory with `SKILL.md` at its root. Optional `scripts/`, `references/`, `assets/`, and other files remain part of the same immutable skill revision.

Phenix validates the complete canonical Agent Skills frontmatter constraints for the supported specification revision before cataloging the skill.

Canonical frontmatter fields:

- `name` is required, 1-64 characters, and contains only lowercase ASCII letters, digits, and hyphens. It cannot start or end with a hyphen, contain consecutive hyphens, or differ from the parent directory name.
- `description` is required, non-empty, and at most 1024 characters. It describes what the skill does and when to use it.
- `license` is optional. It names the license or references a bundled license file.
- `compatibility` is optional and, when present, is 1-500 characters of environment requirements.
- `metadata` is optional and is a map from string keys to string values. This is the portable extension point for additional properties.
- `allowed-tools` is optional, experimental, and is a space-separated string of tools.

Other top-level frontmatter keys are not portable fields for this supported specification revision. Phenix-specific extensions belong under namespaced keys in `metadata` unless a later supported Agent Skills revision defines them.

Validation follows the selected Agent Skills specification revision as one contract. Implementations may use the reference validator or equivalent checks, but must not accept a looser Phenix-specific subset.

## Discovery roots

Default roots, in source order:

1. skills contributed by selected Phenix plugins
2. user-installed skills
3. workspace `.agents/skills/`

Compatibility with vendor-specific roots such as `.claude/skills/` belongs in import plugins. The default catalog uses one workspace convention rather than scanning every vendor directory.

Explicit configuration may add roots. Added roots retain source identity.

## Collision handling

Two visible skills with the same name are ambiguous.

The default resolver rejects ambiguous activation rather than choosing by root precedence. An explicit source-qualified selection may choose one revision.

This prevents a workspace skill from silently shadowing a user-installed or plugin-provided skill with the same name.

## Progressive disclosure

Skill loading has three stages.

### Discovery

Load only metadata required to decide whether the skill may apply:

- name
- description
- source
- exact revision identity
- compatibility summary

Do not load the instruction body, references, assets, or scripts into prompt context.

### Activation

Activation loads the exact `SKILL.md` body for one revision. The activated body becomes an instruction input to prompt assembly.

Activation may be requested by:

- the user
- the selected agent definition
- the model based on discovered metadata
- an orchestration definition

The resulting context record stores who requested activation and the exact revision.

### Resource use

References and assets load only when the active skill points to them or a tool explicitly requests them.

Scripts execute through normal tool and process policy. A skill does not gain execution authority because a script exists in its package.

## Selection defaults

- User-requested skills activate when valid and compatible.
- Agent-required skills activate before the first model turn.
- Other skills remain metadata-only until selected.
- Automatic model selection uses name and description only.
- A failed automatic activation returns a tool or context error to the loop. It does not silently substitute another same-name skill.

## Permissions

Skill metadata may narrow permitted tools or state compatibility requirements. It cannot grant authority.

If the Agent Skills `allowed-tools` field is present, Phenix treats it as an upper bound for that skill execution. Effective tool access is the intersection of:

- execution authority
- agent tool policy
- skill tool restriction

Missing `allowed-tools` means the skill adds no further restriction. It does not mean unrestricted tool authority.

## Revision identity

A discovered skill has an immutable content identity covering the full package, not only `SKILL.md`.

Changing instructions, scripts, references, or assets produces a new revision.

An active execution keeps using its selected exact revision. Files changing on disk may update the catalog for future activation but do not mutate an already activated skill.

## Compatibility

The Agent Skills `compatibility` field is advisory text in the open format. Phenix may derive structured checks only from metadata it explicitly owns.

A Phenix compatibility extension should be namespaced metadata rather than a change to the portable top-level format.

Potential checks include:

- required tools
- required process capabilities
- required network capabilities
- required platform

Compatibility failure prevents activation before model context is changed.

## Trust

Workspace skills are project-controlled instructions. They are not trusted to expand authority.

Skill scripts and resources use the same workspace and sandbox rules as other agent-controlled files. Loading a skill never bypasses prompt-role separation, tool permission checks, process confinement, or secret controls.

## Catalog contract

The skills plugin should expose operations equivalent to:

- list discovered skill metadata
- resolve one skill name and optional source
- activate one exact revision for an execution
- read a resource from an active exact revision

The catalog returns stable ordering by skill name then source identity.

## Non-goals

- Define a Phenix-specific replacement for `SKILL.md`.
- Copy every discovered skill body into startup context.
- Execute skill scripts directly from the catalog.
- Make vendor-specific skill roots part of core discovery.
- Allow skill metadata to grant capabilities.

## Implementation order

1. Parse and validate portable `SKILL.md` metadata.
2. Add immutable package revision identity.
3. Add plugin, user, and workspace discovery roots.
4. Expose metadata-only catalog operations.
5. Add exact-revision activation through context projection.
6. Add on-demand resource reads.
7. Add compatibility import plugins for vendor-specific roots as needed.
