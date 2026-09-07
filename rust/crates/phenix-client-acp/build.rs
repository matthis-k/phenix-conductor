use sha2::{Digest, Sha256};
use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let source = phenix_application_interface::generate::rust(
        &phenix_application_interface::application_descriptor(),
    )
    .expect("the fixed application descriptor is Rust-generatable");
    let fingerprint = format!("{:x}", Sha256::digest(source.as_bytes()));
    let source = format!("pub const DESCRIPTOR_SHA256: &str = {fingerprint:?};\\n{source}");
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo supplies OUT_DIR"))
        .join("application.rs");
    fs::write(output, source).expect("write generated application API");
}
