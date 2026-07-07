#!/bin/bash
set -euo pipefail

IPHONEOS_DEPLOYMENT_TARGET="${IPHONEOS_DEPLOYMENT_TARGET:-17.0}"
IPHONESIMULATOR_DEPLOYMENT_TARGET="${IPHONESIMULATOR_DEPLOYMENT_TARGET:-17.0}"
MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-15.0}"
export IPHONEOS_DEPLOYMENT_TARGET
export IPHONESIMULATOR_DEPLOYMENT_TARGET
export MACOSX_DEPLOYMENT_TARGET

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
PROFILE="dev"

# Crates whose cbindgen-generated headers ship in the unified framework.
# Order matters: earlier headers define types referenced by later ones.
INCLUDED_CRATES=(
  dash-network
  key-wallet-ffi
  rs-sdk-ffi
  platform-wallet-ffi
)

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
  echo "Usage: $0 --target <ios|sim|mac> [--profile <dev|release>]"
  echo ""
  echo "Targets:"
  echo "  ios         -> iPhone device"
  echo "  sim         -> auto-detected iOS simulator"
  echo "  mac         -> Apple Silicon Mac"
  echo "  all         -> all targets"
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

# Map the requested base profile (dev|release) onto its iOS-tuned custom
# profile by appending "-ios" (dev-ios / release-ios)
PROFILE="${PROFILE}-ios"
OUTPUT_DIR="$PROFILE"

log_info "Package: $PACKAGE"
log_info "Profile: $PROFILE"

# -------------------------------
# Build commands
# -------------------------------

inject_modulemap() {
  local HEADERS_DIR="$1"
  local dir name keep c

  for dir in "$HEADERS_DIR"/*/; do
    [[ -d "$dir" ]] || continue
    name=$(basename "$dir")
    keep=0
    for c in "${INCLUDED_CRATES[@]}"; do
      if [[ "$c" == "$name" ]]; then
        keep=1
        break
      fi
    done
    if (( ! keep )); then
      rm -rf "$dir"
      log_info "  → pruned orphan header dir: $name"
    fi
  done

  for c in "${INCLUDED_CRATES[@]}"; do
    if [[ ! -f "$HEADERS_DIR/$c/$c.h" ]]; then
      log_error "Missing header: $HEADERS_DIR/$c/$c.h"
      log_error "  → ensure '$c' is a dependency of $PACKAGE"
      exit 1
    fi
  done

  {
    printf '#ifndef DASHSDKFFI_H\n'
    printf '#define DASHSDKFFI_H\n\n'
    for c in "${INCLUDED_CRATES[@]}"; do
      printf '#include "%s/%s.h"\n' "$c" "$c"
    done
    printf '\n#endif\n'
  } > "$HEADERS_DIR/DashSDKFFI.h"

  cat > "$HEADERS_DIR/module.modulemap" << 'EOF'
module DashSDKFFI {
    umbrella header "DashSDKFFI.h"
    export *
}
EOF
  log_info "  → module.modulemap + umbrella header injected in $HEADERS_DIR"

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

# Shielded (Orchard / ZK) support is compiled in by default. The
# `shielded` Cargo feature is opt-in at the crate level so non-iOS
# consumers don't pay for the heavy crypto deps, but the iOS
# framework ships everything — keep `--features shielded` here so
# the bundled SDK exposes the platform-wallet shielded FFI.
CARGO_FEATURES="shielded"

if [ "$PROFILE" = "dev-ios" ]; then
  CARGO_FEATURES="$CARGO_FEATURES tokio-metrics"
  log_info "  → tokio-metrics enabled (dev profile)"
fi

# iOS device
if $BUILD_IOS; then
  IOS_TARGET="aarch64-apple-ios"
  log_info "Building iOS device ($IOS_TARGET)..."
  cargo build -p "$PACKAGE" --profile "$PROFILE" --target "$IOS_TARGET" --features "$CARGO_FEATURES"
  IOS_LIB="$TARGET_DIR/$IOS_TARGET/$OUTPUT_DIR/librs_unified_sdk_ffi.a"
  IOS_HEADERS="$TARGET_DIR/$IOS_TARGET/$OUTPUT_DIR/include"
  inject_modulemap "$IOS_HEADERS"
fi

# iOS simulator
if $BUILD_SIM; then
  SIM_TARGET="aarch64-apple-ios-sim"
  log_info "Building iOS simulator ($SIM_TARGET)..."
  cargo build -p "$PACKAGE" --profile "$PROFILE" --target "$SIM_TARGET" --features "$CARGO_FEATURES"
  SIM_LIB="$TARGET_DIR/$SIM_TARGET/$OUTPUT_DIR/librs_unified_sdk_ffi.a"
  SIM_HEADERS="$TARGET_DIR/$SIM_TARGET/$OUTPUT_DIR/include"
  inject_modulemap "$SIM_HEADERS"
fi

# macOS
if $BUILD_MAC; then
  MAC_TARGET="aarch64-apple-darwin"
  log_info "Building macOS ($MAC_TARGET)..."
  cargo build -p "$PACKAGE" --profile "$PROFILE" --target "$MAC_TARGET" --features "$CARGO_FEATURES"
  MAC_LIB="$TARGET_DIR/$MAC_TARGET/$OUTPUT_DIR/librs_unified_sdk_ffi.a"
  MAC_HEADERS="$TARGET_DIR/$MAC_TARGET/$OUTPUT_DIR/include"
  inject_modulemap "$MAC_HEADERS"
fi

# -------------------------------
# Create XCFramework
# -------------------------------
log_info "Generating DashSDKFFI.xcframework "
rm -rf "$XCFRAMEWORK"

xcodebuild -create-xcframework \
  ${IOS_LIB:+-library "$IOS_LIB" -headers "$IOS_HEADERS"} \
  ${MAC_LIB:+-library "$MAC_LIB" -headers "$MAC_HEADERS"} \
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

OTHER_SWIFT_FLAGS="-warnings-as-errors"
SWIFT_TREAT_WARNINGS_AS_ERRORS=YES
SWIFT_SUPPRESS_WARNINGS=NO

if [ "${SKIP_EXAMPLE_APP_BUILD:-0}" = "1" ]; then
    log_info "SKIP_EXAMPLE_APP_BUILD=1 — skipping SwiftExampleApp verification build"
elif command -v xcodebuild >/dev/null 2>&1; then
    set +e
    xcodebuild -project "$SWIFT_PROJECT" \
               -scheme "$SWIFT_SCHEME" \
               -sdk iphonesimulator \
               -destination "$SWIFT_DESTINATION" \
               EXCLUDED_ARCHS="$EXCLUDED_ARCHS" \
               OTHER_SWIFT_FLAGS="$OTHER_SWIFT_FLAGS" \
               SWIFT_TREAT_WARNINGS_AS_ERRORS=$SWIFT_TREAT_WARNINGS_AS_ERRORS \
               SWIFT_SUPPRESS_WARNINGS=$SWIFT_SUPPRESS_WARNINGS \
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
