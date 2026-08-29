# Phenix SDK

The default plugin-author SDK re-exports the normal Phenix userspace API and provider helpers.

Define a provider from an endpoint and wire protocol:

```rust
use phenix_sdk::provider::{self, ApiTokenScheme, Protocol};

let provider = provider::new("https://api.example.com/v1")?
    .protocol(Protocol::OpenAiResponses)
    .api_token(ApiTokenScheme::Bearer)
    .oauth()
    .build("provider.example")?;
```

Compatible endpoints reuse the same protocol adapter. A different wire protocol implements `ProtocolAdapter`.

Manage credentials through the running plugin context:

```rust
use phenix_sdk::{Auth, AuthKind, ProviderSdkExt};

let provider = ctx.providers().get("provider.example")?;
provider.add_auth(auth)?;
let configured = provider.list_auth()?;
provider.remove_auth(AuthKind::ApiToken)?;
```

Provider credential state remains private to the provider. Listing returns descriptors, not tokens or secrets.
