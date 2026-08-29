#![forbid(unsafe_code)]

mod providers;

pub use phenix_plugin_sdk::*;
pub use phenix_provider_sdk::{
    Auth, AuthDescriptor, AuthKind, ProviderAuthCommand, ProviderAuthInterface,
    ProviderAuthResponse, ProviderError, RateLimits,
};
pub use providers::{Provider, ProviderSdkError, ProviderSdkExt, Providers};

pub mod provider {
    pub use phenix_provider_sdk::provider::*;
}
