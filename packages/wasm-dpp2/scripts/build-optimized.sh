#!/usr/bin/env bash
#
# Optimized build script for wasm-dpp2 npm release
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "Building wasm-dpp2 with full optimization for npm release..."

# The npm package ships DataContract.validateUpdate, which needs the crate's
# `validation` feature (see Cargo.toml). wasm-sdk builds wasm-dpp2 with its
# default features, so evo-sdk does not carry it.
export CARGO_BUILD_FEATURES="validation${CARGO_BUILD_FEATURES:+,$CARGO_BUILD_FEATURES}"

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
