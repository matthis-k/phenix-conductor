#![forbid(unsafe_code)]

//! Transport-independent ACP adapter boundary.
//!
//! Standard ACP dispatch is not implemented yet. This crate currently owns the
//! ACP wire dependency and a stateless runtime plugin identity only.

use phenix_core::{PluginInstance, PluginManifest};
use phenix_sdk::StaticPluginDefinition;

pub use agent_client_protocol as wire;

pub const ACP_ADAPTER_PLUGIN: &str = "phenix.adapter.acp";

#[phenix_sdk::plugin("phenix.adapter.acp")]
mod plugin {}

pub use plugin::Plugin;

#[must_use]
pub fn adapter_acp_manifest() -> PluginManifest {
    Plugin::manifest()
}

#[must_use]
pub fn adapter_acp_factory() -> Box<dyn PluginInstance> {
    let factory = Plugin::descriptor()
        .embedded_factory
        .expect("stateless ACP adapter has a generated embedded factory");
    factory()
}
