#!/bin/bash
# Setup script for iOS build environment

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../.." && pwd )"

echo "🔧 Setting up iOS build environment..."

# Step 1: Build the Rust FFI
echo "📦 Building Rust FFI..."
cd "$PROJECT_ROOT/packages/rs-sdk-ffi"
if [ ! -f "build_ios.sh" ]; then
    echo "❌ Error: build_ios.sh not found in rs-sdk-ffi directory"
    exit 1
fi

./build_ios.sh

# Check if build succeeded
if [ ! -d "build/DashUnifiedSDK.xcframework" ]; then
    echo "❌ Error: FFI build failed - xcframework not found"
    exit 1
fi

# Step 2: Setup symlinks for Swift SDK
echo "🔗 Setting up symlinks..."
cd "$PROJECT_ROOT/packages/swift-sdk"

# Create CDashSDKFFI directory if it doesn't exist
mkdir -p Sources/CDashSDKFFI

# Remove old symlink if it exists
if [ -L "Sources/CDashSDKFFI/dash_sdk_ffi.h" ]; then
    rm "Sources/CDashSDKFFI/dash_sdk_ffi.h"
fi

# Create symlink to the FFI header
if [ -f "$PROJECT_ROOT/packages/rs-sdk-ffi/build/DashUnifiedSDK.xcframework/ios-arm64/Headers/dash_sdk_ffi.h" ]; then
    ln -sf "$PROJECT_ROOT/packages/rs-sdk-ffi/build/DashUnifiedSDK.xcframework/ios-arm64/Headers/dash_sdk_ffi.h" "Sources/CDashSDKFFI/dash_sdk_ffi.h"
    echo "✅ Header symlink created"
else
    echo "❌ Error: FFI header not found at expected location"
    exit 1
fi

# Step 3: Clean build directory
echo "🧹 Cleaning build artifacts..."
cd "$PROJECT_ROOT/packages/swift-sdk/SwiftExampleApp"
if [ -d "DerivedData" ]; then
    rm -rf DerivedData
fi

# Clean Xcode DerivedData
echo "🧹 Cleaning Xcode DerivedData..."
xcodebuild clean -project SwiftExampleApp.xcodeproj -scheme SwiftExampleApp 2>/dev/null || true

# Step 4: Verify setup
echo "✅ Verifying setup..."
if [ ! -L "$PROJECT_ROOT/packages/swift-sdk/Sources/CDashSDKFFI/dash_sdk_ffi.h" ]; then
    echo "❌ Error: Header symlink not found"
    exit 1
fi

if [ ! -d "$PROJECT_ROOT/packages/rs-sdk-ffi/build/DashUnifiedSDK.xcframework" ]; then
    echo "❌ Error: XCFramework not found"
    exit 1
fi

echo "✅ iOS build environment setup complete!"
echo ""
echo "📱 You can now build SwiftExampleApp with:"
echo "   cd $PROJECT_ROOT/packages/swift-sdk"
echo "   xcodebuild -project SwiftExampleApp/SwiftExampleApp.xcodeproj -scheme SwiftExampleApp -sdk iphonesimulator -destination 'platform=iOS Simulator,name=iPhone 16' build"