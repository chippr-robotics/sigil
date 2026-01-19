#!/bin/bash
# Generate example artifacts for documentation and testing
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
ARTIFACTS_DIR="$REPO_ROOT/artifacts"
EXAMPLES_DIR="$ARTIFACTS_DIR/examples"
TEMP_MOTHER_DIR="$(mktemp -d -t sigil_example_mother.XXXXXX)"

echo "=== Sigil Example Artifacts Generator ==="
echo ""
echo "This script generates example artifacts for development and documentation."
echo "Generated artifacts will be placed in: $EXAMPLES_DIR"
echo ""

# Check if sigil-mother is available
if ! command -v sigil-mother &> /dev/null; then
    echo "Error: sigil-mother command not found"
    echo "Please build and install Sigil first:"
    echo "  cargo build --release"
    echo "  sudo ./scripts/install.sh"
    exit 1
fi

# Clean up on exit
cleanup() {
    if [ -d "$TEMP_MOTHER_DIR" ]; then
        echo "Cleaning up temporary mother data..."
        rm -rf "$TEMP_MOTHER_DIR"
    fi
}
trap cleanup EXIT

# Create temporary mother device
echo "Initializing temporary mother device..."
if ! sigil-mother init --data-dir "$TEMP_MOTHER_DIR" 2>&1 | grep -v "^INFO\|^DEBUG" > /tmp/sigil_init_errors.txt; then
    echo "Error: Failed to initialize mother device"
    cat /tmp/sigil_init_errors.txt
    exit 1
fi

echo "✓ Mother device initialized"
echo ""

# Generate example artifacts
generate_basic_child() {
    local name="$1"
    local presig_count="$2"
    local description="$3"
    
    echo "Generating: $name ($presig_count presigs)"
    echo "  Purpose: $description"
    
    if ! sigil-mother create-child \
        --presig-count "$presig_count" \
        --output "$EXAMPLES_DIR/${name}.img" \
        --agent-output "$EXAMPLES_DIR/${name}_agent_shares.json" \
        --data-dir "$TEMP_MOTHER_DIR" 2>&1 | grep -v "^INFO\|^DEBUG" > /tmp/sigil_create_errors.txt; then
        echo "  ✗ Error creating child disk"
        cat /tmp/sigil_create_errors.txt
        return 1
    fi
    
    echo "  ✓ Created: ${name}.img"
    echo "  ✓ Created: ${name}_agent_shares.json"
    echo ""
}

# Generate different example artifacts
echo "=== Generating Example Artifacts ==="
echo ""

generate_basic_child \
    "basic_child" \
    "100" \
    "Basic example of a freshly created child disk"

generate_basic_child \
    "small_child" \
    "50" \
    "Minimal child disk with only 50 presignatures"

echo "=== Summary ==="
echo ""
echo "Generated artifacts in: $EXAMPLES_DIR"
if compgen -G "$EXAMPLES_DIR/*.img" > /dev/null && compgen -G "$EXAMPLES_DIR/*.json" > /dev/null; then
    ls -lh "$EXAMPLES_DIR"/*.img "$EXAMPLES_DIR"/*.json 2>/dev/null
else
    echo "Warning: No artifact files found"
fi
echo ""
echo "✓ Example artifacts generated successfully!"
echo ""
echo "Next steps:"
echo "  1. Review the generated artifacts"
echo "  2. Update artifacts/examples/README.md with artifact descriptions"
echo "  3. Commit the artifacts: git add artifacts/ && git commit"
echo ""
echo "⚠️  Remember: These are TEST ARTIFACTS ONLY. Never use in production!"
