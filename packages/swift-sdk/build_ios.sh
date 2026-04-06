#!/bin/bash
set -euo pipefail

IPHONEOS_DEPLOYMENT_TARGET="${IPHONEOS_DEPLOYMENT_TARGET:-17.0}"
IPHONESIMULATOR_DEPLOYMENT_TARGET="${IPHONESIMULATOR_DEPLOYMENT_TARGET:-17.0}"
export IPHONEOS_DEPLOYMENT_TARGET
export IPHONESIMULATOR_DEPLOYMENT_TARGET

# -------------------------------
# Colors
# -------------------------------
RED="\033[0;31m"
GREEN="\033[0;32m"
NC="\033[0m"

# -------------------------------
# Paths & package
# -------------------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$SCRIPT_DIR/../../"
TARGET_DIR="$ROOT_DIR/target"
PACKAGE="rs-unified-sdk-ffi"
XCFRAMEWORK="$SCRIPT_DIR/DashSDKFFI.xcframework"
PROFILE="dev" # Rust doesn't allow us to use "debug" for some reason, the profile name internally is dev

# -------------------------------
# Flags
# -------------------------------
BUILD_IOS=false
BUILD_SIM=false
BUILD_MAC=false
BUILD_INTEL_MAC=false
CLEAN=false

log_info() { echo -e "${GREEN}$1${NC}"; }
log_error() { echo -e "${RED}$1${NC}"; }

# -------------------------------
# Help
# -------------------------------
show_help() {
  echo "Usage: $0 --target <ios|sim|mac|intel-mac> [--profile <dev|release>]"
  echo ""
  echo "Targets:"
  echo "  ios         -> iPhone device"
  echo "  sim         -> auto-detected iOS simulator"
  echo "  mac         -> Apple Silicon Mac"
  echo "  intel-mac   -> Intel Mac"
  echo "  all         -> all targets except Intel Mac"
  echo ""
  echo "Profile:"
  echo "  dev (default)"
  echo "  release"
  echo ""
  echo "Examples:"
  echo "  $0 --target sim --profile release"
  exit 1
}

# -------------------------------
# Detect simulator target
# -------------------------------
detect_sim_target() {
  ARCH=$(uname -m)
  if [[ "$ARCH" == "arm64" ]]; then
    echo "aarch64-apple-ios-sim"
  else
    echo "x86_64-apple-ios"
  fi
}

# -------------------------------
# Parse flags
# -------------------------------
while [[ $# -gt 0 ]]; do
  case $1 in
    --clean)
      CLEAN=true
      shift
      ;;
    --target)
      case $2 in
        ios) BUILD_IOS=true ;;
        sim) BUILD_SIM=true ;;
        mac) BUILD_MAC=true ;;
        intel-mac) BUILD_INTEL_MAC=true ;;
        all) BUILD_IOS=true; BUILD_SIM=true; BUILD_MAC=true ;;
        *) log_error "Unknown target $2"; show_help ;;
      esac
      shift 2
      ;;
    --profile)
      PROFILE="$2"

      if [ ! "$PROFILE" = "dev" ] && [ ! "$PROFILE" = "release" ]; then
        log_error "Unknown profile $2"; show_help
      fi

      shift 2
      ;;
    --help)
      show_help
      ;;
    *)
      log_error "Unknown flag $1"; show_help ;;
  esac
done

if $CLEAN; then
  log_info "Cleaning all build artifacts..."
  rm -rf "$TARGET_DIR"
  rm -rf "$XCFRAMEWORK"
fi

# -------------------------------
# Validation
# -------------------------------
if ! $BUILD_IOS && ! $BUILD_SIM && ! $BUILD_MAC && ! $BUILD_INTEL_MAC; then
  log_error "You must specify at least one --target"
  show_help
fi

log_info "Package: $PACKAGE"
log_info "Profile: $PROFILE"

# Rust writes the "dev" profile output to the "debug" directory
if [ "$PROFILE" = "dev" ]; then
  OUTPUT_DIR="debug"
else
  OUTPUT_DIR="$PROFILE"
fi

# -------------------------------
# Build commands
# -------------------------------

inject_modulemap() {
  local HEADERS_DIR="$1"

  # Create umbrella header that includes all FFI headers in dependency order
  cat > "$HEADERS_DIR/DashSDKFFI.h" << 'EOF'
#ifndef DASHSDKFFI_H
#define DASHSDKFFI_H

// key-wallet-ffi defines FFINetwork used by dash-spv-ffi, so must come first
#include "key-wallet-ffi/key-wallet-ffi.h"
#include "dash-spv-ffi/dash-spv-ffi.h"
#include "rs-sdk-ffi/rs-sdk-ffi.h"

#endif
EOF

  cat > "$HEADERS_DIR/module.modulemap" << 'EOF'
module DashSDKFFI {
    umbrella header "DashSDKFFI.h"
    export *
}
EOF
  log_info "  → module.modulemap + umbrella header injected in $HEADERS_DIR"

  # TODO(build_ios): Quick fix — upstream headers from rust-dashcore emit FFIAssetLockFundingType
  # with bare enumerator names (IDENTITY_REGISTRATION, IDENTITY_TOP_UP, etc.) that collide with
  # FFIAccountType, which is invalid C (enum constants share the global namespace).
  # The proper fix belongs in rust-dashcore's cbindgen config (prefix or namespace the variants).
  # Until that fix lands, we strip the enum typedef and replace the type with uint32_t.
  for h in "$HEADERS_DIR"/*/*.h; do
    perl -i -0777 -pe 's{/\*\*?\s*\n\s*The type of funding account.*?\n\s*\*/\s*\ntypedef enum \{.*?\} FFIAssetLockFundingType;\n}{}s' "$h"
    sed -i '' 's/FFIAssetLockFundingType/uint32_t/g' "$h"
  done

  # Give opaque struct forward declarations a body so Swift can use UnsafeMutablePointer<T>.
  # Skip types that already have a full definition in another header to avoid redefinition.
  local defined
  defined=$(grep -oh 'typedef struct [A-Za-z_][A-Za-z_0-9]* {' "$HEADERS_DIR"/*/*.h 2>/dev/null \
    | sed 's/typedef struct \([^ ]*\) {/\1/' | sort -u | paste -sd'|' - || true)
  for h in "$HEADERS_DIR"/*/*.h; do
    if [ -n "$defined" ]; then
      perl -i -pe "s/^typedef struct (\w+) \1;\$/
        my \$n=\$1; \$n=~m{^($defined)\$} ? \$_ : \"typedef struct \$n { uint8_t _opaque; } \$n;\n\"/e" "$h"
    else
      perl -i -pe 's/^typedef struct (\w+) \1;$/typedef struct $1 { uint8_t _opaque; } $1;/' "$h"
    fi
  done
}

# iOS device
if $BUILD_IOS; then
  IOS_TARGET="aarch64-apple-ios"
  log_info "Building iOS device ($IOS_TARGET)..."
  cargo build -p "$PACKAGE" --profile "$PROFILE" --target "$IOS_TARGET"
  IOS_LIB="$TARGET_DIR/$IOS_TARGET/$OUTPUT_DIR/librs_unified_sdk_ffi.a"
  IOS_HEADERS="$TARGET_DIR/$IOS_TARGET/$OUTPUT_DIR/include"
  inject_modulemap "$IOS_HEADERS"
fi

# iOS simulator
if $BUILD_SIM; then
  SIM_TARGET=$(detect_sim_target)
  log_info "Building iOS simulator ($SIM_TARGET)..."
  cargo build -p "$PACKAGE" --profile "$PROFILE" --target "$SIM_TARGET"
  SIM_LIB="$TARGET_DIR/$SIM_TARGET/$OUTPUT_DIR/librs_unified_sdk_ffi.a"
  SIM_HEADERS="$TARGET_DIR/$SIM_TARGET/$OUTPUT_DIR/include"
  inject_modulemap "$SIM_HEADERS"
fi

# macOS
if $BUILD_MAC; then
  MAC_TARGET="aarch64-apple-darwin"
  log_info "Building macOS ($MAC_TARGET)..."
  cargo build -p "$PACKAGE" --profile "$PROFILE" --target "$MAC_TARGET"
  MAC_LIB="$TARGET_DIR/$MAC_TARGET/$OUTPUT_DIR/librs_unified_sdk_ffi.a"
  MAC_HEADERS="$TARGET_DIR/$MAC_TARGET/$OUTPUT_DIR/include"
  inject_modulemap "$MAC_HEADERS"
fi

# Intel Mac
if $BUILD_INTEL_MAC; then
  INTEL_MAC_TARGET="x86_64-apple-darwin"
  log_info "Building Intel macOS ($INTEL_MAC_TARGET)..."
  cargo build -p "$PACKAGE" --profile "$PROFILE" --target "$INTEL_MAC_TARGET"
  INTEL_MAC_LIB="$TARGET_DIR/$INTEL_MAC_TARGET/$OUTPUT_DIR/librs_unified_sdk_ffi.a"
  INTEL_MAC_HEADERS="$TARGET_DIR/$INTEL_MAC_TARGET/$OUTPUT_DIR/include"
  inject_modulemap "$INTEL_MAC_HEADERS"
fi

# -------------------------------
# Create XCFramework
# -------------------------------
log_info "Generating DashSDKFFI.xcframework "
rm -rf "$XCFRAMEWORK"

xcodebuild -create-xcframework \
  ${IOS_LIB:+-library "$IOS_LIB" -headers "$IOS_HEADERS"} \
  ${MAC_LIB:+-library "$MAC_LIB" -headers "$MAC_HEADERS"} \
  ${INTEL_MAC_LIB:+-library "$INTEL_MAC_LIB" -headers "$INTEL_MAC_HEADERS"} \
  ${SIM_LIB:+-library "$SIM_LIB" -headers "$SIM_HEADERS"} \
  -output "$XCFRAMEWORK"

log_info "XCFramework created: $XCFRAMEWORK"

# -------------------------------
# Build Swift SDK
# -------------------------------
SWIFT_PROJECT="$SCRIPT_DIR/SwiftExampleApp/SwiftExampleApp.xcodeproj"
SWIFT_SCHEME="SwiftExampleApp"
SWIFT_DESTINATION="generic/platform=iOS Simulator"
EXCLUDED_ARCHS="x86_64"

if command -v xcodebuild >/dev/null 2>&1; then
    set +e
    xcodebuild -project "$SWIFT_PROJECT" \
               -scheme "$SWIFT_SCHEME" \
               -sdk iphonesimulator \
               -destination "$SWIFT_DESTINATION" \
               EXCLUDED_ARCHS="$EXCLUDED_ARCHS" \
               build
    XC_STATUS=$?
    set -e
    if [[ $XC_STATUS -ne 0 ]]; then
        log_error "Swift/Xcode build failed"
        exit $XC_STATUS
    fi
    log_info "Swift/Xcode build succeeded"
else
    echo "xcodebuild not found; skipping Swift verification."
fi
