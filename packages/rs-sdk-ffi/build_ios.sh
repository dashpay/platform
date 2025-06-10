#!/bin/bash
set -e

# Build script for Dash SDK FFI (iOS targets)
# This script builds the Rust library for iOS targets and creates an XCFramework
# Usage: ./build_ios.sh [arm|x86|universal]
# Default: arm

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$SCRIPT_DIR/../.."
PROJECT_NAME="rs_sdk_ffi"
FRAMEWORK_NAME="DashSDK"

# Get architecture argument (default to arm)
BUILD_ARCH="${1:-arm}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Building Dash iOS SDK for architecture: $BUILD_ARCH${NC}"

# Check if we have the required iOS targets installed
check_target() {
    if ! rustup target list --installed | grep -q "$1"; then
        echo -e "${YELLOW}Installing target $1...${NC}"
        rustup target add "$1"
    fi
}

# Install required targets based on architecture
if [ "$BUILD_ARCH" = "x86" ]; then
    check_target "x86_64-apple-ios"
elif [ "$BUILD_ARCH" = "universal" ]; then
    check_target "aarch64-apple-ios"
    check_target "aarch64-apple-ios-sim"
    check_target "x86_64-apple-ios"
else
    # Default to ARM
    check_target "aarch64-apple-ios"
    check_target "aarch64-apple-ios-sim"
fi

# Build for iOS device (arm64) - always needed
if [ "$BUILD_ARCH" != "x86" ]; then
    echo -e "${GREEN}Building for iOS device (arm64)...${NC}"
    cargo build --target aarch64-apple-ios --release --package rs-sdk-ffi
fi

# Build for iOS simulator based on architecture
if [ "$BUILD_ARCH" = "x86" ]; then
    echo -e "${GREEN}Building for iOS simulator (x86_64)...${NC}"
    cargo build --target x86_64-apple-ios --release --package rs-sdk-ffi
elif [ "$BUILD_ARCH" = "universal" ]; then
    echo -e "${GREEN}Building for iOS simulator (arm64)...${NC}"
    cargo build --target aarch64-apple-ios-sim --release --package rs-sdk-ffi
    echo -e "${GREEN}Building for iOS simulator (x86_64)...${NC}"
    cargo build --target x86_64-apple-ios --release --package rs-sdk-ffi
else
    # Default to ARM
    echo -e "${GREEN}Building for iOS simulator (arm64)...${NC}"
    cargo build --target aarch64-apple-ios-sim --release --package rs-sdk-ffi
fi

# Create output directory
OUTPUT_DIR="$SCRIPT_DIR/build"
mkdir -p "$OUTPUT_DIR"

# Generate C headers
echo -e "${GREEN}Generating C headers...${NC}"
GENERATE_BINDINGS=1 cargo build --release --package rs-sdk-ffi
cp "$PROJECT_ROOT/target/release/build/"*"/out/dash_sdk_ffi.h" "$OUTPUT_DIR/" 2>/dev/null || {
    echo -e "${YELLOW}Warning: Could not find generated header. Running cbindgen manually...${NC}"
    cbindgen --config cbindgen.toml --crate rs-sdk-ffi --output "$OUTPUT_DIR/dash_sdk_ffi.h"
}

# Create simulator library based on architecture
echo -e "${GREEN}Creating simulator library...${NC}"
mkdir -p "$OUTPUT_DIR/simulator"

if [ "$BUILD_ARCH" = "x86" ]; then
    cp "$PROJECT_ROOT/target/x86_64-apple-ios/release/librs_sdk_ffi.a" "$OUTPUT_DIR/simulator/librs_sdk_ffi.a"
elif [ "$BUILD_ARCH" = "universal" ]; then
    echo -e "${GREEN}Creating universal simulator library...${NC}"
    lipo -create \
        "$PROJECT_ROOT/target/x86_64-apple-ios/release/librs_sdk_ffi.a" \
        "$PROJECT_ROOT/target/aarch64-apple-ios-sim/release/librs_sdk_ffi.a" \
        -output "$OUTPUT_DIR/simulator/librs_sdk_ffi.a"
else
    # Default to ARM
    cp "$PROJECT_ROOT/target/aarch64-apple-ios-sim/release/librs_sdk_ffi.a" "$OUTPUT_DIR/simulator/librs_sdk_ffi.a"
fi

# Copy device library (if built)
if [ "$BUILD_ARCH" != "x86" ]; then
    echo -e "${GREEN}Copying device library...${NC}"
    mkdir -p "$OUTPUT_DIR/device"
    cp "$PROJECT_ROOT/target/aarch64-apple-ios/release/librs_sdk_ffi.a" "$OUTPUT_DIR/device/"
fi

# Create module map
echo -e "${GREEN}Creating module map...${NC}"
cat > "$OUTPUT_DIR/module.modulemap" << EOF
module DashSDKFFI {
    header "dash_sdk_ffi.h"
    export *
}
EOF

# Prepare headers directory for XCFramework
echo -e "${GREEN}Preparing headers for XCFramework...${NC}"
HEADERS_DIR="$OUTPUT_DIR/headers"
mkdir -p "$HEADERS_DIR"
cp "$OUTPUT_DIR/dash_sdk_ffi.h" "$HEADERS_DIR/"
cp "$OUTPUT_DIR/module.modulemap" "$HEADERS_DIR/"

# Create XCFramework
echo -e "${GREEN}Creating XCFramework...${NC}"
rm -rf "$OUTPUT_DIR/$FRAMEWORK_NAME.xcframework"

# Build XCFramework command based on what was built
XCFRAMEWORK_CMD="xcodebuild -create-xcframework"

if [ "$BUILD_ARCH" != "x86" ] && [ -f "$OUTPUT_DIR/device/librs_sdk_ffi.a" ]; then
    XCFRAMEWORK_CMD="$XCFRAMEWORK_CMD -library $OUTPUT_DIR/device/librs_sdk_ffi.a -headers $HEADERS_DIR"
fi

if [ -f "$OUTPUT_DIR/simulator/librs_sdk_ffi.a" ]; then
    XCFRAMEWORK_CMD="$XCFRAMEWORK_CMD -library $OUTPUT_DIR/simulator/librs_sdk_ffi.a -headers $HEADERS_DIR"
fi

XCFRAMEWORK_CMD="$XCFRAMEWORK_CMD -output $OUTPUT_DIR/$FRAMEWORK_NAME.xcframework"

eval $XCFRAMEWORK_CMD

echo -e "${GREEN}Build complete!${NC}"
echo -e "XCFramework created at: ${YELLOW}$OUTPUT_DIR/$FRAMEWORK_NAME.xcframework${NC}"
echo -e "To use in your iOS project:"
echo -e "1. Drag $FRAMEWORK_NAME.xcframework into your Xcode project"
echo -e "2. Import the module: ${YELLOW}import DashSDKFFI${NC}"