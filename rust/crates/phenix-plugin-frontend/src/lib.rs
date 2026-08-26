#![forbid(unsafe_code)]
use phenix_plugin_execution::{
    execution_service, ExecutionCommand, ExecutionRecord, ExecutionResponse, ExecutionState,
};
mod implementation {
    include!("implementation.rs");
}
pub use implementation::*;
