#!/bin/bash
# Script to generate Thrift code for Cadence

set -e

cd "$(dirname "$0")/.."

PROTO_DIR="cadence-proto"
THRIFT_DIR="$PROTO_DIR/thrift"
GEN_DIR="$PROTO_DIR/src/generated"

echo "Creating generated directory..."
mkdir -p "$GEN_DIR"

echo "Generating Rust code from Thrift IDL files..."

# Generate shared.thrift first
thrift -r --gen rs -out "$GEN_DIR" -I "$THRIFT_DIR" "$THRIFT_DIR/shared.thrift"
echo "✓ Generated shared.rs"

# Generate cadence.thrift (which includes shared.thrift)
thrift -r --gen rs -out "$GEN_DIR" -I "$THRIFT_DIR" "$THRIFT_DIR/cadence.thrift"
echo "✓ Generated cadence.rs"

echo "Creating mod.rs file..."
cat > "$GEN_DIR/mod.rs" << 'EOF'
pub mod shared;
pub mod cadence;
EOF
echo "✓ Generated mod.rs"

echo ""
echo "Thrift code generation complete!"
echo "Generated files are in: $GEN_DIR"
ls -lh "$GEN_DIR"
