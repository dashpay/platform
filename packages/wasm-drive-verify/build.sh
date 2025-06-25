#!/usr/bin/env bash
set -euo pipefail

# Always run from this script's location
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "Building wasm-drive-verify..."

# Build the Rust WASM target
echo "Building Rust WASM target..."
cargo build --target wasm32-unknown-unknown --release

# Create pkg directory if it doesn't exist
mkdir -p pkg

# Run wasm-bindgen
echo "Running wasm-bindgen..."
if ! command -v wasm-bindgen &> /dev/null; then
  echo "Error: 'wasm-bindgen' not found. Install via 'cargo install wasm-bindgen-cli'." >&2
  exit 1
fi
wasm-bindgen ../../target/wasm32-unknown-unknown/release/wasm_drive_verify.wasm \
    --out-dir pkg \
    --target web \
    --no-modules-global \
    --split

# Create proper package.json if it doesn't exist or is incomplete
if [ ! -f pkg/package.json ] || ! grep -q '"name"' pkg/package.json; then
    echo "Creating package.json..."
    # Extract version from Cargo.toml
    VERSION=$(grep -E '^version =' Cargo.toml | head -1 | sed -E 's/version = "([^"]+)"/\1/')
    cat > pkg/package.json << EOF
{
  "name": "wasm-drive-verify",
  "version": "$VERSION",
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