use crate::{
    Authority, CapabilityId, ExternalPluginProcess, ExternalSandbox, ExternalTransportConfig,
    Kernel, KernelConfig, KernelError, LayerPolicy, PluginExecution, PluginHost, PluginId,
    PluginInstance, PluginManifest, ServiceContribution, ServiceId, ServiceRole,
};
use std::{
    io,
    process::{Child, Command, Stdio},
    sync::Arc,
    time::Duration,
};

struct ScriptSandbox {
    script: String,
}

impl ExternalSandbox for ScriptSandbox {
    fn spawn(&self, _executable: &str) -> io::Result<Child> {
        Command::new("sh")
            .arg("-c")
            .arg(&self.script)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
    }
}

struct Terminal;

impl PluginInstance for Terminal {
    fn start(&mut self, _host: &PluginHost<'_>) -> Result<(), String> {
        Ok(())
    }

    fn invoke(
        &mut self,
        _service: &ServiceId,
        input: &[u8],
        _host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        Ok(input.to_vec())
    }
}

struct AuthorityTerminal;

impl PluginInstance for AuthorityTerminal {
    fn start(&mut self, _host: &PluginHost<'_>) -> Result<(), String> {
        Ok(())
    }

    fn invoke(
        &mut self,
        _service: &ServiceId,
        _input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        let network = CapabilityId::parse("network.read").unwrap();
        Ok(if host.authority().permits(&network) {
            b"network".to_vec()
        } else {
            b"empty".to_vec()
        })
    }
}

fn service() -> ServiceId {
    ServiceId::parse("fixture.external-layer@1").unwrap()
}

fn layer_manifest() -> PluginManifest {
    PluginManifest {
        id: PluginId::parse("fixture.external-layer").unwrap(),
        version: 1,
        execution: PluginExecution::External {
            executable: "fixture".into(),
        },
        dependencies: Vec::new(),
        services: vec![ServiceContribution {
            role: ServiceRole::Layer,
            service: service(),
            priority: 100,
            required_authority: Authority::default(),
        }],
        resource_namespaces: Vec::new(),
        maximum_authority: Authority::new([CapabilityId::parse("network.read").unwrap()]),
    }
}

fn terminal_manifest() -> PluginManifest {
    PluginManifest {
        id: PluginId::parse("fixture.terminal").unwrap(),
        version: 1,
        execution: PluginExecution::Embedded,
        dependencies: Vec::new(),
        services: vec![ServiceContribution {
            role: ServiceRole::Terminal,
            service: service(),
            priority: 1,
            required_authority: Authority::default(),
        }],
        resource_namespaces: Vec::new(),
        maximum_authority: Authority::new([CapabilityId::parse("network.read").unwrap()]),
    }
}

fn kernel_with_script_and_terminal(
    script: String,
    terminal_factory: fn() -> Box<dyn PluginInstance>,
) -> Kernel {
    let layer = layer_manifest();
    let layer_id = layer.id.clone();
    let terminal = terminal_manifest();
    let terminal_id = terminal.id.clone();
    let config = KernelConfig::new([layer.clone(), terminal])
        .unwrap()
        .with_layer_policy(
            service(),
            vec![LayerPolicy {
                plugin: layer_id.clone(),
                priority: 100,
                required: true,
                enabled: true,
            }],
        )
        .unwrap();
    let transport =
        ExternalTransportConfig::new(Arc::new(ScriptSandbox { script }), Duration::from_secs(2));
    let mut kernel = Kernel::new(config);
    kernel
        .register_external_factory(layer_id, move |manifest| {
            Ok(Box::new(ExternalPluginProcess::new(
                manifest.clone(),
                "fixture",
                transport.clone(),
            )))
        })
        .unwrap();
    kernel
        .register_embedded_factory(terminal_id, terminal_factory)
        .unwrap();
    kernel.activate_all().unwrap();
    kernel
}

fn kernel_with_script(script: String) -> Kernel {
    kernel_with_script_and_terminal(script, || Box::new(Terminal))
}

const READ_GENERATION: &str = r#"
    generation=${handshake#*\"generation\":}
    generation=${generation%%,*}
"#;

fn protocol_rejection(script_body: &str) -> KernelError {
    let script = format!(
        r#"
        read handshake
        {READ_GENERATION}
        echo "{{\"type\":\"handshake_ok\",\"protocol\":3,\"plugin\":\"fixture.external-layer\",\"generation\":$generation,\"services\":[{{\"service\":\"fixture.external-layer@1\",\"role\":\"layer\"}}]}}"
        read request
        continuation=$(printf '%s' "$request" | sed -n 's/.*\"continuation\":\([0-9][0-9]*\).*/\1/p')
        {script_body}
    "#
    );
    kernel_with_script(script)
        .invoke(&service(), b"input", &Authority::default(), None)
        .unwrap_err()
}

#[test]
fn external_layer_rejects_stale_generation_before_continuing() {
    let error = protocol_rejection(
        r#"echo "{\"type\":\"continue\",\"request_id\":1,\"generation\":0,\"continuation\":$continuation,\"input\":[],\"authority\":[]}""#,
    );
    assert!(matches!(error, KernelError::ServiceInvoke { .. }));
    assert!(error.to_string().contains("generation"));
}

#[test]
fn external_layer_rejects_cross_request_continuation_use() {
    let error = protocol_rejection(
        r#"echo "{\"type\":\"continue\",\"request_id\":2,\"generation\":$generation,\"continuation\":$continuation,\"input\":[],\"authority\":[]}""#,
    );
    assert!(matches!(error, KernelError::ServiceInvoke { .. }));
    assert!(error.to_string().contains("request"));
}

#[test]
fn external_layer_rejects_forged_continuation_token() {
    let error = protocol_rejection(
        r#"forged=0; if [ "$continuation" = "0" ]; then forged=1; fi; echo "{\"type\":\"continue\",\"request_id\":1,\"generation\":$generation,\"continuation\":$forged,\"input\":[],\"authority\":[]}""#,
    );
    assert!(matches!(error, KernelError::ServiceInvoke { .. }));
    assert!(error.to_string().contains("continuation token"));
}

#[test]
fn external_layer_rejects_double_continuation_use() {
    let error = protocol_rejection(
        r#"
        echo "{\"type\":\"continue\",\"request_id\":1,\"generation\":$generation,\"continuation\":$continuation,\"input\":[],\"authority\":[]}"
        read continued
        echo "{\"type\":\"continue\",\"request_id\":1,\"generation\":$generation,\"continuation\":$continuation,\"input\":[],\"authority\":[]}"
        "#,
    );
    assert!(matches!(error, KernelError::ServiceInvoke { .. }));
    assert!(error.to_string().contains("already consumed"));
}

#[test]
fn external_layer_continuation_cannot_expand_caller_authority() {
    let script = format!(
        r#"
        read handshake
        {READ_GENERATION}
        echo "{{\"type\":\"handshake_ok\",\"protocol\":3,\"plugin\":\"fixture.external-layer\",\"generation\":$generation,\"services\":[{{\"service\":\"fixture.external-layer@1\",\"role\":\"layer\"}}]}}"
        read request
        continuation=$(printf '%s' "$request" | sed -n 's/.*\"continuation\":\([0-9][0-9]*\).*/\1/p')
        echo "{{\"type\":\"continue\",\"request_id\":1,\"generation\":$generation,\"continuation\":$continuation,\"input\":[],\"authority\":[\"network.read\"]}}"
        read continued
        output=$(printf '%s' "$continued" | sed -n 's/.*\"output\":\(\[[^]]*\]\).*/\1/p')
        echo "{{\"type\":\"result\",\"request_id\":1,\"generation\":$generation,\"output\":$output}}"
    "#
    );
    let output = kernel_with_script_and_terminal(script, || Box::new(AuthorityTerminal))
        .invoke(&service(), b"input", &Authority::default(), None)
        .unwrap();

    assert_eq!(output, b"empty");
}
