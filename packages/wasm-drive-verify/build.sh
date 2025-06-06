#!/bin/bash

set -e

echo "Building wasm-drive-verify..."

# Build the Rust WASM target
echo "Building Rust WASM target..."
cargo build --target wasm32-unknown-unknown --release

# Create pkg directory if it doesn't exist
mkdir -p pkg

# Run wasm-bindgen
echo "Running wasm-bindgen..."
wasm-bindgen ../../target/wasm32-unknown-unknown/release/wasm_drive_verify.wasm \
    --out-dir pkg \
    --target web

# Create proper package.json if it doesn't exist or is incomplete
if [ ! -f pkg/package.json ] || ! grep -q '"name"' pkg/package.json; then
    echo "Creating package.json..."
    cat > pkg/package.json << 'EOF'
{
  "name": "wasm-drive-verify",
  "version": "1.8.0",
  "description": "WASM bindings for Drive verify functions",
  "main": "wasm_drive_verify.js",
  "types": "wasm_drive_verify.d.ts",
  "author": "Dash Core Group <dev@dash.org>",
  "license": "MIT",
  "repository": {
    "type": "git",
    "url": "https://github.com/dashpay/platform.git"
  },
  "files": [
    "wasm_drive_verify_bg.wasm",
    "wasm_drive_verify.js",
    "wasm_drive_verify.d.ts",
    "wasm_drive_verify_bg.wasm.d.ts"
  ]
}
EOF
fi

echo "Build complete!"
echo "Output files are in the pkg/ directory"