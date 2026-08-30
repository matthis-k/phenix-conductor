#![forbid(unsafe_code)]

use phenix_client::{ServiceRequest, ServiceResponse};
use phenix_core::{
    Authority, GraphGenerationId, Kernel, KernelError, PluginManifest, ResolvedHarness,
    ResolvedHarnessActivation, ResolvedHarnessActivationError, ResolvedHarnessError, ServiceId,
};
use serde_json::Value;
use std::{
    fmt,
    io::{self, BufRead, Write},
};

/// Generic configured Phenix server runtime.
///
/// Product plugin selection belongs to the Harness. A conductor created with no
/// manifests therefore exposes no first-party services.
pub struct Conductor {
    kernel: Kernel,
    resolved: ResolvedHarness,
}

#[derive(Debug)]
pub enum ConductorBuildError {
    Resolution(ResolvedHarnessError),
    Activation(ResolvedHarnessActivationError),
}

impl fmt::Display for ConductorBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Resolution(error) => fmt::Display::fmt(error, formatter),
            Self::Activation(error) => {
                write!(formatter, "resolved conductor activation failed: {error:?}")
            }
        }
    }
}

impl std::error::Error for ConductorBuildError {}

impl From<ResolvedHarnessError> for ConductorBuildError {
    fn from(error: ResolvedHarnessError) -> Self {
        Self::Resolution(error)
    }
}

impl From<ResolvedHarnessActivationError> for ConductorBuildError {
    fn from(error: ResolvedHarnessActivationError) -> Self {
        Self::Activation(error)
    }
}

impl Conductor {
    pub fn new(
        manifests: impl IntoIterator<Item = PluginManifest>,
    ) -> Result<Self, ConductorBuildError> {
        let resolved = ResolvedHarness::resolve(manifests, [], [], &Authority::default())?;
        let mut kernel = Kernel::new(resolved.kernel_config().clone());
        kernel.activate_resolved_harness(&resolved)?;
        Ok(Self { kernel, resolved })
    }

    #[must_use]
    pub fn kernel(&self) -> &Kernel {
        &self.kernel
    }

    pub fn kernel_mut(&mut self) -> &mut Kernel {
        &mut self.kernel
    }

    #[must_use]
    pub fn resolved_harness(&self) -> &ResolvedHarness {
        &self.resolved
    }

    #[must_use]
    pub fn generation(&self) -> &GraphGenerationId {
        self.resolved.generation()
    }

    pub fn activate_all(&mut self) -> Result<Vec<PluginId>, KernelError> {
        self.kernel.activate_all()
    }

    pub fn invoke(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        authority: &Authority,
    ) -> Result<Vec<u8>, KernelError> {
        self.kernel.invoke(service, input, authority, None)
    }

    pub fn apply_activation(
        &mut self,
        activation: &ResolvedHarnessActivation,
    ) -> Result<(), ResolvedHarnessActivationError> {
        self.kernel.activate_resolved_harness(activation)?;
        self.resolved = activation.resolved.clone();
        Ok(())
    }

    pub fn serve_stdio(self) -> Result<(), String> {
        self.serve(io::stdin().lock(), io::stdout().lock())
    }

    pub fn serve<R: BufRead, W: Write>(
        mut self,
        reader: R,
        mut writer: W,
    ) -> Result<(), String> {
        for line in reader.lines() {
            let line = line.map_err(|error| format!("failed to read request: {error}"))?;
            if line.trim().is_empty() {
                continue;
            }
            let response = self.handle_request(&line);
            serde_json::to_writer(&mut writer, &response)
                .map_err(|error| format!("failed to serialize response: {error}"))?;
            writer
                .write_all(b"\n")
                .map_err(|error| format!("failed to write response: {error}"))?;
            writer
                .flush()
                .map_err(|error| format!("failed to flush response: {error}"))?;
        }
        Ok(())
    }

    fn handle_request(&mut self, line: &str) -> ServiceResponse {
        let request = match serde_json::from_str::<ServiceRequest>(line) {
            Ok(request) => request,
            Err(error) => {
                return ServiceResponse::error(Value::Null, format!("invalid request: {error}"));
            }
        };

        let request_id = request.id.clone();
        let service = match ServiceId::parse(request.service) {
            Ok(service) => service,
            Err(error) => return ServiceResponse::error(request_id, error.to_owned()),
        };
        let input = match serde_json::to_vec(&request.input) {
            Ok(input) => input,
            Err(error) => {
                return ServiceResponse::error(
                    request_id,
                    format!("failed to encode request input: {error}"),
                );
            }
        };

        match self.kernel.invoke(
            &service,
            &input,
            &request.authority.unwrap_or_default(),
            request.causality_id,
        ) {
            Ok(output) => match serde_json::from_slice(&output) {
                Ok(output) => ServiceResponse::success(request_id, output),
                Err(error) => ServiceResponse::error(
                    request_id,
                    format!("service returned invalid JSON: {error}"),
                ),
            },
            Err(error) => ServiceResponse::error(request_id, error.to_string()),
        }
    }
}
