#!/usr/bin/env bash
#
# Optimized build script for wasm-sdk npm release
#
set -euo pipefail

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Always use full optimization for npm releases
echo "Building wasm-sdk with full optimization for npm release..."

# Call unified build script with full optimization
"$SCRIPT_DIR/../scripts/build-wasm.sh" --package wasm-sdk --opt-level full

# Additional post-processing for npm release
echo "Post-processing for npm release..."

cd "$SCRIPT_DIR/pkg"

# Ensure the package.json is correct
if [ ! -f "package.json" ]; then
    echo "Error: package.json not found in pkg directory"
    exit 1
fi

# Verify all required files exist
REQUIRED_FILES=("wasm_sdk.js" "wasm_sdk.d.ts" "wasm_sdk_bg.wasm")
for file in "${REQUIRED_FILES[@]}"; do
    if [ ! -f "$file" ]; then
        echo "Error: Required file $file not found"
        exit 1
    fi
done

# Show final build info
echo "Build complete! Package contents:"
ls -lah

# Show WASM file size
WASM_SIZE=$(wc -c < wasm_sdk_bg.wasm)
WASM_SIZE_KB=$((WASM_SIZE / 1024))
echo "WASM file size: ${WASM_SIZE_KB}KB"

# Verify the package.json has correct name
if ! grep -q '"name": "dash"' package.json; then
    echo "Warning: package.json does not have 'dash' as the package name"
fi

echo "Ready for npm publish!"