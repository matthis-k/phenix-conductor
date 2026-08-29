# Provider protocol SDK

Status: first-party userspace SDK contract.

## Purpose

Provider integrations are plugins. Adding an HTTP-compatible endpoint must not require changes to the kernel or model router.

The default SDK turns a compact provider description into a normal `phenix.models.inference@1` plugin:

```rust
use phenix_sdk::{auth, provider};
use phenix_sdk::provider::Protocol;

let auth = auth::Definition::api_token(auth::ApiTokenMethod::bearer())
    .with_oauth(auth::OAuthMethod::bearer());
let provider = provider::define(
    "provider.example",
    "https://api.example.com/v1",
    Protocol::OpenAiResponses,
    auth,
)?;
```

Provider construction consumes typed values directly. There is no staged provider builder. Constructors express semantic choices; builders are limited to optional refinement of an already valid value.

## Boundary

A provider definition owns:

- a parsed endpoint;
- one protocol adapter;
- one composite authentication definition;
- the derived plugin and component contracts.

Credentials are runtime data. They are not part of the provider definition.

The existing model router remains the source of truth for provider selection. A provider definition exports `phenix.models.inference@1`; routing binds that service to the selected provider plugin ID.

## Protocol adapters

A protocol adapter has one translation in each direction:

```text
ModelInferenceRequest
        |
        | encode
        v
ProviderRequest
        |
        | HTTP
        v
ProviderResponse
        |
        | decode
        v
ModelInferenceResponse
```

The first-party protocol adapters are:

- OpenAI Responses;
- OpenAI Chat Completions;
- Anthropic Messages.

A compatible endpoint normally needs only a new endpoint value and an existing adapter. A genuinely different wire protocol implements `ProtocolAdapter` instead of adding provider-specific branching to the runtime.

Protocol options pass through from the internal request. Adapter-owned required fields cannot be overridden through options.

## Parsed types

External values are parsed into invariant-bearing types at the boundary.

`Endpoint` accepts only HTTP or HTTPS base URLs. It rejects embedded credentials, queries, and fragments, and canonicalizes a trailing slash.

`Token` is non-empty and valid as an HTTP header value.

`HeaderName` is a valid HTTP header name.

`EnvironmentVariable` matches `[A-Za-z_][A-Za-z0-9_]*`.

`Secret` is non-empty.

The runtime does not revalidate these invariants.

## Authentication definition

Provider authentication is one composite value:

```rust
let auth = auth::Definition {
    api_token: Some(auth::ApiTokenMethod::bearer()),
    oauth: Some(auth::OAuthMethod::bearer()),
};
```

`auth::ApiTokenMethod` describes how an API token is applied to requests. It supports bearer authorization and a parsed custom header name. `auth::OAuthMethod` describes accepted OAuth credential semantics. The current generic runtime applies OAuth access tokens as bearer credentials.

The definition uses one optional slot per auth kind, so duplicate or contradictory declarations cannot be represented. An empty definition means the provider is unauthenticated.

## Credentials

Credentials use one wire enum:

```text
Credential
├─ ApiToken
└─ OAuth
```

API-token credential sources are created as typed values:

```rust
let from_environment = auth::ApiToken::env("EXAMPLE_API_KEY")?;
let literal = auth::ApiToken::literal("secret-from-ui")?;
```

Environment references are stored by name and resolved for each request; the referenced secret is never copied into the Phenix credential file. OAuth credentials use a bearer access token and may also contain a refresh token and expiry.

One credential of each auth kind may exist for one provider. This removes ambiguous credential selection.

The provider authentication service supports:

```text
Add(Credential)
Methods
List
Remove(AuthKind)
```

`Methods` reports whether the provider accepts API tokens, OAuth, or both so a frontend can render the correct authentication UI.

The default SDK exposes the same operations through a provider handle:

```rust
use phenix_sdk::{auth, AuthKind, ProviderSdkExt};

let provider = ctx.providers().get("provider.example")?;
provider.add_auth(auth::Credential::api_token(
    auth::ApiToken::env("EXAMPLE_API_KEY")?,
))?;
let configured = provider.list_auth()?;
provider.remove_auth(AuthKind::ApiToken)?;
```

`List` returns only credential descriptors. Secrets and tokens are never returned by the listing API or debug formatting.

OAuth is preferred over an API token when both are configured and present. An expired OAuth access token is rejected rather than silently using it. Browser authorization, token exchange, and refresh are auth-flow policy; they are not inferred from the model wire protocol.

Credentials are stored separately from provider definitions. The default file is `$XDG_STATE_HOME/phenix/provider-credentials.json`, with `PHENIX_PROVIDER_CREDENTIAL_FILE` as an override. On Unix, a newly created credential directory is restricted to `0700` and the credential file is written as `0600`; an override does not change permissions on an existing parent directory.

## Failure model

HTTP and common provider error conventions are normalized into:

```text
Authentication
Permission
NotFound
RateLimited
ContextLimit
InvalidRequest
Unavailable
Transport
Protocol
```

Rate-limit metadata recognizes common standard and provider headers for request and token limits, remaining capacity, resets, and `Retry-After`.

Reset values accept relative seconds, epoch seconds, and common compact durations such as `500ms`, `1s`, `2m`, and `1h`.

Successful responses also expose normalized rate-limit metadata in provider metadata when present.

## Authority

Generated provider plugins derive their authority from the declared behavior:

- model inference requires `network.http`;
- credential management requires `secrets.manage`.

The provider description is the source of truth for this wiring. Callers still need effective authority for the service they invoke.

## Invariants

- Provider selection remains model-router policy, not endpoint-registry policy.
- A provider cannot exist without a parsed endpoint and protocol adapter.
- Provider auth is one composite typed definition rather than independent flags.
- Runtime credentials cannot contain empty or HTTP-header-invalid tokens.
- Credential listing never returns secret material.
- Protocol adapters own wire translation. The generic provider runtime owns HTTP execution, auth application, common error normalization, and rate-limit conventions.
- A new endpoint using an existing protocol does not require a new runtime implementation.
- Provider-specific behavior must not leak into the kernel.
