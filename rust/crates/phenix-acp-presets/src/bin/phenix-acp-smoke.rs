use std::{env, fs, process::Command};

fn main() {
    assert_eq!(phenix_acp::WIRE_PROTOCOL_NAME, "acp");

    let harness = env::var_os("PHENIX_HARNESS").unwrap_or_else(|| "phenix-harness".into());
    let state = env::temp_dir().join(format!("phenix-acp-smoke-{}.sqlite", std::process::id()));
    let _ = fs::remove_file(&state);

    let status = Command::new(harness)
        .arg("--list-services")
        .env("PHENIX_STATE_DB", &state)
        .status()
        .expect("supported Harness product must be executable for ACP smoke");
    let _ = fs::remove_file(&state);

    assert!(
        status.success(),
        "supported Harness product must boot for ACP smoke"
    );
    println!("phenix ACP boundary: wire interoperability through supported Harness product");
}
