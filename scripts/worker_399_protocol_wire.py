from pathlib import Path

# Add a canonical generic Harness service envelope to phenix-protocol.
path = Path('rust/crates/phenix-protocol/src/lib.rs')
text = path.read_text()
needle = 'use std::fmt::{self, Display, Formatter};\n\n'
insert = r'''use std::fmt::{self, Display, Formatter};

/// Canonical frontend/service request accepted by the supported Harness process.
///
/// Authority and provider binding are intentionally absent. They are product
/// policy and cannot be supplied by an untrusted frontend request.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceRequest {
    #[serde(default)]
    pub id: Value,
    pub service: String,
    #[serde(default)]
    pub input: Value,
}

/// Canonical response produced by the supported Harness service wire.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ServiceResponse {
    Ok {
        id: Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        output: Option<Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        output_bytes: Option<Vec<u8>>,
    },
    Error {
        id: Value,
        error: String,
    },
}

impl ServiceResponse {
    #[must_use]
    pub fn json(id: Value, output: Value) -> Self {
        Self::Ok {
            id,
            output: Some(output),
            output_bytes: None,
        }
    }

    #[must_use]
    pub fn bytes(id: Value, output: Vec<u8>) -> Self {
        Self::Ok {
            id,
            output: None,
            output_bytes: Some(output),
        }
    }

    #[must_use]
    pub fn error(id: Value, error: impl Into<String>) -> Self {
        Self::Error {
            id,
            error: error.into(),
        }
    }
}

'''
if 'pub struct ServiceRequest' not in text:
    if needle not in text:
        raise SystemExit('phenix-protocol import anchor not found')
    text = text.replace(needle, insert, 1)

# Add focused serialization/authority-surface tests near the start of the test module.
test_anchor = '''mod tests {
    use super::*;
    use phenix_core::{ProviderId, RoutingProfileId};
'''
tests = r'''mod tests {
    use super::*;
    use phenix_core::{ProviderId, RoutingProfileId};

    #[test]
    fn harness_service_wire_is_protocol_owned_and_policy_neutral() {
        let request: ServiceRequest = serde_json::from_value(serde_json::json!({
            "id": 7,
            "service": "phenix.sessions@1",
            "input": {"operation": "list"}
        }))
        .unwrap();
        assert_eq!(request.id, serde_json::json!(7));
        assert_eq!(request.service, "phenix.sessions@1");
        assert_eq!(request.input["operation"], "list");

        for forbidden in ["authority", "binding"] {
            let mut value = serde_json::json!({
                "id": 7,
                "service": "phenix.sessions@1",
                "input": null
            });
            value[forbidden] = serde_json::json!("ambient");
            assert!(serde_json::from_value::<ServiceRequest>(value).is_err());
        }
    }

    #[test]
    fn harness_service_response_preserves_json_and_byte_wire_shapes() {
        assert_eq!(
            serde_json::to_value(ServiceResponse::json(
                serde_json::json!(1),
                serde_json::json!({"result": "ok"})
            ))
            .unwrap(),
            serde_json::json!({"status": "ok", "id": 1, "output": {"result": "ok"}})
        );
        assert_eq!(
            serde_json::to_value(ServiceResponse::bytes(
                serde_json::json!(2),
                vec![1, 2, 3]
            ))
            .unwrap(),
            serde_json::json!({"status": "ok", "id": 2, "output_bytes": [1, 2, 3]})
        );
        assert_eq!(
            serde_json::to_value(ServiceResponse::error(
                serde_json::json!(3),
                "denied"
            ))
            .unwrap(),
            serde_json::json!({"status": "error", "id": 3, "error": "denied"})
        );
    }
'''
if 'harness_service_wire_is_protocol_owned_and_policy_neutral' not in text:
    if test_anchor not in text:
        raise SystemExit('protocol test anchor not found')
    text = text.replace(test_anchor, tests, 1)
path.write_text(text)

# Make Harness use the protocol-owned envelope rather than ad-hoc JSON maps.
path = Path('rust/crates/phenix-harness/Cargo.toml')
text = path.read_text()
if 'phenix-protocol = { path = "../phenix-protocol" }' not in text:
    dep_anchor = 'phenix-plugin-suite = { path = "../phenix-plugin-suite" }\n'
    if dep_anchor not in text:
        raise SystemExit('Harness dependency anchor not found')
    text = text.replace(dep_anchor, dep_anchor + 'phenix-protocol = { path = "../phenix-protocol" }\n', 1)
path.write_text(text)

path = Path('rust/crates/phenix-harness/src/main.rs')
text = path.read_text()
if 'use phenix_protocol::{ServiceRequest, ServiceResponse};' not in text:
    anchor = 'use phenix_kernel::{\n'
    idx = text.index(anchor)
    # insert after phenix_kernel import block, before serde_json use
    serde_anchor = 'use serde_json::{json, Map, Value};\n'
    if serde_anchor not in text:
        raise SystemExit('Harness serde import anchor not found')
    text = text.replace(serde_anchor, 'use phenix_protocol::{ServiceRequest, ServiceResponse};\nuse serde_json::{json, Map, Value};\n', 1)

start = text.index('fn handle_request(harness: &mut PhenixHarness, line: &str) -> Value {\n')
end = text.index('\nfn state_path() -> Result<PathBuf, Box<dyn Error>> {', start)
replacement = r'''fn handle_request(harness: &mut PhenixHarness, line: &str) -> ServiceResponse {
    let request = match serde_json::from_str::<ServiceRequest>(line) {
        Ok(request) => request,
        Err(error) => return ServiceResponse::error(Value::Null, error.to_string()),
    };
    let id = request.id;
    let service = match ServiceId::parse(request.service) {
        Ok(service) => service,
        Err(error) => return ServiceResponse::error(id, error.to_string()),
    };
    let input = match serde_json::to_vec(&request.input) {
        Ok(input) => input,
        Err(error) => return ServiceResponse::error(id, error.to_string()),
    };

    match harness.invoke(&service, &input, &default_suite_authority(), None) {
        Ok(output) => match serde_json::from_slice::<Value>(&output) {
            Ok(output) => ServiceResponse::json(id, output),
            Err(_) => ServiceResponse::bytes(id, output),
        },
        Err(error) => ServiceResponse::error(id, error.to_string()),
    }
}
'''
text = text[:start] + replacement + text[end:]
path.write_text(text)

# Record canonical external wire ownership without claiming legacy command parity complete.
path = Path('spec/plugin-implementation.md')
text = path.read_text()
current = '''The remaining migration is compatibility removal and parity closure. The legacy conductor crate still owns duplicate domain registries/state and several canonical tests. Move or replace those journeys with Harness-owned coverage before removing the corresponding conductor paths and tables.
'''
updated = '''The remaining migration is compatibility removal and parity closure. The supported Harness JSONL service wire is owned by `phenix-protocol` and routes directly to kernel service contracts without caller-supplied authority or provider bindings. The legacy conductor crate still owns duplicate domain registries/state and several canonical tests. Move or replace those journeys with Harness-owned coverage before removing the corresponding conductor paths and tables.
'''
if current in text:
    text = text.replace(current, updated, 1)
path.write_text(text)
