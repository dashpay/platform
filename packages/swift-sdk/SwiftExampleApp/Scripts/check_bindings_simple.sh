#!/bin/bash
set -e

# Simple script to check if Swift SDK bindings exist
PROJECT_ROOT="${SRCROOT}/../../../.."
SWIFT_SDK_DIR="${PROJECT_ROOT}/packages/swift-sdk"
CDASHSDKFFI_DIR="${SWIFT_SDK_DIR}/Sources/CDashSDKFFI"

echo "Checking for Swift SDK bindings..."

# Check if the header file exists
if [ ! -f "$CDASHSDKFFI_DIR/DashSDKFFI.h" ]; then
    echo "❌ ERROR: DashSDKFFI.h not found!"
    echo ""
    echo "The Swift SDK bindings have not been generated. To fix this:"
    echo ""
    echo "1. Open Terminal"
    echo "2. Navigate to the project:"
    echo "   cd ${PROJECT_ROOT}/packages/rs-sdk-ffi"
    echo ""
    echo "3. Generate the bindings:"
    echo "   GENERATE_BINDINGS=1 cargo build --release --package rs-sdk-ffi"
    echo ""
    echo "4. Copy the generated header:"
    echo "   find ${PROJECT_ROOT}/target -name 'dash_sdk_ffi.h' -exec cp {} ${CDASHSDKFFI_DIR}/DashSDKFFI.h \;"
    echo ""
    echo "5. Build the iOS framework (optional, for full functionality):"
    echo "   ./build_ios.sh"
    echo ""
    echo "6. Try building the app again in Xcode."
    echo ""
    exit 1
fi

# Check if the module map exists
if [ ! -f "$CDASHSDKFFI_DIR/module.modulemap" ]; then
    echo "❌ ERROR: module.modulemap is missing at $CDASHSDKFFI_DIR"
    exit 1
fi

echo "✅ Swift SDK bindings are present!"