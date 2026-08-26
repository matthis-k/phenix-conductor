use agent_client_protocol::schema::v1::ExtResponse;
use phenix_acp::{decode_server_message, encode_client_envelope, CLIENT_ENVELOPE_METHOD};
use phenix_client::{
    ClientEnvelope, ClientMessage, Command, Reply, ResponsePayload, ServerMessage,
};
use serde_json::value::to_raw_value;
use std::sync::Arc;

#[test]
fn acp_package_adapts_canonical_phenix_client_contract() {
    let envelope = ClientEnvelope::Command(ClientMessage {
        id: 41,
        command: Command::GetSnapshot,
    });

    let request = encode_client_envelope(&envelope).expect("encode canonical request through ACP");
    assert_eq!(request.method.as_ref(), CLIENT_ENVELOPE_METHOD);
    let decoded_request: ClientEnvelope =
        serde_json::from_str(request.params.get()).expect("decode canonical client envelope");
    assert_eq!(decoded_request, envelope);

    let message = ServerMessage::Response {
        id: 41,
        response: ResponsePayload::Ok {
            result: Reply::Accepted,
        },
    };
    let payload = to_raw_value(&message).expect("encode canonical server response");
    let decoded_response = decode_server_message(ExtResponse::new(Arc::from(payload)))
        .expect("decode canonical server response through ACP");
    assert_eq!(decoded_response, message);
}
