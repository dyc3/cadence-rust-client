//! Build script for crabdance_proto crate
//!
//! This build script handles gRPC/Protobuf code generation using tonic-build.
//! It compiles all .proto files in the proto/ directory when the `codegen` feature is enabled.

use std::env;
use std::path::PathBuf;

fn main() {
    let crate_root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let proto_dir = crate_root.join("proto/uber/cadence/api/v1");

    // Find all .proto files
    let mut proto_files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&proto_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "proto").unwrap_or(false) {
                // Tell cargo to rerun if proto files change
                println!("cargo:rerun-if-changed={}", path.display());
                proto_files.push(path);
            }
        }
    }

    if proto_files.is_empty() {
        panic!("No .proto files found in {}", proto_dir.display());
    }

    proto_files.sort_unstable_by(|a, b| a.to_string_lossy().cmp(&b.to_string_lossy()));

    // Convert PathBuf to string references for tonic_build
    let proto_paths: Vec<&str> = proto_files
        .iter()
        .map(|p| p.to_str().expect("Invalid path"))
        .collect();

    // The include path should be the proto root so imports like
    // "uber/cadence/api/v1/common.proto" resolve correctly
    let proto_include = crate_root.join("proto");

    // Generate gRPC code
    tonic_build::configure()
        .build_client(true)
        .build_server(false)
        .out_dir("src/generated/")
        .compile_protos(
            &proto_paths,
            &[proto_include.to_str().expect("Invalid path")],
        )
        .expect("Failed to compile protobuf files");
}
