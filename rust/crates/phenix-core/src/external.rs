use crate::{
    runtime::ContinuationBinding, Authority, CapabilityId, ComponentId, LayerResult, PluginHost,
    PluginId, PluginInstance, PluginManifest, ServiceId, ServiceRole,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    io::{self, BufRead, BufReader, BufWriter, Write},
    process::{Child, ChildStdin},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver},
        Arc,
    },
    thread,
    time::Duration,
};

pub const EXTERNAL_PROTOCOL_VERSION: u32 = 3;
const DEFAULT_MAX_FRAME_BYTES: usize = 1024 * 1024;
static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct ExternalService {
    service: String,
    role: ExternalServiceRole,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ExternalServiceRole {
    Terminal,
    Layer,
}

impl From<ServiceRole> for ExternalServiceRole {
    fn from(role: ServiceRole) -> Self {
        match role {
            ServiceRole::Terminal => Self::Terminal,
            ServiceRole::Layer => Self::Layer,
        }
    }
}

impl ExternalService {
    fn from_contribution(contribution: &crate::ServiceContribution) -> Self {
        Self {
            service: contribution.service.as_str().to_owned(),
            role: contribution.role.into(),
        }
    }
}

pub trait ExternalSandbox: Send + Sync {
    fn spawn(&self, executable: &str) -> io::Result<Child>;
}

#[derive(Clone)]
pub struct ExternalTransportConfig {
    pub sandbox: Arc<dyn ExternalSandbox>,
    pub request_timeout: Duration,
    pub max_frame_bytes: usize,
}

impl ExternalTransportConfig {
    pub fn new(sandbox: Arc<dyn ExternalSandbox>, request_timeout: Duration) -> Self {
        Self {
            sandbox,
            request_timeout,
            max_frame_bytes: DEFAULT_MAX_FRAME_BYTES,
        }
    }
}

#[derive(Debug)]
pub enum ExternalTransportError {
    Spawn(io::Error),
    Io(io::Error),
    Protocol(String),
    FrameTooLarge(usize),
    Timeout,
    Disconnected,
    StaleGeneration {
        expected: u64,
        actual: u64,
    },
    WrongRequest {
        expected: u64,
        actual: u64,
    },
    PluginMismatch {
        expected: PluginId,
        actual: String,
    },
    UndeclaredService(ServiceId),
    ServiceRoleMismatch {
        service: ServiceId,
        expected: ServiceRole,
        actual: ServiceRole,
    },
}

impl Display for ExternalTransportError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spawn(error) => write!(f, "external plugin spawn failed: {error}"),
            Self::Io(error) => write!(f, "external plugin I/O failed: {error}"),
            Self::Protocol(message) => write!(f, "external plugin protocol error: {message}"),
            Self::FrameTooLarge(size) => write!(f, "external plugin frame is too large: {size}"),
            Self::Timeout => f.write_str("external plugin request timed out"),
            Self::Disconnected => f.write_str("external plugin disconnected"),
            Self::StaleGeneration { expected, actual } => write!(
                f,
                "external plugin response generation {actual} does not match {expected}"
            ),
            Self::WrongRequest { expected, actual } => write!(
                f,
                "external plugin response request {actual} does not match {expected}"
            ),
            Self::PluginMismatch { expected, actual } => write!(
                f,
                "external plugin handshake identity {actual} does not match {expected}"
            ),
            Self::UndeclaredService(service) => {
                write!(f, "external plugin advertised undeclared service {service}")
            }
            Self::ServiceRoleMismatch {
                service,
                expected,
                actual,
            } => write!(
                f,
                "external plugin advertised role {actual:?} for service {service}, expected {expected:?}"
            ),
        }
    }
}

impl Error for ExternalTransportError {}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum HostFrame {
    Handshake {
        protocol: u32,
        plugin: String,
        generation: u64,
        services: Vec<ExternalService>,
    },
    Invoke {
        request_id: u64,
        generation: u64,
        service: String,
        component: Option<String>,
        input: Vec<u8>,
        authority: Vec<String>,
        continuation: Option<u64>,
    },
    ContinuationResult {
        request_id: u64,
        generation: u64,
        continuation: u64,
        output: Vec<u8>,
    },
    Stop {
        generation: u64,
    },
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum PluginFrame {
    HandshakeOk {
        protocol: u32,
        plugin: String,
        generation: u64,
        services: Vec<ExternalService>,
    },
    Result {
        request_id: u64,
        generation: u64,
        output: Vec<u8>,
    },
    Error {
        request_id: u64,
        generation: u64,
        message: String,
    },
    Denied {
        request_id: u64,
        generation: u64,
        message: String,
    },
    Continue {
        request_id: u64,
        generation: u64,
        continuation: u64,
        input: Vec<u8>,
        authority: Vec<String>,
    },
}

pub struct ExternalPluginProcess {
    manifest: PluginManifest,
    executable: String,
    config: ExternalTransportConfig,
    generation: u64,
    next_request: u64,
    child: Option<Child>,
    input: Option<BufWriter<ChildStdin>>,
    output: Option<Receiver<Result<String, ExternalTransportError>>>,
}

fn continuation_token(generation: u64, request_id: u64, binding: &ContinuationBinding) -> u64 {
    let mut digest = Sha256::new();
    digest.update(b"phenix.external.continuation.v1\0");
    digest.update(generation.to_be_bytes());
    digest.update(request_id.to_be_bytes());
    digest.update(binding.policy_identity.get().to_be_bytes());
    digest.update((binding.next_position as u64).to_be_bytes());
    if let Some(graph_generation) = &binding.graph_generation {
        digest.update((graph_generation.as_str().len() as u64).to_be_bytes());
        digest.update(graph_generation.as_str().as_bytes());
    } else {
        digest.update(0_u64.to_be_bytes());
    }
    digest.update((binding.service.as_str().len() as u64).to_be_bytes());
    digest.update(binding.service.as_str().as_bytes());
    for capability in binding.authority.capabilities() {
        digest.update((capability.as_str().len() as u64).to_be_bytes());
        digest.update(capability.as_str().as_bytes());
    }
    let digest = digest.finalize();
    u64::from_be_bytes(
        digest[..8]
            .try_into()
            .expect("sha256 prefix is eight bytes"),
    )
}

impl ExternalPluginProcess {
    pub fn new(
        manifest: PluginManifest,
        executable: impl Into<String>,
        config: ExternalTransportConfig,
    ) -> Self {
        Self {
            manifest,
            executable: executable.into(),
            config,
            generation: 0,
            next_request: 0,
            child: None,
            input: None,
            output: None,
        }
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    fn start_process(&mut self) -> Result<(), ExternalTransportError> {
        self.terminate();
        let mut child = self
            .config
            .sandbox
            .spawn(&self.executable)
            .map_err(ExternalTransportError::Spawn)?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| ExternalTransportError::Protocol("sandbox did not pipe stdin".into()))?;
        let stdout = child.stdout.take().ok_or_else(|| {
            ExternalTransportError::Protocol("sandbox did not pipe stdout".into())
        })?;
        let (sender, receiver) = mpsc::channel();
        let max_frame_bytes = self.config.max_frame_bytes;
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) if line.len() > max_frame_bytes => {
                        let _ = sender.send(Err(ExternalTransportError::FrameTooLarge(line.len())));
                        break;
                    }
                    Ok(_) => {
                        let _ = sender.send(Ok(line));
                    }
                    Err(error) => {
                        let _ = sender.send(Err(ExternalTransportError::Io(error)));
                        break;
                    }
                }
            }
        });

        self.generation = NEXT_GENERATION.fetch_add(1, Ordering::Relaxed);
        self.next_request = 0;
        self.child = Some(child);
        self.input = Some(BufWriter::new(stdin));
        self.output = Some(receiver);
        Ok(())
    }

    fn handshake(&mut self) -> Result<(), ExternalTransportError> {
        let frame = HostFrame::Handshake {
            protocol: EXTERNAL_PROTOCOL_VERSION,
            plugin: self.manifest.id.as_str().to_owned(),
            generation: self.generation,
            services: self
                .manifest
                .services
                .iter()
                .map(ExternalService::from_contribution)
                .collect(),
        };
        self.send(&frame)?;
        let response = self.receive()?;
        match response {
            PluginFrame::HandshakeOk {
                protocol,
                plugin,
                generation,
                services,
            } => {
                if protocol != EXTERNAL_PROTOCOL_VERSION {
                    return Err(ExternalTransportError::Protocol(format!(
                        "unsupported protocol version {protocol}"
                    )));
                }
                if plugin != self.manifest.id.as_str() {
                    return Err(ExternalTransportError::PluginMismatch {
                        expected: self.manifest.id.clone(),
                        actual: plugin,
                    });
                }
                self.require_generation(generation)?;
                if services.len() != self.manifest.services.len() {
                    return Err(ExternalTransportError::Protocol(format!(
                        "external plugin advertised {} services, expected {}",
                        services.len(),
                        self.manifest.services.len()
                    )));
                }
                for advertised in services {
                    let service = ServiceId::parse(advertised.service)
                        .map_err(|message| ExternalTransportError::Protocol(message.into()))?;
                    let Some(declared) = self
                        .manifest
                        .services
                        .iter()
                        .find(|declared| declared.service == service)
                    else {
                        return Err(ExternalTransportError::UndeclaredService(service));
                    };
                    let expected = ExternalServiceRole::from(declared.role);
                    if advertised.role != expected {
                        return Err(ExternalTransportError::ServiceRoleMismatch {
                            service,
                            expected: declared.role,
                            actual: match advertised.role {
                                ExternalServiceRole::Terminal => ServiceRole::Terminal,
                                ExternalServiceRole::Layer => ServiceRole::Layer,
                            },
                        });
                    }
                }
                Ok(())
            }
            _ => Err(ExternalTransportError::Protocol(
                "expected handshake response".into(),
            )),
        }
    }

    fn next_request(&mut self) -> u64 {
        self.next_request += 1;
        self.next_request
    }

    fn validate_response(
        &self,
        expected_request: u64,
        actual_request: u64,
        generation: u64,
    ) -> Result<(), ExternalTransportError> {
        self.require_generation(generation)?;
        if actual_request == expected_request {
            Ok(())
        } else {
            Err(ExternalTransportError::WrongRequest {
                expected: expected_request,
                actual: actual_request,
            })
        }
    }

    fn decode_authority(values: Vec<String>) -> Result<Authority, ExternalTransportError> {
        values
            .into_iter()
            .map(|value| {
                CapabilityId::parse(value)
                    .map_err(|message| ExternalTransportError::Protocol(message.into()))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Authority::new)
    }

    fn invoke_service(
        &mut self,
        component: Option<&ComponentId>,
        service: &ServiceId,
        input: &[u8],
        authority: &Authority,
    ) -> Result<Vec<u8>, ExternalTransportError> {
        let request_id = self.next_request();
        self.send(&HostFrame::Invoke {
            request_id,
            generation: self.generation,
            service: service.as_str().to_owned(),
            component: component.map(|component| component.as_str().to_owned()),
            input: input.to_vec(),
            authority: authority
                .capabilities()
                .map(|capability| capability.as_str().to_owned())
                .collect(),
            continuation: None,
        })?;
        match self.receive()? {
            PluginFrame::Result {
                request_id: actual_request,
                generation,
                output,
            } => {
                self.validate_response(request_id, actual_request, generation)?;
                Ok(output)
            }
            PluginFrame::Error {
                request_id: actual_request,
                generation,
                message,
            } => {
                self.validate_response(request_id, actual_request, generation)?;
                Err(ExternalTransportError::Protocol(message))
            }
            PluginFrame::Denied { .. } => Err(ExternalTransportError::Protocol(
                "terminal provider cannot deny as a layer".into(),
            )),
            PluginFrame::Continue { .. } => Err(ExternalTransportError::Protocol(
                "terminal provider requested an unavailable continuation".into(),
            )),
            PluginFrame::HandshakeOk { .. } => Err(ExternalTransportError::Protocol(
                "unexpected handshake response".into(),
            )),
        }
    }

    fn invoke_layer_service(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<LayerResult, ExternalTransportError> {
        let request_id = self.next_request();
        let binding = host
            .continuation_binding()
            .map_err(|error| ExternalTransportError::Protocol(error.to_string()))?;
        let continuation = continuation_token(self.generation, request_id, &binding);
        self.send(&HostFrame::Invoke {
            request_id,
            generation: self.generation,
            service: service.as_str().to_owned(),
            component: None,
            input: input.to_vec(),
            authority: host
                .authority()
                .capabilities()
                .map(|capability| capability.as_str().to_owned())
                .collect(),
            continuation: Some(continuation),
        })?;

        loop {
            match self.receive()? {
                PluginFrame::Result {
                    request_id: actual_request,
                    generation,
                    output,
                } => {
                    self.validate_response(request_id, actual_request, generation)?;
                    return Ok(LayerResult::Handled(output));
                }
                PluginFrame::Denied {
                    request_id: actual_request,
                    generation,
                    message,
                } => {
                    self.validate_response(request_id, actual_request, generation)?;
                    return Ok(LayerResult::Denied(message));
                }
                PluginFrame::Error {
                    request_id: actual_request,
                    generation,
                    message,
                } => {
                    self.validate_response(request_id, actual_request, generation)?;
                    return Err(ExternalTransportError::Protocol(message));
                }
                PluginFrame::Continue {
                    request_id: actual_request,
                    generation,
                    continuation: actual_continuation,
                    input,
                    authority,
                } => {
                    self.validate_response(request_id, actual_request, generation)?;
                    if actual_continuation != continuation {
                        return Err(ExternalTransportError::Protocol(
                            "external plugin used an invalid continuation token".into(),
                        ));
                    }
                    let requested_authority = Self::decode_authority(authority)?;
                    let output = host
                        .continue_service(&input, &requested_authority)
                        .map_err(|error| ExternalTransportError::Protocol(error.to_string()))?;
                    self.send(&HostFrame::ContinuationResult {
                        request_id,
                        generation: self.generation,
                        continuation,
                        output,
                    })?;
                }
                PluginFrame::HandshakeOk { .. } => {
                    return Err(ExternalTransportError::Protocol(
                        "unexpected handshake response".into(),
                    ));
                }
            }
        }
    }

    fn send(&mut self, frame: &HostFrame) -> Result<(), ExternalTransportError> {
        let encoded = serde_json::to_vec(frame)
            .map_err(|error| ExternalTransportError::Protocol(error.to_string()))?;
        if encoded.len() > self.config.max_frame_bytes {
            return Err(ExternalTransportError::FrameTooLarge(encoded.len()));
        }
        let input = self
            .input
            .as_mut()
            .ok_or(ExternalTransportError::Disconnected)?;
        let result = input
            .write_all(&encoded)
            .and_then(|()| input.write_all(b"\n"))
            .and_then(|()| input.flush());
        match result {
            Ok(()) => Ok(()),
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::BrokenPipe
                        | io::ErrorKind::ConnectionAborted
                        | io::ErrorKind::ConnectionReset
                        | io::ErrorKind::NotConnected
                ) =>
            {
                self.terminate();
                Err(ExternalTransportError::Disconnected)
            }
            Err(error) => Err(ExternalTransportError::Io(error)),
        }
    }

    fn receive(&mut self) -> Result<PluginFrame, ExternalTransportError> {
        let result = {
            let receiver = self
                .output
                .as_ref()
                .ok_or(ExternalTransportError::Disconnected)?;
            receiver.recv_timeout(self.config.request_timeout)
        };
        let line = match result {
            Ok(result) => result?,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                self.terminate();
                return Err(ExternalTransportError::Timeout);
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                self.terminate();
                return Err(ExternalTransportError::Disconnected);
            }
        };
        serde_json::from_str(&line)
            .map_err(|error| ExternalTransportError::Protocol(error.to_string()))
    }

    fn require_generation(&self, actual: u64) -> Result<(), ExternalTransportError> {
        if actual == self.generation {
            Ok(())
        } else {
            Err(ExternalTransportError::StaleGeneration {
                expected: self.generation,
                actual,
            })
        }
    }

    fn terminate(&mut self) {
        self.input.take();
        self.output.take();
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for ExternalPluginProcess {
    fn drop(&mut self) {
        self.terminate();
    }
}

impl PluginInstance for ExternalPluginProcess {
    fn start(&mut self, _host: &PluginHost<'_>) -> Result<(), String> {
        self.start_process()
            .and_then(|()| self.handshake())
            .map_err(|error| {
                self.terminate();
                error.to_string()
            })
    }

    fn invoke(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        self.invoke_service(None, service, input, host.authority())
            .map_err(|error| error.to_string())
    }

    fn invoke_component(
        &mut self,
        component: &ComponentId,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        self.invoke_service(Some(component), service, input, host.authority())
            .map_err(|error| error.to_string())
    }

    fn invoke_layer(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<LayerResult, String> {
        self.invoke_layer_service(service, input, host)
            .map_err(|error| error.to_string())
    }

    fn stop(&mut self) -> Result<(), String> {
        if self.child.is_some() {
            let _ = self.send(&HostFrame::Stop {
                generation: self.generation,
            });
        }
        self.terminate();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CapabilityId, PluginExecution, ServiceContribution};
    use crate::{Kernel, KernelConfig, KernelError, LayerPolicy};
    use std::process::{Command, Stdio};

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

    fn manifest() -> PluginManifest {
        PluginManifest {
            id: PluginId::parse("external").unwrap(),
            version: 1,
            execution: PluginExecution::External {
                executable: "fixture".into(),
            },
            dependencies: Vec::new(),
            services: vec![ServiceContribution {
                role: ServiceRole::Terminal,
                service: ServiceId::parse("echo@1").unwrap(),
                priority: 1,
                required_authority: Authority::default(),
            }],
            resource_namespaces: Vec::new(),
            maximum_authority: Authority::default(),
        }
    }

    fn config(script: &str, timeout: Duration) -> ExternalTransportConfig {
        ExternalTransportConfig::new(
            Arc::new(ScriptSandbox {
                script: script.into(),
            }),
            timeout,
        )
    }

    const READ_GENERATION: &str = r#"
        generation=${handshake#*\"generation\":}
        generation=${generation%%,*}
    "#;

    #[test]
    fn invoke_frame_carries_only_effective_authority() {
        let frame = HostFrame::Invoke {
            request_id: 1,
            generation: 7,
            service: "echo@1".into(),
            component: None,
            input: Vec::new(),
            authority: Authority::new([
                CapabilityId::parse("fs.read").unwrap(),
                CapabilityId::parse("network.read").unwrap(),
            ])
            .capabilities()
            .map(|capability| capability.as_str().to_owned())
            .collect(),
            continuation: None,
        };
        let encoded = serde_json::to_value(frame).unwrap();
        assert_eq!(
            encoded["authority"],
            serde_json::json!(["fs.read", "network.read"])
        );
    }

    #[test]
    fn component_invoke_frame_carries_exact_component_identity() {
        let frame = HostFrame::Invoke {
            request_id: 1,
            generation: 7,
            service: "echo@1".into(),
            component: Some("provider.component".into()),
            input: Vec::new(),
            authority: Vec::new(),
            continuation: None,
        };
        let encoded = serde_json::to_value(frame).unwrap();
        assert_eq!(encoded["component"], "provider.component");
    }

    #[test]
    fn continuation_token_binds_invocation_service_policy_authority_and_position() {
        let read = CapabilityId::parse("workspace.read").unwrap();
        let baseline_policy = KernelConfig::empty().policy_identity();
        let other_policy = KernelConfig::empty().policy_identity();
        let baseline = ContinuationBinding {
            graph_generation: None,
            policy_identity: baseline_policy,
            service: ServiceId::parse("echo@1").unwrap(),
            authority: Authority::new([read.clone()]),
            next_position: 1,
        };
        let token = continuation_token(7, 11, &baseline);

        let mut changed = baseline.clone();
        changed.service = ServiceId::parse("other@1").unwrap();
        assert_ne!(token, continuation_token(7, 11, &changed));

        let mut changed = baseline.clone();
        changed.policy_identity = other_policy;
        assert_ne!(token, continuation_token(7, 11, &changed));

        let mut changed = baseline.clone();
        changed.authority = Authority::default();
        assert_ne!(token, continuation_token(7, 11, &changed));

        let mut changed = baseline.clone();
        changed.next_position = 2;
        assert_ne!(token, continuation_token(7, 11, &changed));

        assert_ne!(token, continuation_token(8, 11, &baseline));
        assert_ne!(token, continuation_token(7, 12, &baseline));
    }

    #[test]
    fn compatible_plugin_handshake_and_invoke_succeed() {
        let script = format!(
            r#"
            read handshake
            {READ_GENERATION}
            echo "{{\"type\":\"handshake_ok\",\"protocol\":3,\"plugin\":\"external\",\"generation\":$generation,\"services\":[{{\"service\":\"echo@1\",\"role\":\"terminal\"}}]}}"
            read request
            echo "{{\"type\":\"result\",\"request_id\":1,\"generation\":$generation,\"output\":[111,107]}}"
            read stop || true
        "#
        );
        let mut plugin = ExternalPluginProcess::new(
            manifest(),
            "fixture",
            config(&script, Duration::from_secs(2)),
        );
        plugin.start_process().unwrap();
        plugin.handshake().unwrap();
        assert_eq!(
            plugin
                .invoke_service(
                    None,
                    &ServiceId::parse("echo@1").unwrap(),
                    b"input",
                    &Authority::default(),
                )
                .unwrap(),
            b"ok"
        );
    }

    #[test]
    fn handshake_cannot_add_undeclared_service() {
        let script = format!(
            r#"
            read handshake
            {READ_GENERATION}
            echo "{{\"type\":\"handshake_ok\",\"protocol\":3,\"plugin\":\"external\",\"generation\":$generation,\"services\":[{{\"service\":\"admin@1\",\"role\":\"terminal\"}}]}}"
        "#
        );
        let mut plugin = ExternalPluginProcess::new(
            manifest(),
            "fixture",
            config(&script, Duration::from_secs(2)),
        );
        plugin.start_process().unwrap();
        assert!(matches!(
            plugin.handshake(),
            Err(ExternalTransportError::UndeclaredService(_))
        ));
    }

    #[test]
    fn crash_becomes_disconnect_instead_of_kernel_crash() {
        let script = format!(
            r#"
            read handshake
            {READ_GENERATION}
            echo "{{\"type\":\"handshake_ok\",\"protocol\":3,\"plugin\":\"external\",\"generation\":$generation,\"services\":[{{\"service\":\"echo@1\",\"role\":\"terminal\"}}]}}"
            exit 0
        "#
        );
        let mut plugin = ExternalPluginProcess::new(
            manifest(),
            "fixture",
            config(&script, Duration::from_secs(2)),
        );
        plugin.start_process().unwrap();
        plugin.handshake().unwrap();
        assert!(matches!(
            plugin.invoke_service(
                None,
                &ServiceId::parse("echo@1").unwrap(),
                b"input",
                &Authority::default(),
            ),
            Err(ExternalTransportError::Disconnected)
        ));
    }

    #[test]
    fn request_timeout_terminates_generation() {
        let script = format!(
            r#"
            read handshake
            {READ_GENERATION}
            echo "{{\"type\":\"handshake_ok\",\"protocol\":3,\"plugin\":\"external\",\"generation\":$generation,\"services\":[{{\"service\":\"echo@1\",\"role\":\"terminal\"}}]}}"
            read request
            sleep 5
        "#
        );
        let mut plugin = ExternalPluginProcess::new(
            manifest(),
            "fixture",
            config(&script, Duration::from_millis(50)),
        );
        plugin.start_process().unwrap();
        plugin.handshake().unwrap();
        assert!(matches!(
            plugin.invoke_service(
                None,
                &ServiceId::parse("echo@1").unwrap(),
                b"input",
                &Authority::default(),
            ),
            Err(ExternalTransportError::Timeout)
        ));
        assert!(plugin.child.is_none());
    }

    fn layer_manifest() -> PluginManifest {
        let mut value = manifest();
        value.id = PluginId::parse("external-layer").unwrap();
        value.services[0].role = ServiceRole::Layer;
        value
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
            let mut output = b"terminal:".to_vec();
            output.extend_from_slice(input);
            Ok(output)
        }
    }

    #[test]
    fn handshake_rejects_service_role_mismatch() {
        let script = format!(
            r#"
            read handshake
            {READ_GENERATION}
            echo "{{\"type\":\"handshake_ok\",\"protocol\":3,\"plugin\":\"external\",\"generation\":$generation,\"services\":[{{\"service\":\"echo@1\",\"role\":\"layer\"}}]}}"
        "#
        );
        let mut plugin = ExternalPluginProcess::new(
            manifest(),
            "fixture",
            config(&script, Duration::from_secs(2)),
        );
        plugin.start_process().unwrap();
        assert!(matches!(
            plugin.handshake(),
            Err(ExternalTransportError::ServiceRoleMismatch { .. })
        ));
    }

    #[test]
    fn external_layer_delegates_through_opaque_continuation() {
        let script = format!(
            r#"
            read handshake
            {READ_GENERATION}
            echo "{{\"type\":\"handshake_ok\",\"protocol\":3,\"plugin\":\"external-layer\",\"generation\":$generation,\"services\":[{{\"service\":\"echo@1\",\"role\":\"layer\"}}]}}"
            read request
            continuation=$(printf '%s' "$request" | sed -n 's/.*\"continuation\":\([0-9][0-9]*\).*/\1/p')
            echo "{{\"type\":\"continue\",\"request_id\":1,\"generation\":$generation,\"continuation\":$continuation,\"input\":[100,101,108,101,103,97,116,101,100],\"authority\":[]}}"
            read continued
            echo "{{\"type\":\"result\",\"request_id\":1,\"generation\":$generation,\"output\":[108,97,121,101,114,45,111,107]}}"
            read stop || true
        "#
        );
        let layer = layer_manifest();
        let layer_id = layer.id.clone();
        let mut terminal_manifest = manifest();
        terminal_manifest.execution = crate::PluginExecution::Embedded;
        let terminal_id = terminal_manifest.id.clone();
        let kernel_config = KernelConfig::new([layer.clone(), terminal_manifest])
            .unwrap()
            .with_layer_policy(
                ServiceId::parse("echo@1").unwrap(),
                vec![LayerPolicy {
                    plugin: layer_id.clone(),
                    priority: 100,
                    required: true,
                    enabled: true,
                }],
            )
            .unwrap();
        let transport = config(&script, Duration::from_secs(2));
        let mut kernel = Kernel::new(kernel_config);
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
            .register_embedded_factory(terminal_id, || Box::new(Terminal))
            .unwrap();
        kernel.activate_all().unwrap();
        assert_eq!(
            kernel
                .invoke(
                    &ServiceId::parse("echo@1").unwrap(),
                    b"input",
                    &Authority::default(),
                    None,
                )
                .unwrap(),
            b"layer-ok"
        );
    }

    #[test]
    fn external_layer_cannot_replay_continuation() {
        let script = format!(
            r#"
            read handshake
            {READ_GENERATION}
            echo "{{\"type\":\"handshake_ok\",\"protocol\":3,\"plugin\":\"external-layer\",\"generation\":$generation,\"services\":[{{\"service\":\"echo@1\",\"role\":\"layer\"}}]}}"
            read request
            continuation=$(printf '%s' "$request" | sed -n 's/.*\"continuation\":\([0-9][0-9]*\).*/\1/p')
            echo "{{\"type\":\"continue\",\"request_id\":1,\"generation\":$generation,\"continuation\":$continuation,\"input\":[],\"authority\":[]}}"
            read continued
            echo "{{\"type\":\"continue\",\"request_id\":1,\"generation\":$generation,\"continuation\":$continuation,\"input\":[],\"authority\":[]}}"
        "#
        );
        let layer = layer_manifest();
        let layer_id = layer.id.clone();
        let mut terminal_manifest = manifest();
        terminal_manifest.execution = crate::PluginExecution::Embedded;
        let terminal_id = terminal_manifest.id.clone();
        let kernel_config = KernelConfig::new([layer.clone(), terminal_manifest])
            .unwrap()
            .with_layer_policy(
                ServiceId::parse("echo@1").unwrap(),
                vec![LayerPolicy {
                    plugin: layer_id.clone(),
                    priority: 100,
                    required: true,
                    enabled: true,
                }],
            )
            .unwrap();
        let transport = config(&script, Duration::from_secs(2));
        let mut kernel = Kernel::new(kernel_config);
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
            .register_embedded_factory(terminal_id, || Box::new(Terminal))
            .unwrap();
        kernel.activate_all().unwrap();
        assert!(matches!(
            kernel.invoke(
                &ServiceId::parse("echo@1").unwrap(),
                b"input",
                &Authority::default(),
                None,
            ),
            Err(KernelError::ServiceInvoke { .. })
        ));
    }
}
