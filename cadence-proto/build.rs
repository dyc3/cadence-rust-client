//! Build script for cadence-proto crate
//!
//! This build script handles Thrift code generation. It will:
//! 1. Check if the Apache Thrift compiler is available
//! 2. If available and the 'gen-thrift' feature is enabled, regenerate the code
//! 3. Otherwise, use the pre-generated code checked into the repository
//!
//! To regenerate the Thrift code:
//! 1. Install the Apache Thrift compiler (version 0.17.0)
//!    - macOS: brew install thrift
//!    - Ubuntu/Debian: apt-get install thrift-compiler
//!    - Or build from source: https://thrift.apache.org/download
//! 2. Run: cargo build --features gen-thrift

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Tell Cargo to rerun this script if thrift files change
    println!("cargo:rerun-if-changed=thrift/shared.thrift");
    println!("cargo:rerun-if-changed=thrift/cadence.thrift");
    println!("cargo:rerun-if-changed=src/generated/mod.rs");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let crate_root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    // Check if we should regenerate thrift code
    let should_regenerate = env::var("CARGO_FEATURE_GEN_THRIFT").is_ok();

    if should_regenerate {
        match generate_thrift_code(&crate_root, &out_dir) {
            Ok(_) => {
                println!("cargo:warning=Thrift code regenerated successfully");
            }
            Err(e) => {
                println!("cargo:warning=Failed to regenerate Thrift code: {}", e);
                println!("cargo:warning=Using pre-generated code instead");
                use_pre_generated_code(&crate_root);
            }
        }
    } else {
        // Use pre-generated code
        use_pre_generated_code(&crate_root);
    }
}

fn generate_thrift_code(
    crate_root: &PathBuf,
    _out_dir: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    // Check if thrift compiler is available
    let thrift_check = Command::new("thrift").arg("--version").output();

    match thrift_check {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            println!("cargo:warning=Found Thrift compiler: {}", version.trim());
        }
        _ => {
            return Err("Thrift compiler not found. Please install Apache Thrift 0.19.0".into());
        }
    }

    let thrift_dir = crate_root.join("thrift");
    let gen_dir = crate_root.join("src/generated");

    // Ensure the generated directory exists
    std::fs::create_dir_all(&gen_dir)?;

    // Generate Rust code from shared.thrift
    let shared_thrift = thrift_dir.join("shared.thrift");
    if shared_thrift.exists() {
        let output = Command::new("thrift")
            .arg("-r")
            .arg("--gen")
            .arg("rs")
            .arg("-out")
            .arg(&gen_dir)
            .arg(&shared_thrift)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to generate code for shared.thrift: {}", stderr).into());
        }
        println!("cargo:warning=Generated code for shared.thrift");
    }

    // Generate Rust code from cadence.thrift
    let cadence_thrift = thrift_dir.join("cadence.thrift");
    if cadence_thrift.exists() {
        let output = Command::new("thrift")
            .arg("-r")
            .arg("--gen")
            .arg("rs")
            .arg("-out")
            .arg(&gen_dir)
            .arg(&cadence_thrift)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to generate code for cadence.thrift: {}", stderr).into());
        }
        println!("cargo:warning=Generated code for cadence.thrift");
    }

    // Create mod.rs to export generated modules
    let mod_rs_content = generate_mod_rs(&gen_dir)?;
    let mod_rs_path = gen_dir.join("mod.rs");
    std::fs::write(&mod_rs_path, mod_rs_content)?;

    println!("cargo:warning=Created generated/mod.rs");

    Ok(())
}

fn use_pre_generated_code(crate_root: &PathBuf) {
    let gen_dir = crate_root.join("src/generated");

    if gen_dir.exists() && gen_dir.join("mod.rs").exists() {
        println!("cargo:warning=Using pre-generated Thrift code from src/generated/");
    } else {
        println!("cargo:warning=Pre-generated Thrift code not found at src/generated/");
        println!("cargo:warning=The project may not compile without the Thrift generated code");
        println!("cargo:warning=To generate code, install thrift compiler and run: cargo build --features gen-thrift");
    }
}

fn generate_mod_rs(gen_dir: &PathBuf) -> Result<String, Box<dyn std::error::Error>> {
    let mut modules = Vec::new();

    // Scan for generated .rs files
    if let Ok(entries) = std::fs::read_dir(gen_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.extension().map(|e| e == "rs").unwrap_or(false) {
                    if let Some(stem) = path.file_stem() {
                        let module_name = stem.to_string_lossy();
                        if module_name != "mod" {
                            modules.push(format!("pub mod {};", module_name));
                        }
                    }
                }
            }
        }
    }

    modules.sort();

    let content = format!(
        "//! Generated Thrift code for Cadence protocol\n\n#![allow(unknown_lints)]\n#![allow(clippy::all)]\n#![allow(dead_code)]\n\n{}\n",
        modules.join("\n")
    );

    Ok(content)
}
