use phenix_core::{Authority, GraphGenerationId, PluginHost};

/// Kernel-scoped metadata supplied to an event listener.
///
/// Events never grant authority. The listener sees the authority already
/// resolved for its plugin and the graph generation under which it runs.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct EventContext {
    authority: Authority,
    graph_generation: Option<GraphGenerationId>,
}

impl EventContext {
    #[doc(hidden)]
    #[must_use]
    pub fn from_host(host: &PluginHost<'_>) -> Self {
        Self {
            authority: host.authority().clone(),
            graph_generation: host.graph_generation().cloned(),
        }
    }

    #[must_use]
    pub fn authority(&self) -> &Authority {
        &self.authority
    }

    #[must_use]
    pub fn graph_generation(&self) -> Option<&GraphGenerationId> {
        self.graph_generation.as_ref()
    }
}
