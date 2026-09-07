use sha2::{Digest, Sha256};
use std::{env, fs, path::PathBuf};

const EXTENSION_CLIENT: &str = r#"
impl super::AcpConnection {
    /// Returns the descriptor-backed Phenix extension methods negotiated at ACP initialize.
    pub fn negotiated_extensions(&self) -> Vec<super::ExtensionMethod> {
        self.extensions.methods.values().cloned().collect()
    }

    /// Invokes one negotiated descriptor-backed Phenix extension.
    pub async fn invoke_extension(
        &self,
        operation: &phenix_core::ContractId,
        input: phenix_core::PhenixValue,
    ) -> Result<phenix_core::PhenixValue, super::ClientError> {
        let method = self.extensions.require(operation)?;
        method.input.parse(&input).map_err(|error| {
            super::ClientError::Protocol(format!(
                "ACP extension input violates the application descriptor: {error}"
            ))
        })?;
        let raw = serde_json::value::to_raw_value(&input).map_err(|error| {
            super::ClientError::Protocol(format!("cannot encode ACP extension input: {error}"))
        })?;
        let request = agent_client_protocol::schema::v1::ClientRequest::ExtMethodRequest(
            agent_client_protocol::schema::v1::ExtRequest::new(
                method.method.clone(),
                std::sync::Arc::from(raw),
            ),
        );
        let response = self
            .connection
            .send_request(request)
            .block_task()
            .await
            .map_err(|error| {
                super::ClientError::Protocol(format!("ACP extension request failed: {error}"))
            })?;
        let output = serde_json::from_value::<phenix_core::PhenixValue>(response).map_err(|error| {
            super::ClientError::Protocol(format!("cannot decode ACP extension output: {error}"))
        })?;
        method.output.parse(&output).map_err(|error| {
            super::ClientError::Protocol(format!(
                "ACP extension output violates the application descriptor: {error}"
            ))
        })?;
        Ok(output)
    }
}
"#;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let source = phenix_application_interface::generate::rust(
        &phenix_application_interface::application_descriptor(),
    )
    .expect("the fixed application descriptor is Rust-generatable");
    let fingerprint = format!("{:x}", Sha256::digest(source.as_bytes()));
    let source = format!(
        "pub const DESCRIPTOR_SHA256: &str = {fingerprint:?};\n{source}\n{EXTENSION_CLIENT}"
    );
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo supplies OUT_DIR"))
        .join("application.rs");
    fs::write(output, source).expect("write generated application API");
}
