#![forbid(unsafe_code)]

//! ACP interoperability boundary.
//!
//! This crate deliberately contains no Phenix application/runtime semantics.
//! The Phenix Plugin Suite owns first-party agent-domain semantics. `phenix-harness`
//! composes those plugins over `phenix-kernel`. ACP remains one wire/adaptation
//! boundary and does not own session, execution, routing, tool, or durable state.

pub use agent_client_protocol as wire;

/// Stable name used by smoke tests and diagnostics to identify this adapter.
pub const WIRE_PROTOCOL_NAME: &str = "acp";
