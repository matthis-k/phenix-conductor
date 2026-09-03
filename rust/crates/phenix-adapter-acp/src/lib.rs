#![forbid(unsafe_code)]

//! ACP interoperability boundary.
//!
//! This crate owns ACP wire translation only. `phenix-client` is an internal
//! conductor wire, `phenix-conductor` owns the generic server, and first-party
//! plugins own agent-domain semantics. ACP must not own session, execution,
//! routing, tool, or durable state.

pub use agent_client_protocol as wire;

/// Independently activatable ACP protocol adapter.
#[phenix_sdk::plugin("phenix.adapter.acp")]
pub struct Plugin;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_is_authored_as_the_canonical_runtime_plugin() {
        let manifest = <Plugin as phenix_sdk::StaticPluginDefinition>::manifest();

        assert_eq!(manifest.id.as_str(), "phenix.adapter.acp");
        assert!(matches!(
            manifest.execution,
            phenix_sdk::PluginExecution::Embedded
        ));
        assert!(manifest.dependencies.is_empty());
        assert!(manifest.resource_namespaces.is_empty());
    }
}
