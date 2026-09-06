use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let source = phenix_application_interface::generate::rust(
        &phenix_application_interface::application_descriptor(),
    )
    .expect("the fixed application descriptor is Rust-generatable");
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo supplies OUT_DIR"))
        .join("application.rs");
    fs::write(output, source).expect("write generated application API");
}
