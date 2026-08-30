#![forbid(unsafe_code)]

use phenix_core::{PluginId, PluginManifest};

mod component;
mod implementation;
mod prompt;

pub use component::*;
pub use implementation::{
    context_factory, context_service, ContextCommand, ContextDescriptor, ContextInjection,
    ContextInjectionLifetime, ContextInjectionRequester, ContextResourceKind,
    ContextResourceRevision, ContextResponse, ContextScope, ExactContextReference,
    ExecutionContextProjection, ProjectedContextEntry, RepositoryContextSource, CONTEXT_SERVICE,
};
pub use prompt::{
    assemble_prompt, PromptAssembly, PromptSection, PromptSectionKind, PromptSectionRole,
    PHENIX_HARNESS_IDENTITY,
};

/// Context loads validate execution liveness through `phenix.execution`.
/// Activation therefore depends on the execution plugin rather than failing at
/// the first load request when that service is absent.
#[must_use]
pub fn context_manifest() -> PluginManifest {
    let mut manifest = implementation::context_manifest();
    manifest.dependencies =
        vec![PluginId::parse("phenix.execution").expect("static execution plugin id is valid")];
    manifest
}

#[cfg(test)]
mod manifest_tests {
    use super::*;

    #[test]
    fn context_declares_execution_dependency() {
        assert_eq!(
            context_manifest().dependencies,
            vec![PluginId::parse("phenix.execution").unwrap()]
        );
    }
}
