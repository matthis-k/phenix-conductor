# Secrets

status: specification-only

## Goal

Define one harness-wide contract for secret references, storage, resolution, scoping, and redaction.

Provider authentication keeps owning its authentication methods. This document defines how secret material reaches providers, tools, processes, and plugins without becoming ordinary config or telemetry data.

## Rules

- Durable config stores secret references, not secret values.
- Secret values are resolved only when a call that needs them begins.
- Consumers receive only the secret values required for that call.
- Secret values never appear in debug formatting, listings, telemetry, events, errors, or persisted process metadata.
- A secret reference cannot grant authority. The caller also needs authority for the operation that consumes it.
- Missing or unavailable secret material fails the operation. It does not trigger credential guessing or fallback to a broader source.

## Secret references

Portable reference kinds:

### Environment

References one validated environment variable name.

The runtime resolves the variable for each use. The value is never copied into durable Phenix state.

This keeps the existing provider behavior for `ApiToken::env`.

### Store

References one logical entry in a selected secret-store plugin.

The reference includes store identity and entry identity. It does not expose backend-specific keyring handles to ordinary plugin code.

### Transient

Represents secret material supplied for the current runtime or interaction only.

Transient secrets must not be serialized into durable state. A frontend may use this for one-shot credentials or for handing a value to a secret store.

## Default storage

Interactive credential creation prefers a platform secret-store backend when one is available.

A deployment may explicitly select a file-backed store for portability. File-backed storage must:

- live under the platform state directory
- restrict new directories and files to the current user on platforms with Unix-style permissions
- use atomic replacement
- never share the ordinary Phenix config file

A missing secure store does not silently cause an interactive secret value to be persisted as plaintext in normal configuration.

Existing provider credential files are compatibility storage until provider credentials migrate to the common secret-store contract.

## Resolution

Resolution takes:

- exact secret reference
- requesting plugin
- execution identity when present
- effective authority
- purpose identifier

The result is a short-lived in-memory secret value scoped to the request.

The runtime should avoid APIs that list or return all secret values. Discovery returns descriptors only.

## Descriptors

A secret descriptor may contain:

- logical id
- source kind
- owning or consuming plugin when scoped
- creation and update timestamps when known
- expiry when known
- non-secret authentication kind or purpose

Descriptors never contain the secret value or a reversible representation of it.

## Authority

Use separate authority for secret administration and secret consumption.

Suggested capability classes:

- `secrets.manage`: create, replace, and remove references or stored values
- `secrets.use:<scope>`: resolve a secret for the named provider, tool, or plugin scope

Broad `secrets.manage` authority does not imply every execution may consume every secret. Agent executions should receive only the scoped `secrets.use` capabilities they need.

A child execution inherits only attenuated secret-use authority.

## Provider credentials

Provider auth definitions continue to describe accepted authentication methods such as API token or OAuth.

A configured provider credential points to secret references for sensitive fields:

- API token
- OAuth access token
- OAuth refresh token
- client secret when a flow requires one

OAuth expiry and non-secret token metadata may remain ordinary credential metadata.

Refresh writes the replacement token material back through the selected secret store. Provider code does not rewrite config files directly.

## Process injection

A process receives secrets only when its request names the required references and authority permits them.

The process backend injects the resolved values at spawn time. The process record stores the destination variable or input channel and secret reference identity, not the value.

Secrets are not inherited by unrelated child processes outside the confined process tree.

## Tools and plugins

A tool or plugin requests named secret references through its declared contract or configuration.

The host resolves the value at the last practical boundary. Generic tool catalogs and plugin graphs retain references only.

Dynamic plugin reload must not log or serialize resolved values while reconstructing state.

## Redaction

Redaction is defense in depth, not the primary storage model.

At minimum:

- secret wrapper types redact `Debug` and `Display`
- structured telemetry excludes secret fields by schema
- error conversion strips secret values
- process environment logging records names only
- HTTP diagnostics redact authorization and configured secret headers

The system should not depend on substring replacement over arbitrary logs as its only protection.

## Rotation and expiry

Replacing a stored secret keeps the logical reference stable when possible. New calls resolve the new value. Already running calls keep the value they received unless their protocol defines refresh.

Expired credentials fail with a typed authentication or credential-expired error. A provider may run an explicit refresh flow. It must not silently switch to an unrelated credential source.

## Import and compatibility

Provider-specific credential files, environment conventions, external CLIs, and OS keyrings can be adapters into the same reference contract.

Importers should preserve source identity so users can tell where a credential comes from.

Automatic import may discover descriptors. Reading secret values still requires explicit secret-use authority.

## Non-goals

- Define OAuth browser flows or token exchange protocols.
- Put secret values into `PhenixValue` for general plugin transport.
- Make one operating-system keyring mandatory.
- Treat filesystem permissions alone as a complete secret-store design.
- Let skills, tools, or plugins grant themselves secret authority.

## Implementation order

1. Add secret reference and descriptor types.
2. Separate `secrets.manage` from scoped secret-use authority.
3. Add environment and transient resolvers.
4. Add a secret-store plugin contract and platform backend.
5. Migrate provider credentials from direct value storage to references.
6. Route process and tool secret injection through the same resolver.
7. Add regression tests for serialization, debug output, telemetry, and error redaction.
