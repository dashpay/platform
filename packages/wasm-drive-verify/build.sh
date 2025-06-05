#!/bin/bash

set -e

echo "Building wasm-drive-verify..."

# Install wasm-pack if not already installed
if ! command -v wasm-pack &> /dev/null; then
    echo "Installing wasm-pack..."
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
fi

# Build the WASM module
echo "Building WASM module..."
wasm-pack build --target web --out-dir pkg --release

# Generate TypeScript definitions if needed
echo "Build complete!"
echo "Output files are in the pkg/ directory"