//! Application-owned payloads. Runtime service records and protocol envelopes stay at adapters.
use phenix_core::{
    CallableId, ContractId, ModelId, PhenixSchema, PhenixValue, RoutingProfileId, SessionId,
    SkillId,
};
use phenix_sdk_macros::{PhenixContract, PhenixValue};

macro_rules! record {
    ($name:ident, $id:literal, { $($field:ident: $ty:ty),* $(,)? }) => {
        #[derive(Clone, Debug, PartialEq, PhenixValue, PhenixContract)]
        #[phenix(id = $id)]
        pub struct $name { $(pub $field: $ty),* }
    };
}
macro_rules! variants {
    ($name:ident, $id:literal, { $($variant:ident $( { $($field:ident: $ty:ty),* $(,)? } )?),* $(,)? }) => {
        #[derive(Clone, Debug, PartialEq, PhenixValue, PhenixContract)]
        #[phenix(id = $id)]
        pub enum $name { $($variant $( { $($field: $ty),* } )?),* }
    };
}

mod discovery;
mod interaction;
mod session;
pub use discovery::*;
pub use interaction::*;
pub use session::*;

record!(Empty, "phenix.application.type.empty@1", {});
record!(Acknowledged, "phenix.application.type.acknowledged@1", {});

variants!(ApplicationError, "phenix.application.error@1", {
    UnsupportedCapability { capability: ContractId },
    InvalidInput { message: String },
    InvalidResponse { message: String },
    NotFound { resource: String },
    Unauthenticated { message: String },
    PermissionDenied { message: String },
    Conflict { message: String },
    Cancelled,
    Disconnected,
    Failed { message: String },
});

impl std::fmt::Display for ApplicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedCapability { capability } => {
                write!(f, "unsupported capability: {capability}")
            }
            Self::NotFound { resource } => write!(f, "resource not found: {resource}"),
            Self::InvalidInput { message } => write!(f, "invalid input: {message}"),
            Self::InvalidResponse { message } => write!(f, "invalid response: {message}"),
            Self::Unauthenticated { message } => write!(f, "authentication required: {message}"),
            Self::PermissionDenied { message } => write!(f, "permission denied: {message}"),
            Self::Conflict { message } => write!(f, "conflict: {message}"),
            Self::Failed { message } => write!(f, "application failure: {message}"),
            Self::Cancelled => f.write_str("cancelled"),
            Self::Disconnected => f.write_str("disconnected"),
        }
    }
}
impl std::error::Error for ApplicationError {}
