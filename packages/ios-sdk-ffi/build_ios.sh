#!/bin/bash
set -e

# Build script for iOS SDK FFI
# This script builds the Rust library for iOS targets and creates an XCFramework

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_NAME="ios_sdk_ffi"
FRAMEWORK_NAME="DashSDK"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Building Dash iOS SDK...${NC}"

# Check if we have the required iOS targets installed
check_target() {
    if ! rustup target list --installed | grep -q "$1"; then
        echo -e "${YELLOW}Installing target $1...${NC}"
        rustup target add "$1"
    fi
}

# Install required targets
check_target "aarch64-apple-ios"
check_target "aarch64-apple-ios-sim"
check_target "x86_64-apple-ios"

# Build for iOS device (arm64)
echo -e "${GREEN}Building for iOS device (arm64)...${NC}"
cargo build --target aarch64-apple-ios --release

# Build for iOS simulator (arm64)
echo -e "${GREEN}Building for iOS simulator (arm64)...${NC}"
cargo build --target aarch64-apple-ios-sim --release

# Build for iOS simulator (x86_64)
echo -e "${GREEN}Building for iOS simulator (x86_64)...${NC}"
cargo build --target x86_64-apple-ios --release

# Create output directory
OUTPUT_DIR="$SCRIPT_DIR/build"
mkdir -p "$OUTPUT_DIR"

# Generate C headers
echo -e "${GREEN}Generating C headers...${NC}"
GENERATE_BINDINGS=1 cargo build --release
cp "$SCRIPT_DIR/target/release/build/"*"/out/ios_sdk_ffi.h" "$OUTPUT_DIR/" 2>/dev/null || {
    echo -e "${YELLOW}Warning: Could not find generated header. Running cbindgen manually...${NC}"
    cbindgen --config cbindgen.toml --crate ios-sdk-ffi --output "$OUTPUT_DIR/ios_sdk_ffi.h"
}

# Create fat library for simulator
echo -e "${GREEN}Creating universal simulator library...${NC}"
mkdir -p "$OUTPUT_DIR/simulator"
lipo -create \
    "$SCRIPT_DIR/target/x86_64-apple-ios/release/libios_sdk_ffi.a" \
    "$SCRIPT_DIR/target/aarch64-apple-ios-sim/release/libios_sdk_ffi.a" \
    -output "$OUTPUT_DIR/simulator/libios_sdk_ffi.a"

# Copy device library
echo -e "${GREEN}Copying device library...${NC}"
mkdir -p "$OUTPUT_DIR/device"
cp "$SCRIPT_DIR/target/aarch64-apple-ios/release/libios_sdk_ffi.a" "$OUTPUT_DIR/device/"

# Create module map
echo -e "${GREEN}Creating module map...${NC}"
cat > "$OUTPUT_DIR/module.modulemap" << EOF
module DashSDKFFI {
    header "ios_sdk_ffi.h"
    export *
}
EOF

# Create XCFramework
echo -e "${GREEN}Creating XCFramework...${NC}"
rm -rf "$OUTPUT_DIR/$FRAMEWORK_NAME.xcframework"

xcodebuild -create-xcframework \
    -library "$OUTPUT_DIR/device/libios_sdk_ffi.a" \
    -headers "$OUTPUT_DIR" \
    -library "$OUTPUT_DIR/simulator/libios_sdk_ffi.a" \
    -headers "$OUTPUT_DIR" \
    -output "$OUTPUT_DIR/$FRAMEWORK_NAME.xcframework"

echo -e "${GREEN}Build complete!${NC}"
echo -e "XCFramework created at: ${YELLOW}$OUTPUT_DIR/$FRAMEWORK_NAME.xcframework${NC}"
echo -e "To use in your iOS project:"
echo -e "1. Drag $FRAMEWORK_NAME.xcframework into your Xcode project"
echo -e "2. Import the module: ${YELLOW}import DashSDKFFI${NC}"