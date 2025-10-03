#!/bin/bash
# Helper script to generate a complete value-setter-zk transaction JSON
# with a real RISC0 proof.
#
# Usage: ./generate_risc0_transaction.sh <value> [gas]
#
# Examples:
#   ./generate_risc0_transaction.sh 42
#   ./generate_risc0_transaction.sh 100 '[1000, 0]'
#
# Note: This script requires RISC0 toolchain to be installed.
# See: https://dev.risczero.com/api/zkvm/install

set -e

if [ $# -lt 1 ]; then
    echo "Usage: $0 <value> [gas]"
    echo "Examples:"
    echo "  $0 42"
    echo "  $0 100 '[1000, 0]'"
    exit 1
fi

VALUE=$1
GAS=${2:-"null"}

# Navigate to the sov-value-setter-zk directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."

# Generate the proof using the RISC0 proof generator
echo "=== RISC0 Proof Generation ===" >&2
echo "Generating RISC0 proof for value: $VALUE..." >&2

# Run proof generator and capture output, separating stderr and stdout
TEMP_DIR=$(mktemp -d)
STDERR_FILE="$TEMP_DIR/stderr.txt"
STDOUT_FILE="$TEMP_DIR/stdout.txt"

cargo run --release --example risc0_proof_generator --features prove -- "$VALUE" 2>"$STDERR_FILE" >"$STDOUT_FILE"
RESULT=$?

# Show progress messages from stderr
cat "$STDERR_FILE" >&2

if [ $RESULT -ne 0 ]; then
    echo "Error: Failed to generate RISC0 proof" >&2
    cat "$STDOUT_FILE" >&2
    rm -rf "$TEMP_DIR"
    exit 1
fi

# Get the JSON from stdout
PROOF_JSON=$(cat "$STDOUT_FILE")

# Cleanup
rm -rf "$TEMP_DIR"

# Create the complete transaction JSON
cat <<EOF
{
    "set_value": {
        "value": $VALUE,
        "gas": $GAS,
        "proof": $PROOF_JSON
    }
}
EOF

echo "" >&2
echo "✓ Transaction JSON with RISC0 proof generated successfully!" >&2

