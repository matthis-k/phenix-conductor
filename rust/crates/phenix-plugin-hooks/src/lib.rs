#![forbid(unsafe_code)]
use phenix_plugin_context::{
    context_service, ContextCommand, ContextInjectionLifetime, ContextInjectionRequester,
};
use phenix_plugin_execution::{execution_service, ExecutionCommand, ExecutionResponse};
mod implementation {
    include!("implementation.rs");
}
pub use implementation::*;
