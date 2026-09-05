#![forbid(unsafe_code)]

//! Transport-independent ACP translation over the fixed Phenix application interface.
//!
//! This crate owns ACP protocol semantics. Runtime state and transport lifecycle stay outside it.

mod callbacks;
mod dispatch;
mod elicitation;
mod extension_callbacks;
mod extension_dispatch;
mod extensions;
mod updates;

use phenix_core::{PluginInstance, PluginManifest};
use phenix_sdk::StaticPluginDefinition;

pub use agent_client_protocol as wire;
pub use callbacks::*;
pub use dispatch::*;
pub use elicitation::*;
pub use extensions::*;
pub use updates::*;

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
