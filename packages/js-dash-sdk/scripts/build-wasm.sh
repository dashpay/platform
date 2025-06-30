#!/bin/bash

# Build the WASM SDK and copy it to this package
echo "Building WASM SDK..."

# Navigate to wasm-sdk directory
cd ../wasm-sdk

# Build the WASM package
wasm-pack build --target web --out-dir ../js-dash-sdk/wasm --no-typescript

# Return to js-dash-sdk directory  
cd ../js-dash-sdk

echo "WASM SDK build complete!"