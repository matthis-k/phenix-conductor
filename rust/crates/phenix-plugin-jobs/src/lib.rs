#![forbid(unsafe_code)]
mod component;
mod implementation;
pub use component::*;
pub use implementation::{job_factory, job_manifest, job_service};
pub use phenix_sdk::{
    JobCommand, JobInterface, JobResponse, RuntimeResourceKind, RuntimeResourceRecord,
    RuntimeResourceState, JOB_SERVICE,
};
