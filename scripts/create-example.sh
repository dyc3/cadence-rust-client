#!/usr/bin/env bash
# Script to create a new example following the standard template
# Usage: ./scripts/create-example.sh <number> <name> <dependencies>
# Example: ./scripts/create-example.sh 02 activity_basics "01_hello_workflow"

set -e

if [ $# -lt 2 ]; then
    echo "Usage: $0 <number> <name> [dependencies]"
    echo "Example: $0 02 activity_basics '01_hello_workflow'"
    exit 1
fi

NUMBER=$1
NAME=$2
DEPS=$3
EXAMPLE_DIR="examples/${NUMBER}_${NAME}"

# Create directory structure
echo "Creating example ${NUMBER}_${NAME}..."
mkdir -p "${EXAMPLE_DIR}/src/tests"
touch "${EXAMPLE_DIR}/src/tests/unit_tests.rs"
touch "${EXAMPLE_DIR}/src/tests/integration_tests.rs"

# Create Cargo.toml
cat > "${EXAMPLE_DIR}/Cargo.toml" << EOF
[package]
name = "${NUMBER}_${NAME}"
version = "0.1.0"
edition = "2021"
description = "Example demonstrating ${NAME//_/ }"

[dependencies]
uber_cadence_client = { path = "../../uber_cadence_client" }
uber_cadence_worker = { path = "../../uber_cadence_worker" }
uber_cadence_workflow = { path = "../../uber_cadence_workflow" }
uber_cadence_activity = { path = "../../uber_cadence_activity" }
uber_cadence_core = { path = "../../uber_cadence_core" }
uber_cadence_testsuite = { path = "../../uber_cadence_testsuite" }
examples_common = { path = "../examples-common", package = "examples_common" }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = "0.3"
chrono = { workspace = true }
uuid = { workspace = true }

[dev-dependencies]
tokio-test = { workspace = true }
EOF

# Create lib.rs
cat > "${EXAMPLE_DIR}/src/lib.rs" << EOF
//! ${NAME//_/ } example library exports.
//!
//! This module provides the workflow and activity definitions
//! that can be used by other examples.

pub mod activities;
pub mod workflows;
pub mod types;
EOF

# Create activities/mod.rs
mkdir -p "${EXAMPLE_DIR}/src/activities"
cat > "${EXAMPLE_DIR}/src/activities/mod.rs" << EOF
//! Activity implementations for ${NAME//_/ } example.

use uber_cadence_activity::{ActivityContext, ActivityError};
use serde::{Deserialize, Serialize};

/// Example activity input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleActivityInput {
    pub data: String,
}

/// Example activity output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleActivityOutput {
    pub result: String,
}

/// Example activity implementation
pub async fn example_activity(
    _ctx: &ActivityContext,
    input: ExampleActivityInput,
) -> Result<ExampleActivityOutput, ActivityError> {
    Ok(ExampleActivityOutput {
        result: format!("Processed: {}", input.data),
    })
}
EOF

# Create workflows/mod.rs
mkdir -p "${EXAMPLE_DIR}/src/workflows"
cat > "${EXAMPLE_DIR}/src/workflows/mod.rs" << EOF
//! Workflow implementations for ${NAME//_/ } example.

use uber_cadence_core::ActivityOptions;
use uber_cadence_workflow::{WorkflowContext, WorkflowError};
use serde::{Deserialize, Serialize};
use crate::activities::{ExampleActivityInput, example_activity};

/// Example workflow input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleWorkflowInput {
    pub name: String,
}

/// Example workflow output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleWorkflowOutput {
    pub greeting: String,
}

/// Example workflow implementation
pub async fn example_workflow(
    ctx: &mut WorkflowContext,
    input: ExampleWorkflowInput,
) -> Result<ExampleWorkflowOutput, WorkflowError> {
    let activity_input = ExampleActivityInput {
        data: input.name.clone(),
    };

    let result = ctx.execute_activity(
        "example_activity",
        Some(serde_json::to_vec(&activity_input).unwrap()),
        ActivityOptions::default(),
    ).await?;

    let activity_output = serde_json::from_slice(&result.unwrap_or_default())
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse: {}", e)))?;

    Ok(ExampleWorkflowOutput {
        greeting: activity_output.result,
    })
}
EOF

# Create types/mod.rs
mkdir -p "${EXAMPLE_DIR}/src/types"
cat > "${EXAMPLE_DIR}/src/types/mod.rs" << EOF
//! Type definitions for ${NAME//_/ } example.

use serde::{Deserialize, Serialize};

/// Common error type for this example
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExampleError {
    NotFound,
    InvalidInput(String),
    ProcessingFailed(String),
}
EOF

# Create main.rs
cat > "${EXAMPLE_DIR}/src/main.rs" << 'EOF'
//! # Example NUMBER: TITLE
//!
//! This example demonstrates:
//! - Feature 1
//! - Feature 2
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run -p NUMBER_NAME
//! ```

use uber_cadence_testsuite::TestWorkflowEnvironment;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    println!("\n=== Example NUMBER: TITLE ===\n");

    let mut env = TestWorkflowEnvironment::new();
    info!("Test environment created");

    // TODO: Implement example logic

    println!("\nExample completed!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_example() {
        let _env = TestWorkflowEnvironment::new();
        // TODO: Add tests
    }
}
EOF

# Replace placeholders in main.rs
sed -i "s/NUMBER/${NUMBER}/g" "${EXAMPLE_DIR}/src/main.rs"
sed -i "s/NAME/${NAME}/g" "${EXAMPLE_DIR}/src/main.rs"
sed -i "s/TITLE/${NAME//_/ }/g" "${EXAMPLE_DIR}/src/main.rs"

# Create README.md
cat > "${EXAMPLE_DIR}/README.md" << EOF
# Example ${NUMBER}: ${NAME//_/ }

## Overview

This example demonstrates ${NAME//_/ }.

## Features Demonstrated

- Feature 1: Description
- Feature 2: Description

## Running the Example

\`\`\`bash
cargo run -p ${NUMBER}_${NAME}
\`\`\`

## Running Tests

\`\`\`bash
cargo test -p ${NUMBER}_${NAME}
\`\`\`

## Dependencies

${DEPS:+- Previous: $DEPS}

## Learn More

- [Cadence Documentation](https://cadenceworkflow.io/)
EOF

echo "✓ Created ${EXAMPLE_DIR}"
echo "  - Update Cargo.toml to add to workspace"
echo "  - Edit src/main.rs to implement the example"
echo "  - Edit README.md with proper documentation"
echo "  - Add tests to src/tests/"
