#!/bin/bash

# Build optimized WASM SDK

set -e

echo "Building optimized WASM SDK..."

# Clean previous builds
rm -rf pkg target

# Set optimization flags
export RUSTFLAGS="-C opt-level=z -C lto=fat -C embed-bitcode=yes -C strip=symbols"

# Build with wasm-pack
wasm-pack build --release \
  --target web \
  --out-dir pkg \
  --no-typescript \
  -- --features wasm

echo "Running wasm-opt for additional optimization..."

# Install wasm-opt if not available
if ! command -v wasm-opt &> /dev/null; then
    echo "wasm-opt not found. Please install binaryen:"
    echo "  brew install binaryen  # macOS"
    echo "  apt-get install binaryen  # Ubuntu/Debian"
    exit 1
fi

# Optimize with wasm-opt
wasm-opt -Oz \
  --enable-simd \
  --enable-bulk-memory \
  --converge \
  pkg/wasm_sdk_bg.wasm \
  -o pkg/wasm_sdk_bg_optimized.wasm

# Replace original with optimized
mv pkg/wasm_sdk_bg_optimized.wasm pkg/wasm_sdk_bg.wasm

# Generate size report
echo ""
echo "Size report:"
ls -lh pkg/wasm_sdk_bg.wasm

# Optional: Use wasm-snip to remove unused functions
# wasm-snip pkg/wasm_sdk_bg.wasm -o pkg/wasm_sdk_bg.wasm

echo ""
echo "Build complete! Output in pkg/"