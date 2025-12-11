#!/usr/bin/env bash
#
# Optimized build script for wasm-dpp2 npm release
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "Building wasm-dpp2 with full optimization for npm release..."

"$SCRIPT_DIR/../../scripts/build-wasm.sh" --package wasm-dpp2 --opt-level full

cd "$SCRIPT_DIR/../pkg"

REQUIRED_FILES=("wasm_dpp2.js" "wasm_dpp2.d.ts" "wasm_dpp2_bg.wasm")
for file in "${REQUIRED_FILES[@]}"; do
    if [ ! -f "$file" ]; then
        echo "Error: Required file $file not found"
        exit 1
    fi
done

echo "Package contents:"
ls -lah

WASM_SIZE=$(wc -c < wasm_dpp2_bg.wasm)
WASM_SIZE_KB=$((WASM_SIZE / 1024))
echo "WASM file size: ${WASM_SIZE_KB}KB"
