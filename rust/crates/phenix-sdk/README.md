# Phenix SDK

The default plugin-author SDK re-exports the normal Phenix userspace API and provider helpers.

Define a provider from typed protocol and auth values:

```rust
use phenix_sdk::{auth, PluginId};
use phenix_sdk::provider::{Endpoint, Protocol, ProviderDefinition};

let auth = auth::Definition::api_token(auth::ApiTokenMethod::bearer())
    .with_oauth(auth::OAuthMethod::bearer());
let provider = ProviderDefinition::new(
    PluginId::parse("provider.example")?,
    Endpoint::parse("https://api.example.com/v1")?,
    Protocol::OpenAiResponses,
    auth,
);
```

The provider definition is composite: endpoint, protocol, and authentication stay separate typed values. Constructors express the semantic choice. Builders are reserved for optional refinement, such as adding another accepted auth method.

Compatible endpoints reuse the same protocol adapter. A different wire protocol implements `ProtocolAdapter`.

Manage credentials through the running plugin context. API tokens are either literal values supplied by the frontend or environment-variable references:

```rust
use phenix_sdk::{auth, AuthKind, ProviderSdkExt};

let provider = ctx.providers().get("provider.example")?;
provider.add_auth(auth::Credential::api_token(
    auth::ApiToken::env("EXAMPLE_API_KEY")?,
))?;
let configured = provider.list_auth()?;
provider.remove_auth(AuthKind::ApiToken)?;
```

`auth::ApiToken::literal(...)` parses a directly supplied token. `auth::ApiToken::env(...)` parses an environment-variable name. Environment references resolve at request time, so the secret is not copied into Phenix credential storage.

Provider credential state remains private to the provider. Listing returns descriptors, not tokens or secrets.
