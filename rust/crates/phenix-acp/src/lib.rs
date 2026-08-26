#![forbid(unsafe_code)]

//! ACP interoperability boundary.
//!
//! This crate owns ACP wire translation only. `phenix-client` is the canonical
//! Phenix client/server contract, `phenix-conductor` owns the generic server,
//! and first-party plugins own agent-domain semantics. ACP must not own session,
//! execution, routing, tool, or durable state.

use agent_client_protocol::schema::v1::{ExtRequest, ExtResponse};
use phenix_client::{ClientEnvelope, ServerMessage};
use serde_json::value::to_raw_value;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;

pub use agent_client_protocol as wire;

/// Stable name used by smoke tests and diagnostics to identify this adapter.
pub const WIRE_PROTOCOL_NAME: &str = "acp";

/// ACP extension method carrying one canonical Phenix client envelope.
pub const CLIENT_ENVELOPE_METHOD: &str = "_phenix/client/envelope";

/// Encode the canonical client contract into the ACP extension transport.
pub fn encode_client_envelope(envelope: &ClientEnvelope) -> Result<ExtRequest, AdapterError> {
    let payload = to_raw_value(envelope).map_err(AdapterError::Encode)?;
    Ok(ExtRequest::new(CLIENT_ENVELOPE_METHOD, Arc::from(payload)))
}

/// Decode a canonical Phenix server message returned over ACP.
pub fn decode_server_message(response: ExtResponse) -> Result<ServerMessage, AdapterError> {
    serde_json::from_str(response.0.get()).map_err(AdapterError::Decode)
}

#[derive(Debug)]
pub enum AdapterError {
    Encode(serde_json::Error),
    Decode(serde_json::Error),
}

impl Display for AdapterError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Encode(error) => write!(
                f,
                "failed to encode Phenix client envelope for ACP: {error}"
            ),
            Self::Decode(error) => write!(
                f,
                "failed to decode Phenix server message from ACP: {error}"
            ),
        }
    }
}

impl Error for AdapterError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Encode(error) | Self::Decode(error) => Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_client::{ClientMessage, Command, Reply, ResponsePayload};

    #[test]
    fn acp_request_carries_the_canonical_client_envelope() {
        let envelope = ClientEnvelope::Command(ClientMessage {
            id: 7,
            command: Command::GetSnapshot,
        });
        let request = encode_client_envelope(&envelope).expect("encode ACP request");

        assert_eq!(request.method.as_ref(), CLIENT_ENVELOPE_METHOD);
        let decoded: ClientEnvelope =
            serde_json::from_str(request.params.get()).expect("canonical client payload");
        assert_eq!(decoded, envelope);
    }

    #[test]
    fn acp_response_decodes_to_the_canonical_server_contract() {
        let message = ServerMessage::Response {
            id: 7,
            response: ResponsePayload::Ok {
                result: Reply::Accepted,
            },
        };
        let payload = to_raw_value(&message).expect("server message JSON");
        let decoded = decode_server_message(ExtResponse::new(Arc::from(payload)))
            .expect("decode ACP response");

        assert_eq!(decoded, message);
    }
}
