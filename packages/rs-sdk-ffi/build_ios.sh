#!/bin/bash
set -e

# Build script for Dash SDK FFI (iOS targets)
# This script builds the Rust library for iOS targets and creates an XCFramework
# Usage: ./build_ios.sh [arm|x86|universal]
# Default: arm
# Note: This builds rs-sdk-ffi with unified SDK functions that wrap both Core and Platform

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$SCRIPT_DIR/../.."
PROJECT_NAME="rs_sdk_ffi"

# Parse arguments
BUILD_ARCH="${1:-arm}"

# Parse command line arguments
for arg in "$@"; do
    case $arg in
        arm|x86|universal)
            BUILD_ARCH="$arg"
            shift
            ;;
    esac
done

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Build with unified SDK support
CARGO_FEATURES=""
FRAMEWORK_NAME="DashSDKFFI"

echo -e "${GREEN}Building Dash SDK FFI for iOS ($BUILD_ARCH)${NC}"

# Check if we have the required iOS targets installed
check_target() {
    if ! rustup target list --installed | grep -q "$1"; then
        echo -e "${YELLOW}Installing target $1...${NC}"
        rustup target add "$1" > /tmp/rustup_target.log 2>&1
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
    echo -ne "${GREEN}Building for iOS device (arm64)...${NC}"
    if cargo build --lib --target aarch64-apple-ios --release --package rs-sdk-ffi $CARGO_FEATURES > /tmp/cargo_build_device.log 2>&1; then
        echo -e "\r${GREEN}✓ iOS device (arm64) build successful${NC}       "
    else
        echo -e "\r${RED}✗ iOS device build failed${NC}              "
        cat /tmp/cargo_build_device.log
        exit 1
    fi
fi

# Build for iOS simulator based on architecture
if [ "$BUILD_ARCH" = "x86" ]; then
    echo -ne "${GREEN}Building for iOS simulator (x86_64)...${NC}"
    if cargo build --lib --target x86_64-apple-ios --release --package rs-sdk-ffi $CARGO_FEATURES > /tmp/cargo_build_sim_x86.log 2>&1; then
        echo -e "\r${GREEN}✓ iOS simulator (x86_64) build successful${NC}      "
    else
        echo -e "\r${RED}✗ iOS simulator (x86_64) build failed${NC}          "
        cat /tmp/cargo_build_sim_x86.log
        exit 1
    fi
elif [ "$BUILD_ARCH" = "universal" ]; then
    echo -ne "${GREEN}Building for iOS simulator (arm64)...${NC}"
    if cargo build --lib --target aarch64-apple-ios-sim --release --package rs-sdk-ffi $CARGO_FEATURES > /tmp/cargo_build_sim_arm.log 2>&1; then
        echo -e "\r${GREEN}✓ iOS simulator (arm64) build successful${NC}       "
    else
        echo -e "\r${RED}✗ iOS simulator (arm64) build failed${NC}           "
        cat /tmp/cargo_build_sim_arm.log
        exit 1
    fi
    echo -ne "${GREEN}Building for iOS simulator (x86_64)...${NC}"
    if cargo build --lib --target x86_64-apple-ios --release --package rs-sdk-ffi $CARGO_FEATURES > /tmp/cargo_build_sim_x86.log 2>&1; then
        echo -e "\r${GREEN}✓ iOS simulator (x86_64) build successful${NC}      "
    else
        echo -e "\r${RED}✗ iOS simulator (x86_64) build failed${NC}          "
        cat /tmp/cargo_build_sim_x86.log
        exit 1
    fi
else
    # Default to ARM
    echo -ne "${GREEN}Building for iOS simulator (arm64)...${NC}"
    if cargo build --lib --target aarch64-apple-ios-sim --release --package rs-sdk-ffi $CARGO_FEATURES > /tmp/cargo_build_sim_arm.log 2>&1; then
        echo -e "\r${GREEN}✓ iOS simulator (arm64) build successful${NC}       "
    else
        echo -e "\r${RED}✗ iOS simulator (arm64) build failed${NC}           "
        cat /tmp/cargo_build_sim_arm.log
        exit 1
    fi
fi

# Create output directory
OUTPUT_DIR="$SCRIPT_DIR/build"
mkdir -p "$OUTPUT_DIR"

# Generate C headers
echo -ne "${GREEN}Generating C headers...${NC}"
cd "$PROJECT_ROOT"
if GENERATE_BINDINGS=1 cargo build --lib --release --package rs-sdk-ffi $CARGO_FEATURES > /tmp/cargo_build_headers.log 2>&1; then
    if cp "$PROJECT_ROOT/target/release/build/"*"/out/dash_sdk_ffi.h" "$OUTPUT_DIR/" 2>/dev/null; then
        echo -e "\r${GREEN}✓ Headers generated successfully${NC}              "
    else
        echo -e "\r${YELLOW}⚠ Generated header not found, using cbindgen...${NC}"
        cd "$SCRIPT_DIR"
        if cbindgen --config cbindgen-ios.toml --crate rs-sdk-ffi --output "$OUTPUT_DIR/dash_sdk_ffi.h" > /tmp/cbindgen.log 2>&1; then
            echo -e "${GREEN}✓ Headers generated with cbindgen${NC}"
        else
            echo -e "${RED}✗ Failed to generate headers${NC}"
            cat /tmp/cbindgen.log
            exit 1
        fi
    fi
else
    echo -e "\r${RED}✗ Header generation build failed${NC}              "
    cat /tmp/cargo_build_headers.log
    exit 1
fi

# Merge all FFI headers to create unified header
echo -e "${GREEN}Merging headers...${NC}"
RUST_DASHCORE_PATH="$PROJECT_ROOT/../rust-dashcore"
KEY_WALLET_HEADER_PATH="$RUST_DASHCORE_PATH/key-wallet-ffi/include/key_wallet_ffi.h"
SPV_HEADER_PATH="$RUST_DASHCORE_PATH/dash-spv-ffi/include/dash_spv_ffi.h"

if [ -f "$KEY_WALLET_HEADER_PATH" ] && [ -f "$SPV_HEADER_PATH" ]; then
    # Create merged header with unified include guard
    MERGED_HEADER="$OUTPUT_DIR/dash_unified_ffi.h"
    
    # Start with unified include guard
    cat > "$MERGED_HEADER" << 'EOF'
#ifndef DASH_UNIFIED_FFI_H
#define DASH_UNIFIED_FFI_H

#pragma once

/* This file is auto-generated by merging Dash SDK, SPV FFI, and Key Wallet FFI headers. Do not modify manually. */

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Key Wallet FFI Functions and Types
// ============================================================================

EOF
    
    # Extract Key Wallet FFI content
    # 1. Skip everything up to and including the last #include <stdlib.h>
    # 2. Skip header guards and pragma once
    # 3. Strip out all __cplusplus extern "C" blocks (we'll add them properly at the end)
    # 4. Fix ManagedWalletInfo reference to FFIManagedWalletInfo
    # 5. Stop at the header guard closing
    awk '
        BEGIN { found_stdlib = 0; in_content = 0 }
        /^#include <stdlib\.h>/ { found_stdlib = 1; next }
        /^#include <stdint\.h>/ { next }
        /^#include <stddef\.h>/ { next }
        /^#include <stdbool\.h>/ { next }
        /^#ifndef KEY_WALLET_FFI_H/ { next }
        /^#define KEY_WALLET_FFI_H/ { next }
        /^#pragma once/ { next }
        /^\/\* Warning: This file is auto-generated/ { next }
        /^\/\* Generated with cbindgen/ { next }
        found_stdlib && /^\/\*/ { in_content = 1 }
        found_stdlib && /^typedef/ { in_content = 1 }
        /^#ifdef __cplusplus$/ { 
            in_content = 1
            next  # Skip the ifdef __cplusplus line
        }
        /^extern "C" \{$/ { next }  # Skip extern "C" opening
        /^}  \/\/ extern "C"$/ { next }  # Skip extern "C" closing
        /^#endif.*__cplusplus/ { next }  # Skip any endif with __cplusplus
        /^#endif  \/\* KEY_WALLET_FFI_H \*\/$/ { exit }
        in_content {
            # Fix the ManagedWalletInfo reference in FFIManagedWallet struct
            if (/ManagedWalletInfo \*inner;/) {
                gsub(/ManagedWalletInfo \*inner;/, "FFIManagedWalletInfo *inner;")
            }
            print
        }
    ' "$KEY_WALLET_HEADER_PATH" >> "$MERGED_HEADER"
    
    # Add separator for SPV FFI
    cat >> "$MERGED_HEADER" << 'EOF'

// ============================================================================
// Dash SPV FFI Functions and Types
// ============================================================================

// Forward declaration for FFIClientConfig (opaque type)
typedef struct FFIClientConfig FFIClientConfig;

EOF
    
    # Extract SPV FFI content
    # Skip duplicate types that conflict with key-wallet-ffi
    awk '
        BEGIN { skip = 0; in_enum = 0 }
        /^#include/ { next }
        /^typedef enum FFINetwork \{/ { skip = 1; in_enum = 1; next }
        in_enum && /^\} FFINetwork;/ { skip = 0; in_enum = 0; next }
        /^typedef struct CoreSDKHandle \{/ { skip = 1 }
        /^\} CoreSDKHandle;/ && skip { skip = 0; next }
        /^typedef ClientConfig FFIClientConfig;/ { next }  # Skip broken typedef
        !skip && !in_enum { print }
    ' "$SPV_HEADER_PATH" >> "$MERGED_HEADER"
    
    # Add separator and SDK content
    cat >> "$MERGED_HEADER" << 'EOF'

// ============================================================================
// Dash SDK FFI Functions and Types  
// ============================================================================

EOF
    
    # Extract SDK FFI content (skip the header include guards and system includes)
    sed -e '1,/^#include <stdlib\.h>/d' \
        -e '/^#ifndef DASH_SDK_FFI_H$/d' \
        -e '/^#define DASH_SDK_FFI_H$/d' \
        -e '/^#endif.*DASH_SDK_FFI_H.*$/d' \
        -e '/^#pragma once$/d' \
        -e '/^#ifdef __cplusplus$/d' \
        -e '/^extern "C" {$/d' \
        -e '/^}  \/\/ extern "C"$/d' \
        -e '/^#endif.*__cplusplus.*$/d' \
        "$OUTPUT_DIR/dash_sdk_ffi.h" >> "$MERGED_HEADER"
    
    # Close C++ guard and add compatibility notes
    cat >> "$MERGED_HEADER" << 'EOF'

// ============================================================================
// Type Compatibility Notes
// ============================================================================

// This unified header combines types from:
// 1. Key Wallet FFI - Core wallet functionality (addresses, keys, UTXOs)
// 2. Dash SPV FFI - SPV client and network functionality
// 3. Dash SDK FFI - Platform SDK for identities and documents
//
// Naming conflicts have been resolved:
// - FFINetwork enum is used from key-wallet-ffi only
// - CoreSDKHandle from SPV header is removed to avoid conflicts
// - ManagedWalletInfo references are properly prefixed with FFI

#ifdef __cplusplus
}  // extern "C"
#endif

#endif /* DASH_UNIFIED_FFI_H */
EOF
    
    # Replace the original header reference with unified header
    cp "$MERGED_HEADER" "$OUTPUT_DIR/dash_sdk_ffi.h"
    echo -e "${GREEN}✓ Headers merged successfully${NC}"
else
    echo -e "${YELLOW}⚠ Key Wallet FFI or SPV FFI headers not found${NC}"
    echo -e "${YELLOW}  Please build key-wallet-ffi and dash-spv-ffi first:${NC}"
    echo -e "${YELLOW}  cd ../../../rust-dashcore/key-wallet-ffi && cargo build --release${NC}"
    echo -e "${YELLOW}  cd ../../../rust-dashcore/dash-spv-ffi && cargo build --release${NC}"
fi

# Create simulator library based on architecture
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
    mkdir -p "$OUTPUT_DIR/device"
    cp "$PROJECT_ROOT/target/aarch64-apple-ios/release/librs_sdk_ffi.a" "$OUTPUT_DIR/device/"
fi

# Create module map for both DashSDKFFI and DashSPVFFI
cat > "$OUTPUT_DIR/module.modulemap" << EOF
module DashSDKFFI {
    header "dash_sdk_ffi.h"
    export *
}

module DashSPVFFI {
    header "dash_sdk_ffi.h"
    export *
}
EOF

# Prepare headers directory for XCFramework
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

if eval $XCFRAMEWORK_CMD > /tmp/xcframework.log 2>&1; then
    echo -e "${GREEN}✓ XCFramework created successfully${NC}"
else
    echo -e "${RED}✗ XCFramework creation failed${NC}"
    cat /tmp/xcframework.log
    exit 1
fi

echo -e "\n${GREEN}Build complete!${NC}"
echo -e "Output: ${YELLOW}$OUTPUT_DIR/$FRAMEWORK_NAME.xcframework${NC}"

# Copy XCFramework to Swift SDK directory
SWIFT_SDK_DIR="$PROJECT_ROOT/../swift-sdk"
if [ -d "$SWIFT_SDK_DIR" ]; then
    echo -e "\n${GREEN}Copying XCFramework to Swift SDK...${NC}"
    rm -rf "$SWIFT_SDK_DIR/$FRAMEWORK_NAME.xcframework"
    cp -R "$OUTPUT_DIR/$FRAMEWORK_NAME.xcframework" "$SWIFT_SDK_DIR/"
    echo -e "${GREEN}✓ XCFramework copied to ${YELLOW}$SWIFT_SDK_DIR/$FRAMEWORK_NAME.xcframework${NC}"
fi