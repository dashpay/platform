#!/bin/bash

# Minimal script to generate Swift SDK bindings header
# This script should be run as a pre-build phase in Xcode

set -e

# Get the directory of this script
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$SCRIPT_DIR/../../../.."
SWIFT_SDK_DIR="$PROJECT_ROOT/packages/swift-sdk"
RS_SDK_FFI_DIR="$PROJECT_ROOT/packages/rs-sdk-ffi"
CDASHSDKFFI_DIR="$SWIFT_SDK_DIR/Sources/CDashSDKFFI"

echo "Checking for Swift SDK bindings..."

# Check if the header file exists
if [ ! -f "$CDASHSDKFFI_DIR/DashSDKFFI.h" ]; then
    echo "DashSDKFFI.h not found. Generating header..."
    
    # Create the directory if it doesn't exist
    mkdir -p "$CDASHSDKFFI_DIR"
    
    # Navigate to rs-sdk-ffi directory
    cd "$RS_SDK_FFI_DIR"
    
    # Generate only the header using cbindgen
    echo "Generating C header with cbindgen..."
    GENERATE_BINDINGS=1 cargo build --release --package rs-sdk-ffi
    
    # Check if the header was generated
    if [ -f "dash_sdk_ffi.h" ]; then
        # Copy the header to the expected location with the expected name
        cp "dash_sdk_ffi.h" "$CDASHSDKFFI_DIR/DashSDKFFI.h"
        echo "Successfully copied header to $CDASHSDKFFI_DIR/DashSDKFFI.h"
    else
        echo "Error: dash_sdk_ffi.h was not generated"
        echo ""
        echo "Please ensure you have cbindgen installed:"
        echo "  cargo install cbindgen"
        echo ""
        echo "Then manually generate the header by running:"
        echo "  cd $RS_SDK_FFI_DIR"
        echo "  GENERATE_BINDINGS=1 cargo build --release --package rs-sdk-ffi"
        echo "  cp dash_sdk_ffi.h $CDASHSDKFFI_DIR/DashSDKFFI.h"
        exit 1
    fi
    
    echo "Header generated successfully!"
    echo ""
    echo "NOTE: The iOS libraries still need to be built. To build them:"
    echo "  1. Install iOS targets: rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios"
    echo "  2. Run: cd $RS_SDK_FFI_DIR && ./build_ios.sh"
else
    echo "DashSDKFFI.h already exists. Skipping generation."
fi

# Verify all required files exist
if [ ! -f "$CDASHSDKFFI_DIR/DashSDKFFI.h" ]; then
    echo "Error: DashSDKFFI.h is missing after generation"
    exit 1
fi

if [ ! -f "$CDASHSDKFFI_DIR/module.modulemap" ]; then
    echo "Error: module.modulemap is missing"
    exit 1
fi

echo "All required files are present."