#!/bin/bash
# Helper script to generate a complete value-setter-zk transaction JSON
# with a mock proof for testing purposes.
#
# Usage: ./generate_transaction.sh <value> [gas]
#
# Examples:
#   ./generate_transaction.sh 42
#   ./generate_transaction.sh 100 "[1000, 0]"

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

# Generate the proof using the proof_generator example
echo "Generating mock proof for value: $VALUE..." >&2
PROOF_JSON=$(cargo run --quiet --example proof_generator --features native -- "$VALUE")

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
echo "✓ Transaction JSON generated successfully!" >&2
echo "Note: This uses a MOCK proof for testing only." >&2

