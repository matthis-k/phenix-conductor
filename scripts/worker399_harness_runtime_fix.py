from pathlib import Path

lib = Path("rust/crates/phenix-harness/src/lib.rs")
text = lib.read_text()
text = text.replace(
    "\nfn default_suite_authority() -> Authority {",
    "\npub fn default_suite_authority() -> Authority {",
    1,
)
lib.write_text(text)

main = Path("rust/crates/phenix-harness/src/main.rs")
text = main.read_text()
text = text.replace(
    "use phenix_harness::PhenixHarness;\nuse phenix_kernel::{Authority, CapabilityId, LocalPersistence, PluginId, ServiceId};",
    "use phenix_harness::{default_suite_authority, PhenixHarness};\nuse phenix_kernel::{LocalPersistence, ServiceId};",
    1,
)
old = '''    let authority = match request_authority(request.get("authority")) {
        Ok(authority) => authority,
        Err(error) => return json!({ "id": id, "status": "error", "error": error }),
    };
    let binding = match request.get("binding") {
        None | Some(Value::Null) => None,
        Some(Value::String(value)) => match PluginId::parse(value.clone()) {
            Ok(binding) => Some(binding),
            Err(error) => return json!({ "id": id, "status": "error", "error": error }),
        },
        Some(_) => return json!({ "id": id, "status": "error", "error": "binding must be a string or null" }),
    };

    match harness.invoke(&service, &input, &authority, binding.as_ref()) {'''
new = '''    if request.contains_key("authority") || request.contains_key("binding") {
        return json!({
            "id": id,
            "status": "error",
            "error": "authority and provider binding are owned by Harness policy",
        });
    }

    match harness.invoke(&service, &input, &default_suite_authority(), None) {'''
if old not in text:
    raise SystemExit("request authority block missing")
text = text.replace(old, new, 1)
start = text.index("fn request_authority(")
end = text.index("fn state_path()", start)
text = text[:start] + text[end:]
main.write_text(text)

nix = Path("modules/phenix-acp.nix")
text = nix.read_text()
text = text.replace(
    '''            jq -e '\n              .id == 1\n              and .status == "ok"''',
    '''            cat "$TMPDIR/create.json"\n            jq -e '\n              .id == 1\n              and .status == "ok"''',
    1,
)
text = text.replace(
    '''            jq -e '\n              .id == 2\n              and .status == "ok"''',
    '''            cat "$TMPDIR/restore.json"\n            jq -e '\n              .id == 2\n              and .status == "ok"''',
    1,
)
nix.write_text(text)
