#![forbid(unsafe_code)]
use phenix_plugin_execution::{
    ExecutionCommand, ExecutionInterface, ExecutionRecord, ExecutionResponse, ExecutionState,
};
mod component;
mod implementation {
    include!("implementation.rs");
}
pub use component::*;
pub use implementation::*;
