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

// Forward declarations to ensure cross-refs compile regardless of merge order
typedef struct FFIClientConfig FFIClientConfig;
// Provide explicit opaque definitions so Swift can import the type names
typedef struct FFIDashSpvClient { unsigned char _private[0]; } FFIDashSpvClient;
typedef struct FFIWallet { unsigned char _private[0]; } FFIWallet;
typedef struct FFIAccount { unsigned char _private[0]; } FFIAccount;
typedef struct FFIAccountCollection { unsigned char _private[0]; } FFIAccountCollection;
typedef struct FFIBLSAccount { unsigned char _private[0]; } FFIBLSAccount;
typedef struct FFIEdDSAAccount { unsigned char _private[0]; } FFIEdDSAAccount;
typedef struct FFIAddressPool { unsigned char _private[0]; } FFIAddressPool;
typedef struct FFIManagedAccountCollection { unsigned char _private[0]; } FFIManagedAccountCollection;
typedef struct FFIWalletManager { unsigned char _private[0]; } FFIWalletManager;
typedef struct FFIManagedAccount { unsigned char _private[0]; } FFIManagedAccount;
// Platform SDK opaque handles
typedef struct SDKHandle { unsigned char _private[0]; } SDKHandle;
typedef struct DataContractHandle { unsigned char _private[0]; } DataContractHandle;
typedef struct DocumentHandle { unsigned char _private[0]; } DocumentHandle;
typedef struct IdentityHandle { unsigned char _private[0]; } IdentityHandle;
typedef struct IdentityPublicKeyHandle { unsigned char _private[0]; } IdentityPublicKeyHandle;
typedef struct SignerHandle { unsigned char _private[0]; } SignerHandle;

// ============================================================================
// Key Wallet FFI Functions and Types
// ============================================================================

EOF
    
    # Extract Key Wallet FFI content
    # 1. Skip everything up to and including the last #include <stdlib.h>
    # 2. Skip header guards and pragma once
    # 3. Strip out all __cplusplus extern "C" blocks (we'll add them properly at the end)
    # 4. Fix ManagedWalletInfo reference to FFIManagedWalletInfo
    # 5. Include all content (including FFINetworks enum which Swift needs)
    # 6. Stop at the header guard closing
    awk '
        BEGIN { found_stdlib = 0; in_content = 0 }
        /^#include <stdlib\.h>/ { found_stdlib = 1; next }
        /^#include <stdint\.h>/ { next }
        /^#include <stddef\.h>/ { next }
        /^#include <stdbool\.h>/ { next }
        /^#include <stdarg\.h>/ { next }
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
        in_content { print }
    ' "$KEY_WALLET_HEADER_PATH" >> "$MERGED_HEADER"
    
    # Add separator for SPV FFI
    cat >> "$MERGED_HEADER" << 'EOF'

// ============================================================================
// Dash SPV FFI Functions and Types
// ============================================================================

EOF
    
    # Extract SPV FFI content
    # Skip duplicate types and problematic parts
    awk '
        BEGIN { skip = 0 }
        /^#include/ { next }
        /^#ifndef DASH_SPV_FFI_H/ { next }
        /^#define DASH_SPV_FFI_H/ { next }
        /^#pragma once/ { next }
        /^typedef struct CoreSDKHandle \{/ { skip = 1 }
        /^\} CoreSDKHandle;/ && skip { skip = 0; next }
        /^#ifdef __cplusplus$/ { next }
        /^namespace dash_spv_ffi \{/ { next }
        /^extern "C" \{$/ { next }
        /^\}  \/\/ namespace dash_spv_ffi$/ { next }
        /^}  \/\/ extern "C"$/ { next }
        /^#endif.*__cplusplus/ { next }
        /^#endif.*DASH_SPV_FFI_H/ { next }
        !skip { print }
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
// - FFINetwork enum from key-wallet-ffi (single network selection)
// - FFINetworks enum from key-wallet-ffi (bit flags for multiple networks)
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

# Build dash-spv-ffi from local rust-dashcore for device and simulator
RUST_DASHCORE_PATH="$PROJECT_ROOT/../rust-dashcore"
SPV_CRATE_PATH="$RUST_DASHCORE_PATH/dash-spv-ffi"
if [ -d "$SPV_CRATE_PATH" ]; then
  echo -e "${GREEN}Building dash-spv-ffi (local rust-dashcore)${NC}"
  pushd "$SPV_CRATE_PATH" >/dev/null
  if [ "$BUILD_ARCH" != "x86" ]; then
    cargo build --lib --target aarch64-apple-ios --release > /tmp/cargo_build_spv_device.log 2>&1 || { echo -e "${RED}✗ dash-spv-ffi device build failed${NC}"; cat /tmp/cargo_build_spv_device.log; exit 1; }
  fi
  if [ "$BUILD_ARCH" = "universal" ]; then
    cargo build --lib --target aarch64-apple-ios-sim --release > /tmp/cargo_build_spv_sim_arm.log 2>&1 || { echo -e "${RED}✗ dash-spv-ffi sim (arm64) build failed${NC}"; cat /tmp/cargo_build_spv_sim_arm.log; exit 1; }
    cargo build --lib --target x86_64-apple-ios --release > /tmp/cargo_build_spv_sim_x86.log 2>&1 || { echo -e "${RED}✗ dash-spv-ffi sim (x86_64) build failed${NC}"; cat /tmp/cargo_build_spv_sim_x86.log; exit 1; }
  else
    cargo build --lib --target aarch64-apple-ios-sim --release > /tmp/cargo_build_spv_sim_arm.log 2>&1 || { echo -e "${RED}✗ dash-spv-ffi sim (arm64) build failed${NC}"; cat /tmp/cargo_build_spv_sim_arm.log; exit 1; }
  fi
  popd >/dev/null
else
  echo -e "${YELLOW}⚠ Local rust-dashcore not found at $SPV_CRATE_PATH; SPV symbols must be provided by rs-sdk-ffi${NC}"
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
    # Merge with dash-spv-ffi device lib if available
    if [ -f "$SPV_CRATE_PATH/target/aarch64-apple-ios/release/libdash_spv_ffi.a" ]; then
      echo -e "${GREEN}Merging device libs (rs-sdk-ffi + dash-spv-ffi)${NC}"
      libtool -static -o "$OUTPUT_DIR/device/libDashSDKFFI_combined.a" \
        "$OUTPUT_DIR/device/librs_sdk_ffi.a" \
        "$SPV_CRATE_PATH/target/aarch64-apple-ios/release/libdash_spv_ffi.a"
      COMBINED_DEVICE_LIB=1
    fi
fi

# Create module map; include SDK, SPV, and KeyWallet headers
cat > "$OUTPUT_DIR/module.modulemap" << EOF
module DashSDKFFI {
    header "dash_sdk_ffi.h"
    export *
}

module DashSPVFFI {
    header "dash_spv_ffi.h"
    export *
}

module KeyWalletFFI {
    header "key_wallet_ffi.h"
    export *
}
EOF

# Prepare headers directory for XCFramework
HEADERS_DIR="$OUTPUT_DIR/headers"
mkdir -p "$HEADERS_DIR"
cp "$OUTPUT_DIR/dash_sdk_ffi.h" "$HEADERS_DIR/"
cp "$OUTPUT_DIR/module.modulemap" "$HEADERS_DIR/"

# Also copy raw SPV and KeyWallet headers (SPV now includes KeyWallet)
RUST_DASHCORE_PATH="$PROJECT_ROOT/../rust-dashcore"
KEY_WALLET_HEADER_PATH="$RUST_DASHCORE_PATH/key-wallet-ffi/include/key_wallet_ffi.h"
SPV_HEADER_PATH="$RUST_DASHCORE_PATH/dash-spv-ffi/include/dash_spv_ffi.h"

if [ -f "$SPV_HEADER_PATH" ]; then
  cp "$SPV_HEADER_PATH" "$HEADERS_DIR/"
else
  echo -e "${YELLOW}⚠ Missing SPV header at $SPV_HEADER_PATH${NC}"
fi

if [ -f "$KEY_WALLET_HEADER_PATH" ]; then
  cp "$KEY_WALLET_HEADER_PATH" "$HEADERS_DIR/"
else
  echo -e "${YELLOW}⚠ Missing KeyWallet header at $KEY_WALLET_HEADER_PATH${NC}"
fi

# Create XCFramework
echo -e "${GREEN}Creating XCFramework...${NC}"
rm -rf "$OUTPUT_DIR/$FRAMEWORK_NAME.xcframework"

# Build XCFramework command based on what was built
XCFRAMEWORK_CMD="xcodebuild -create-xcframework"

if [ "$BUILD_ARCH" != "x86" ] && [ -f "$OUTPUT_DIR/device/librs_sdk_ffi.a" ]; then
    if [ -n "${COMBINED_DEVICE_LIB:-}" ] && [ -f "$OUTPUT_DIR/device/libDashSDKFFI_combined.a" ]; then
      XCFRAMEWORK_CMD="$XCFRAMEWORK_CMD -library $OUTPUT_DIR/device/libDashSDKFFI_combined.a -headers $HEADERS_DIR"
    else
      XCFRAMEWORK_CMD="$XCFRAMEWORK_CMD -library $OUTPUT_DIR/device/librs_sdk_ffi.a -headers $HEADERS_DIR"
    fi
fi

if [ -f "$OUTPUT_DIR/simulator/librs_sdk_ffi.a" ]; then
    # Try to merge with SPV sim lib if it exists
    SIM_SPV_LIB=""
    if [ -f "$SPV_CRATE_PATH/target/aarch64-apple-ios-sim/release/libdash_spv_ffi.a" ]; then
      SIM_SPV_LIB="$SPV_CRATE_PATH/target/aarch64-apple-ios-sim/release/libdash_spv_ffi.a"
    elif [ -f "$SPV_CRATE_PATH/target/x86_64-apple-ios/release/libdash_spv_ffi.a" ]; then
      SIM_SPV_LIB="$SPV_CRATE_PATH/target/x86_64-apple-ios/release/libdash_spv_ffi.a"
    fi
    if [ -n "$SIM_SPV_LIB" ]; then
      echo -e "${GREEN}Merging simulator libs (rs-sdk-ffi + dash-spv-ffi)${NC}"
      libtool -static -o "$OUTPUT_DIR/simulator/libDashSDKFFI_combined.a" \
        "$OUTPUT_DIR/simulator/librs_sdk_ffi.a" \
        "$SIM_SPV_LIB"
      XCFRAMEWORK_CMD="$XCFRAMEWORK_CMD -library $OUTPUT_DIR/simulator/libDashSDKFFI_combined.a -headers $HEADERS_DIR"
    else
      XCFRAMEWORK_CMD="$XCFRAMEWORK_CMD -library $OUTPUT_DIR/simulator/librs_sdk_ffi.a -headers $HEADERS_DIR"
    fi
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
SWIFT_SDK_DIR="$PROJECT_ROOT/packages/swift-sdk"
if [ -d "$SWIFT_SDK_DIR" ]; then
    echo -e "\n${GREEN}Copying XCFramework to Swift SDK...${NC}"
    rm -rf "$SWIFT_SDK_DIR/$FRAMEWORK_NAME.xcframework"
    cp -R "$OUTPUT_DIR/$FRAMEWORK_NAME.xcframework" "$SWIFT_SDK_DIR/"
    echo -e "${GREEN}✓ XCFramework copied to ${YELLOW}$SWIFT_SDK_DIR/$FRAMEWORK_NAME.xcframework${NC}"

    # Best-effort: resolve package dependencies and clean stale references in Xcode project
    if command -v xcodebuild >/dev/null 2>&1; then
        if [ -d "$SWIFT_SDK_DIR/SwiftExampleApp/SwiftExampleApp.xcodeproj" ]; then
            echo -e "\n${GREEN}Resolving Swift package dependencies for SwiftExampleApp...${NC}"
            (cd "$SWIFT_SDK_DIR" && xcodebuild -project SwiftExampleApp/SwiftExampleApp.xcodeproj -resolvePackageDependencies >/tmp/xcode_resolve.log 2>&1 || true)

            # Optional clean of DerivedData for a fresh build
            if [ "${CLEAN_DERIVED_DATA:-0}" = "1" ]; then
                echo -e "${YELLOW}Cleaning DerivedData for SwiftExampleApp (CLEAN_DERIVED_DATA=1)...${NC}"
                rm -rf "$HOME/Library/Developer/Xcode/DerivedData"/SwiftExampleApp-* 2>/dev/null || true
            fi

            # Validate headers and module visibility
            echo -e "${GREEN}Validating DashSDKFFI.xcframework presence in SwiftDashSDK Package.swift...${NC}"
            if ! grep -q "DashSDKFFI.xcframework" "$SWIFT_SDK_DIR/Package.swift"; then
                echo -e "${YELLOW}⚠ DashSDKFFI.xcframework not referenced in Package.swift. Please update the binaryTarget path.${NC}"
            fi
        fi
    else
        echo -e "${YELLOW}xcodebuild not found; skipping Xcode project dependency resolution.${NC}"
    fi
fi
