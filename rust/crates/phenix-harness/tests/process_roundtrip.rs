use serde_json::Value;
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

fn run_harness(state: &Path, requests: &[Value]) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_phenix-harness"))
        .env("PHENIX_STATE_DB", state)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("supported Harness binary must start");

    {
        let mut stdin = child.stdin.take().expect("Harness stdin must be piped");
        for request in requests {
            serde_json::to_writer(&mut stdin, request).unwrap();
            stdin.write_all(b"\n").unwrap();
        }
    }

    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "Harness process failed: {output:?}"
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn process_roundtrip_routes_and_restores_plugin_owned_state() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let state = std::env::temp_dir().join(format!(
        "phenix-harness-process-roundtrip-{}-{nonce}.sqlite",
        std::process::id()
    ));
    let _ = fs::remove_file(&state);

    let first = run_harness(
        &state,
        &[
            serde_json::json!({
                "id": 1,
                "service": "phenix.sessions@1",
                "input": {"operation": "create", "id": "process-session"}
            }),
            serde_json::json!({
                "id": 2,
                "service": "phenix.context@1",
                "input": {
                    "operation": "register",
                    "resource_id": "process:context",
                    "kind": "external",
                    "source": "process-roundtrip",
                    "scope": "workspace",
                    "content": [112, 114, 111, 99, 101, 115, 115]
                }
            }),
            serde_json::json!({
                "id": 3,
                "service": "phenix.planning@1",
                "input": {
                    "operation": "create_objective",
                    "id": "process-objective",
                    "title": "Process parity",
                    "parent": null
                }
            }),
        ],
    );
    assert_eq!(first.len(), 3);
    assert_eq!(first[0]["status"], "ok");
    assert_eq!(first[0]["output"]["session"]["id"], "process-session");
    assert_eq!(first[1]["status"], "ok");
    assert_eq!(
        first[1]["output"]["resource"]["descriptor"]["resource_id"],
        "process:context"
    );
    assert_eq!(first[2]["status"], "ok");
    assert_eq!(first[2]["output"]["objective"]["id"], "process-objective");

    let second = run_harness(
        &state,
        &[
            serde_json::json!({
                "id": 4,
                "service": "phenix.sessions@1",
                "input": {"operation": "get", "id": "process-session"}
            }),
            serde_json::json!({
                "id": 5,
                "service": "phenix.context@1",
                "input": {"operation": "list"}
            }),
            serde_json::json!({
                "id": 6,
                "service": "phenix.planning@1",
                "input": {"operation": "get_objective", "id": "process-objective"}
            }),
        ],
    );
    assert_eq!(second.len(), 3);
    assert_eq!(second[0]["status"], "ok");
    assert_eq!(second[0]["output"]["session"]["id"], "process-session");
    assert_eq!(second[1]["status"], "ok");
    assert!(second[1]["output"]["descriptors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|descriptor| descriptor["resource_id"] == "process:context"));
    assert_eq!(second[2]["status"], "ok");
    assert_eq!(second[2]["output"]["objective"]["id"], "process-objective");

    let _ = fs::remove_file(&state);
}
