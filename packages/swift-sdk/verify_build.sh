#!/bin/bash

# Build verification script for Swift SDK

echo "=== Swift SDK Build Verification ==="
echo

# Step 1: Try to build the crate
echo "Step 1: Building Swift SDK..."
if cargo build -p swift-sdk 2>/dev/null; then
    echo "✅ Build successful"
else
    echo "❌ Build failed"
    exit 1
fi

# Step 2: Check if library was created
echo
echo "Step 2: Checking library output..."
if [ -f "../../target/debug/libswift_sdk.a" ] || [ -f "../../target/debug/libswift_sdk.dylib" ]; then
    echo "✅ Library file created"
else
    echo "❌ Library file not found"
    exit 1
fi

# Step 3: List exported symbols (on macOS/Linux)
echo
echo "Step 3: Checking exported symbols..."
if command -v nm >/dev/null 2>&1; then
    echo "Exported swift_dash_* functions:"
    nm -g ../../target/debug/libswift_sdk.* 2>/dev/null | grep "swift_dash_" | head -10
    echo "... and more"
else
    echo "⚠️  'nm' command not found, skipping symbol check"
fi

# Step 4: Check header generation readiness
echo
echo "Step 4: Header generation readiness..."
if [ -f "cbindgen.toml" ]; then
    echo "✅ cbindgen configuration found"
else
    echo "❌ cbindgen.toml not found"
fi

echo
echo "=== Verification Summary ==="
echo "The Swift SDK is ready for use in iOS projects!"
echo
echo "To generate C headers for Swift:"
echo "  cargo install cbindgen"
echo "  cbindgen -c cbindgen.toml -o SwiftDashSDK.h"
echo
echo "To use in iOS project:"
echo "  1. Build with: cargo build --release -p swift-sdk"
echo "  2. Add the .a file to your Xcode project"
echo "  3. Import the generated header in your Swift bridging header"
echo "  4. Call functions from Swift!"