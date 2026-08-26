#![forbid(unsafe_code)]
use phenix_plugin_workspace::{workspace_service, WorkspaceCommand, WorkspaceResponse};
mod implementation {
    include!("implementation.rs");
}
pub use implementation::*;
