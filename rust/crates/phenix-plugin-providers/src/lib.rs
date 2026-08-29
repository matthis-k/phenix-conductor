#![forbid(unsafe_code)]

use phenix_core::{Authority, PluginExecution, PluginId, PluginManifest};
use phenix_provider_sdk::{auth, provider, Protocol, ProviderDefinition};

pub const PROVIDERS_PLUGIN: &str = "phenix.providers";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ApiTokenAuth {
    Bearer,
    Header(&'static str),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProviderPreset {
    id: &'static str,
    endpoint: &'static str,
    protocol: Protocol,
    api_token: ApiTokenAuth,
}

impl ProviderPreset {
    const fn bearer(id: &'static str, endpoint: &'static str, protocol: Protocol) -> Self {
        Self {
            id,
            endpoint,
            protocol,
            api_token: ApiTokenAuth::Bearer,
        }
    }

    const fn header(
        id: &'static str,
        endpoint: &'static str,
        protocol: Protocol,
        header: &'static str,
    ) -> Self {
        Self {
            id,
            endpoint,
            protocol,
            api_token: ApiTokenAuth::Header(header),
        }
    }

    pub const fn id(self) -> &'static str {
        self.id
    }

    pub const fn endpoint(self) -> &'static str {
        self.endpoint
    }

    pub const fn protocol(self) -> Protocol {
        self.protocol
    }

    #[must_use]
    pub fn auth(self) -> auth::Definition {
        let api_token = match self.api_token {
            ApiTokenAuth::Bearer => auth::ApiTokenMethod::bearer(),
            ApiTokenAuth::Header(header) => {
                auth::ApiTokenMethod::header(header).expect("common provider header name is valid")
            }
        };
        auth::Definition::api_token(api_token)
    }

    #[must_use]
    pub fn definition(self) -> ProviderDefinition {
        self.definition_with_auth(self.auth())
    }

    #[must_use]
    pub fn definition_with_auth(self, auth: auth::Definition) -> ProviderDefinition {
        provider::define(self.id, self.endpoint, self.protocol, auth)
            .expect("common provider definition is valid")
    }
}

pub const COMMON_PROVIDERS: [ProviderPreset; 10] = [
    ProviderPreset::bearer(
        "phenix.provider.openai",
        "https://api.openai.com/v1",
        Protocol::OpenAiResponses,
    ),
    ProviderPreset::header(
        "phenix.provider.anthropic",
        "https://api.anthropic.com/v1",
        Protocol::AnthropicMessages,
        "x-api-key",
    ),
    ProviderPreset::bearer(
        "phenix.provider.openrouter",
        "https://openrouter.ai/api/v1",
        Protocol::OpenAiChatCompletions,
    ),
    ProviderPreset::bearer(
        "phenix.provider.groq",
        "https://api.groq.com/openai/v1",
        Protocol::OpenAiResponses,
    ),
    ProviderPreset::bearer(
        "phenix.provider.gemini",
        "https://generativelanguage.googleapis.com/v1beta/openai/",
        Protocol::OpenAiChatCompletions,
    ),
    ProviderPreset::bearer(
        "phenix.provider.deepseek",
        "https://api.deepseek.com",
        Protocol::OpenAiChatCompletions,
    ),
    ProviderPreset::bearer(
        "phenix.provider.together",
        "https://api.together.xyz/v1",
        Protocol::OpenAiChatCompletions,
    ),
    ProviderPreset::bearer(
        "phenix.provider.mistral",
        "https://api.mistral.ai/v1",
        Protocol::OpenAiChatCompletions,
    ),
    ProviderPreset::bearer(
        "phenix.provider.xai",
        "https://api.x.ai/v1",
        Protocol::OpenAiResponses,
    ),
    ProviderPreset::bearer(
        "phenix.provider.fireworks",
        "https://api.fireworks.ai/inference/v1",
        Protocol::OpenAiChatCompletions,
    ),
];

#[must_use]
pub fn common_provider_definitions() -> Vec<ProviderDefinition> {
    COMMON_PROVIDERS
        .into_iter()
        .map(ProviderPreset::definition)
        .collect()
}

#[must_use]
pub fn providers_manifest() -> PluginManifest {
    PluginManifest {
        id: PluginId::parse(PROVIDERS_PLUGIN).expect("static provider bundle id is valid"),
        version: 1,
        execution: PluginExecution::ResourceOnly,
        dependencies: COMMON_PROVIDERS
            .into_iter()
            .map(|provider| {
                PluginId::parse(provider.id()).expect("common provider plugin id is valid")
            })
            .collect(),
        services: Vec::new(),
        resource_namespaces: Vec::new(),
        maximum_authority: Authority::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn common_provider_ids_and_endpoints_are_unique() {
        let ids = COMMON_PROVIDERS
            .into_iter()
            .map(ProviderPreset::id)
            .collect::<BTreeSet<_>>();
        let endpoints = COMMON_PROVIDERS
            .into_iter()
            .map(ProviderPreset::endpoint)
            .collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), COMMON_PROVIDERS.len());
        assert_eq!(endpoints.len(), COMMON_PROVIDERS.len());
    }

    #[test]
    fn provider_bundle_depends_on_every_common_provider() {
        let expected = COMMON_PROVIDERS
            .into_iter()
            .map(|provider| provider.id().to_owned())
            .collect::<BTreeSet<_>>();
        let actual = providers_manifest()
            .dependencies
            .into_iter()
            .map(|id| id.as_str().to_owned())
            .collect::<BTreeSet<_>>();
        assert_eq!(actual, expected);
    }

    #[test]
    fn common_catalog_covers_every_builtin_protocol() {
        let protocols = COMMON_PROVIDERS
            .into_iter()
            .map(ProviderPreset::protocol)
            .collect::<Vec<_>>();
        for protocol in [
            Protocol::AnthropicMessages,
            Protocol::OpenAiChatCompletions,
            Protocol::OpenAiResponses,
        ] {
            assert!(protocols.contains(&protocol));
        }
    }

    #[test]
    fn every_common_provider_exposes_auth_and_model_services() {
        for definition in common_provider_definitions() {
            assert_eq!(definition.manifest().services.len(), 2);
            assert_eq!(definition.component_manifest().exports.len(), 2);
        }
    }

    #[test]
    fn preset_auth_can_be_composed_with_oauth() {
        let preset = COMMON_PROVIDERS[0];
        let definition =
            preset.definition_with_auth(preset.auth().with_oauth(auth::OAuthMethod::bearer()));

        assert_eq!(
            definition.auth_kinds(),
            vec![
                phenix_provider_sdk::AuthKind::ApiToken,
                phenix_provider_sdk::AuthKind::OAuth
            ]
        );
    }
}
