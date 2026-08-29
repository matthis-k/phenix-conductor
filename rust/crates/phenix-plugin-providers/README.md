# Common providers

`phenix.providers` is a resource-only bundle for common protocol-backed model providers.

| Provider plugin | Endpoint | Protocol | API-token auth |
| --- | --- | --- | --- |
| `phenix.provider.openai` | `https://api.openai.com/v1` | OpenAI Responses | Bearer |
| `phenix.provider.anthropic` | `https://api.anthropic.com/v1` | Anthropic Messages | `x-api-key` |
| `phenix.provider.openrouter` | `https://openrouter.ai/api/v1` | OpenAI Chat Completions | Bearer |
| `phenix.provider.groq` | `https://api.groq.com/openai/v1` | OpenAI Responses | Bearer |
| `phenix.provider.gemini` | `https://generativelanguage.googleapis.com/v1beta/openai/` | OpenAI Chat Completions | Bearer |
| `phenix.provider.deepseek` | `https://api.deepseek.com` | OpenAI Chat Completions | Bearer |
| `phenix.provider.together` | `https://api.together.xyz/v1` | OpenAI Chat Completions | Bearer |
| `phenix.provider.mistral` | `https://api.mistral.ai/v1` | OpenAI Chat Completions | Bearer |
| `phenix.provider.xai` | `https://api.x.ai/v1` | OpenAI Responses | Bearer |
| `phenix.provider.fireworks` | `https://api.fireworks.ai/inference/v1` | OpenAI Chat Completions | Bearer |

The bundle contains no provider-specific request code. Each child provider is derived from its endpoint, built-in protocol adapter, and `auth::Definition` through `phenix-provider-sdk`.

For API-token providers, the frontend may submit the token itself or an environment-variable name. Use `auth::ApiToken::literal(...)` or `auth::ApiToken::env(...)`; environment references are resolved at request time and only the variable name is persisted.

A preset exposes its default auth definition through `ProviderPreset::auth()`. `definition_with_auth(...)` accepts a replacement composite definition, so additional methods such as `auth::OAuthMethod::bearer()` do not require provider-specific booleans or branches.

Use `provider::define(...)` when an endpoint needs a different protocol or auth definition. Add a `ProtocolAdapter` when the wire format itself differs.
