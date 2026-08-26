#![forbid(unsafe_code)]
use phenix_plugin_context::{context_service, ContextCommand};
use phenix_plugin_frontend::{frontend_service, FrontendCommand};
use phenix_plugin_jobs::{job_service, JobCommand};
use phenix_plugin_models::{model_routing_service, ModelCommand};
use phenix_plugin_planning::{planning_service, PlanningCommand};
use phenix_plugin_sessions::{session_service, SessionCommand};
mod implementation {
    include!("implementation.rs");
}
pub use implementation::*;
