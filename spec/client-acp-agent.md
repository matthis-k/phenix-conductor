# ACP plugin

Status: specification only. Implement after repository cleanup and after the canonical local connection plugin from #436 is available.

## Goal

Make ACP support an ordinary Phenix plugin rather than a separate privileged client/package category.

The first-party package is `phenix-plugin-acp`, exposed through `phenixPlugins.${system}.acp`, with plugin identity `phenix.acp`.

The plugin owns the ACP adapter executable (`phenix-acp`) and its ACP-specific translation. It does not own Phenix sessions, execution, routing, tools, authority, or persistence.

## Migration from current packaging

The repository currently exposes ACP through the separate `phenixClients.${system}.acp` / `mkPhenixClient` path and the `phenix-acp` crate.

This implementation must migrate ACP into the ordinary plugin package set:

- move or rename the implementation to `phenix-plugin-acp` as appropriate;
- expose it as `phenixPlugins.${system}.acp`;
- package the `phenix-acp` executable as an entrypoint/resources of that plugin;
- remove `phenixClients.acp`;
- remove `mkPhenixClient` and the separate `phenixClients` package family if no current consumer remains after migration;
- do not retain compatibility aliases in this prerelease repository.

If the plugin model needs a generic way for a plugin to package executable client entrypoints, add that capability generically to plugin metadata/packaging. Do not create an ACP-specific exception.

## Plugin semantics

- Selecting `phenix.acp` makes the ACP entrypoint available through the selected Harness/plugin package.
- Omitting `phenix.acp` omits ACP support completely.
- Replacing it with another ACP plugin uses ordinary plugin selection and package composition.
- Plugin metadata declares its executable/client entrypoint and required canonical connection capability.
- Harness policy controls whether the plugin is selected and what local connection mechanism it may use.
- ACP-specific configuration is namespaced to the plugin and resolved through the canonical configuration path.

No `phenixClients`, ACP registry, ACP lifecycle path, or second plugin model may remain as the public architecture.

## Process model

```text
ACP client/editor
      |
      | ACP JSON-RPC on stdio
      v
phenix-plugin-acp / phenix-acp
      |
      | canonical phenix-client connection
      v
phenix.socket or another selected local transport
      |
      v
Phenix Harness/conductor
```

- ACP stdout is protocol-only. Diagnostics go to stderr.
- The Phenix side uses the canonical client connection exposed by the selected transport plugin; #436 is the default first-party local transport.
- One ACP process may represent multiple Phenix sessions supported by the ACP connection.
- ACP process exit disconnects the frontend. It does not delete durable Phenix sessions.

The ACP plugin must not reach into conductor internals or first-party domain plugins directly.

## ACP methods

Implement the standard methods supported by the pinned ACP protocol version before adding extensions.

Required baseline:

- `initialize`;
- `session/new`;
- `session/prompt`;
- `session/cancel`.

Implement standard session discovery/lifecycle methods when they map to the canonical Phenix contract, including list, resume/load, and close for the pinned ACP major.

Advertise only capabilities that the implementation actually supports. Do not claim filesystem, terminal, authentication, mode, configuration, image, or other capabilities until the matching behavior is wired.

Keep ACP protocol-version compatibility inside the ACP plugin. Do not leak ACP versioning into `phenix-client` or core.

## Mapping rules

- ACP session identity maps to canonical Phenix `SessionId`.
- `session/new` creates one canonical Phenix session.
- `session/prompt` submits through the canonical session command path. It must not invoke a backend or model directly.
- `session/cancel` cancels the canonical active execution associated with that ACP turn when one exists.
- Session list/resume/load use conductor-owned session state. The plugin stores no parallel transcript or session registry.
- Closing an ACP session maps to the canonical close operation. Disconnecting the ACP process does not imply close.
- ACP working-directory and workspace inputs must be translated through existing canonical target/workspace types. Reject unsupported security-relevant input instead of silently widening scope.

## Streaming and updates

Translate canonical server events into standard ACP `session/update` notifications where ACP has a matching representation.

Prefer standard ACP update kinds for assistant text, exposed reasoning, tool calls and updates, plan/progress updates, usage, and session metadata changes.

Do not synthesize information Phenix did not emit. Preserve stable Phenix identities in ACP metadata only for correlation; they are not a parallel state owner.

## Client-provided capabilities

ACP can let the Agent call back into the Client for permissions, filesystem operations, terminals, and other frontend services.

Use the canonical frontend-service request/response mechanism for those callbacks. ACP client capabilities may register connection-scoped frontend providers. The plugin must not bypass conductor authority checks by invoking editor capabilities directly from backend or domain code.

Capability advertisement must match implemented callbacks exactly.

## Authentication and routing

ACP does not own provider/model policy.

- Use canonical Phenix authentication commands where ACP has a valid mapping.
- Expose no fake ACP authentication method when Phenix has nothing usable to advertise.
- Routing/profile selection remains canonical Phenix state and policy.
- ACP modes/config options may map to existing typed Phenix settings only when semantics match.
- Do not create ACP-specific routing, authority, or credential state.

## Extensions

Use ACP extension methods only for Phenix functionality with no standard ACP representation.

The existing `_phenix/client/envelope` extension must not remain the ordinary user-facing path once standard methods cover the operation. Keep it only if a current concrete consumer still requires it; otherwise delete it.

## Regression coverage

- `phenix.acp` is selectable/omittable/replacable through ordinary plugin composition.
- Omitting the plugin removes the ACP executable/integration without changing conductor code.
- No `phenixClients.acp` public output remains after migration.
- ACP `initialize` advertises only implemented capabilities.
- `session/new` creates exactly one canonical Phenix session.
- `session/prompt` reaches the canonical submit path and streams ordered ACP updates.
- `session/cancel` cancels the matching canonical execution without closing the session.
- Session list/resume/load, when advertised, reconstruct from conductor state rather than plugin state.
- ACP disconnect followed by reconnect can resume a durable Phenix session.
- Standard ACP methods do not require `_phenix/client/envelope`.
- Unsupported security-relevant ACP inputs fail explicitly instead of widening workspace or authority.
- Frontend callbacks travel through the canonical frontend-service mechanism.
- ACP process logs never corrupt protocol stdout.

## Completion

- [ ] `phenix-plugin-acp` is an ordinary first-party plugin in `phenixPlugins`;
- [ ] the plugin owns the `phenix-acp` executable/ACP translation entrypoint;
- [ ] `phenixClients.acp` and any now-unused separate client packaging API are removed;
- [ ] selection, omission, and replacement use ordinary plugin composition;
- [ ] ordinary ACP behavior uses standard ACP methods;
- [ ] no parallel session, transcript, routing, authority, or persistence state exists;
- [ ] ACP capability advertisement matches real behavior;
- [ ] the Phenix side uses the canonical client connection from the selected transport plugin;
- [ ] exact-head Source, Rust, Product, and Maintenance validation passes.
