#![forbid(unsafe_code)]

mod providers;

pub use phenix_plugin_sdk::*;
pub use phenix_provider_sdk::{
    ApiTokenSource, Auth, AuthDescriptor, AuthKind, EnvironmentVariable, ProviderAuthCommand,
    ProviderAuthInterface, ProviderAuthResponse, ProviderError, RateLimits,
};
pub use providers::{Provider, ProviderSdkError, ProviderSdkExt, Providers};

pub mod auth {
    pub use phenix_provider_sdk::auth::*;
}

pub mod provider {
    pub use phenix_provider_sdk::provider::*;
}
