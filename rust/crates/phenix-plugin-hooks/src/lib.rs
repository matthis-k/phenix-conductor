#![forbid(unsafe_code)]
use phenix_plugin_context::{ContextCommand, ContextInjectionLifetime, ContextInjectionRequester};
use phenix_plugin_execution::{ExecutionCommand, ExecutionResponse};
mod component;
mod implementation {
    include!("implementation.rs");
}
pub use component::*;
pub use implementation::*;

#[cfg(test)]
mod ownership_regression;
