use std::{
    env, fs,
    io::Write,
    process::{Command, Stdio},
};

fn main() {
    assert_eq!(phenix_acp::WIRE_PROTOCOL_NAME, "acp");

    let harness = env::var_os("PHENIX_HARNESS").unwrap_or_else(|| "phenix-harness".into());
    let state = env::temp_dir().join(format!("phenix-acp-smoke-{}.sqlite", std::process::id()));
    let _ = fs::remove_file(&state);

    let mut child = Command::new(harness)
        .env("PHENIX_STATE_DB", &state)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("supported Harness product must be executable for ACP smoke");
    {
        let mut stdin = child.stdin.take().expect("Harness stdin must be piped");
        stdin
            .write_all(
                b"{\"id\":1,\"service\":\"phenix.sessions@1\",\"input\":{\"type\":\"variant\",\"value\":{\"tag\":\"Create\",\"value\":{\"type\":\"table\",\"value\":{\"id\":{\"type\":\"string\",\"value\":\"acp-smoke\"}}}}}}\n{\"id\":2,\"service\":\"phenix.sessions@1\",\"input\":{\"type\":\"variant\",\"value\":{\"tag\":\"Get\",\"value\":{\"type\":\"table\",\"value\":{\"id\":{\"type\":\"string\",\"value\":\"acp-smoke\"}}}}}}\n",
            )
            .expect("ACP smoke requests must be written");
    }
    let output = child
        .wait_with_output()
        .expect("supported Harness product must complete ACP smoke");
    let _ = fs::remove_file(&state);

    assert!(
        output.status.success(),
        "supported Harness product must execute ACP smoke journey: {output:?}"
    );
    let stdout = String::from_utf8(output.stdout).expect("Harness output must be UTF-8 JSON lines");
    let responses = stdout.lines().collect::<Vec<_>>();
    assert_eq!(responses.len(), 2, "ACP smoke must receive two responses");
    assert!(
        responses[0].contains(r#""status":"ok""#)
            && responses[0].contains(r#""id":1"#)
            && responses[0].contains(r#""tag":"Created""#)
            && responses[0].contains(r#""value":"acp-smoke""#),
        "session creation must cross the supported Harness process boundary: {}",
        responses[0]
    );
    assert!(
        responses[1].contains(r#""status":"ok""#)
            && responses[1].contains(r#""id":2"#)
            && responses[1].contains(r#""tag":"Session""#)
            && responses[1].contains(r#""value":"acp-smoke""#),
        "session lookup must cross the supported Harness process boundary: {}",
        responses[1]
    );

    println!("phenix ACP boundary: structural session journey through supported Harness product");
}
