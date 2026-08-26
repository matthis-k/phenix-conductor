use crate::{Authority, PluginHost, PluginId, PluginInstance, PluginManifest, ServiceId};
use serde::{Deserialize, Serialize};
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

pub const EXTERNAL_PROTOCOL_VERSION: u32 = 1;
const DEFAULT_MAX_FRAME_BYTES: usize = 1024 * 1024;
static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);

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
    StaleGeneration { expected: u64, actual: u64 },
    WrongRequest { expected: u64, actual: u64 },
    PluginMismatch { expected: PluginId, actual: String },
    UndeclaredService(ServiceId),
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
        services: Vec<String>,
    },
    Invoke {
        request_id: u64,
        generation: u64,
        service: String,
        input: Vec<u8>,
        authority: Vec<String>,
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
        services: Vec<String>,
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
                .map(|service| service.service.as_str().to_owned())
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
                for service in services {
                    let service = ServiceId::parse(service)
                        .map_err(|message| ExternalTransportError::Protocol(message.into()))?;
                    if !self
                        .manifest
                        .services
                        .iter()
                        .any(|declared| declared.service == service)
                    {
                        return Err(ExternalTransportError::UndeclaredService(service));
                    }
                }
                Ok(())
            }
            _ => Err(ExternalTransportError::Protocol(
                "expected handshake response".into(),
            )),
        }
    }

    fn invoke_service(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        authority: &Authority,
    ) -> Result<Vec<u8>, ExternalTransportError> {
        self.next_request += 1;
        let request_id = self.next_request;
        self.send(&HostFrame::Invoke {
            request_id,
            generation: self.generation,
            service: service.as_str().to_owned(),
            input: input.to_vec(),
            authority: authority
                .capabilities()
                .map(|capability| capability.as_str().to_owned())
                .collect(),
        })?;
        match self.receive()? {
            PluginFrame::Result {
                request_id: actual_request,
                generation,
                output,
            } => {
                self.require_generation(generation)?;
                if actual_request != request_id {
                    return Err(ExternalTransportError::WrongRequest {
                        expected: request_id,
                        actual: actual_request,
                    });
                }
                Ok(output)
            }
            PluginFrame::Error {
                request_id: actual_request,
                generation,
                message,
            } => {
                self.require_generation(generation)?;
                if actual_request != request_id {
                    return Err(ExternalTransportError::WrongRequest {
                        expected: request_id,
                        actual: actual_request,
                    });
                }
                Err(ExternalTransportError::Protocol(message))
            }
            PluginFrame::HandshakeOk { .. } => Err(ExternalTransportError::Protocol(
                "unexpected handshake response".into(),
            )),
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
        input
            .write_all(&encoded)
            .map_err(ExternalTransportError::Io)?;
        input.write_all(b"\n").map_err(ExternalTransportError::Io)?;
        input.flush().map_err(ExternalTransportError::Io)
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
        authority: &Authority,
    ) -> Result<Vec<u8>, String> {
        self.invoke_service(service, input, authority)
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
            input: Vec::new(),
            authority: Authority::new([
                CapabilityId::parse("fs.read").unwrap(),
                CapabilityId::parse("network.read").unwrap(),
            ])
            .capabilities()
            .map(|capability| capability.as_str().to_owned())
            .collect(),
        };
        let encoded = serde_json::to_value(frame).unwrap();
        assert_eq!(
            encoded["authority"],
            serde_json::json!(["fs.read", "network.read"])
        );
    }

    #[test]
    fn compatible_plugin_handshake_and_invoke_succeed() {
        let script = format!(
            r#"
            read handshake
            {READ_GENERATION}
            echo "{{\"type\":\"handshake_ok\",\"protocol\":1,\"plugin\":\"external\",\"generation\":$generation,\"services\":[\"echo@1\"]}}"
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
            echo "{{\"type\":\"handshake_ok\",\"protocol\":1,\"plugin\":\"external\",\"generation\":$generation,\"services\":[\"admin@1\"]}}"
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
            echo "{{\"type\":\"handshake_ok\",\"protocol\":1,\"plugin\":\"external\",\"generation\":$generation,\"services\":[\"echo@1\"]}}"
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
            echo "{{\"type\":\"handshake_ok\",\"protocol\":1,\"plugin\":\"external\",\"generation\":$generation,\"services\":[\"echo@1\"]}}"
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
                &ServiceId::parse("echo@1").unwrap(),
                b"input",
                &Authority::default(),
            ),
            Err(ExternalTransportError::Timeout)
        ));
        assert!(plugin.child.is_none());
    }
}
