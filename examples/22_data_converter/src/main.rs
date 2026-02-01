//! # Example 22: Data Converter
//!
//! This example demonstrates custom data converter and serialization patterns.

use data_converter::*;
use examples_common::tracing_setup::init_tracing;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    println!("\n=== Cadence Rust Client - Data Converter Example ===\n");
    println!("This example demonstrates:");
    println!("1. Custom DataConverter implementation");
    println!("2. Alternative serialization formats");
    println!("3. Encryption at rest");

    Ok(())
}

// #[cfg(test)]
// mod tests {
//     use super::*;
// 
//     #[tokio::test]
//     async fn test_data_converter() {
//         // Test data converter functionality
//     }
// }
