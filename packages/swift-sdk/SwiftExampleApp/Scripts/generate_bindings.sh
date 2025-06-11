#!/bin/bash

# Script to generate Swift SDK bindings if they don't exist
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
    echo "DashSDKFFI.h not found. Generating bindings..."
    
    # Create the directory if it doesn't exist
    mkdir -p "$CDASHSDKFFI_DIR"
    
    # Navigate to rs-sdk-ffi directory
    cd "$RS_SDK_FFI_DIR"
    
    # Generate the header using cargo build with GENERATE_BINDINGS
    echo "Generating C header..."
    GENERATE_BINDINGS=1 cargo build --release --package rs-sdk-ffi
    
    # Find the generated header in the target directory
    HEADER_PATH=$(find "$PROJECT_ROOT/target" -name "dash_sdk_ffi.h" -type f | head -1)
    
    if [ -n "$HEADER_PATH" ] && [ -f "$HEADER_PATH" ]; then
        # Copy the header to the expected location with the expected name
        cp "$HEADER_PATH" "$CDASHSDKFFI_DIR/DashSDKFFI.h"
        echo "Successfully copied header from $HEADER_PATH to $CDASHSDKFFI_DIR/DashSDKFFI.h"
    else
        echo "Error: dash_sdk_ffi.h was not generated"
        echo "Please ensure cbindgen is available and try again"
        exit 1
    fi
    
    echo "Swift SDK header generated successfully!"
    echo ""
    echo "NOTE: The iOS libraries (.xcframework) still need to be built separately."
    echo "To build the complete iOS framework, run:"
    echo "  cd $RS_SDK_FFI_DIR && ./build_ios.sh"
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

echo "All required header files are present."