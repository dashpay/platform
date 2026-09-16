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
YELLOW="\033[0;33m"
NC="\033[0m"

# -------------------------------
# Paths & package
# -------------------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$SCRIPT_DIR/../../"
TARGET_DIR="$ROOT_DIR/target"
PACKAGE="rs-unified-sdk-ffi"
XCFRAMEWORK="$SCRIPT_DIR/DashSDKFFI.xcframework"
PROFILE="release"
PRUNE_CARGO_TARGETS="${PRUNE_CARGO_TARGETS:-0}"
STAGING_DIR=""

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
log_warn() { echo -e "${YELLOW}$1${NC}"; }

cleanup_staging_dir() {
  if [ -n "$STAGING_DIR" ]; then
    rm -rf "$STAGING_DIR"
  fi
}

stage_target_artifacts() {
  local target="$1"
  local library="$2"
  local headers="$3"
  local target_staging_dir="$STAGING_DIR/$target"

  mkdir -p "$target_staging_dir"
  cp "$library" "$target_staging_dir/"
  cp -R "$headers" "$target_staging_dir/include"

  STAGED_LIB="$target_staging_dir/$(basename "$library")"
  STAGED_HEADERS="$target_staging_dir/include"

  # The final static library and generated headers are all xcodebuild needs.
  # Release the much larger per-architecture dependency tree before building
  # the next target so persistent CI runners cannot exhaust their disk.
  rm -rf "${TARGET_DIR:?}/${target:?}"
}

# -------------------------------
# Help
# -------------------------------
show_help() {
  echo "Usage: $0 --target <ios|sim|mac|all|tests> [--profile <dev|release>]"
  echo ""
  echo "Targets:"
  echo "  ios         -> iPhone device"
  echo "  sim         -> auto-detected iOS simulator"
  echo "  mac         -> Apple Silicon Mac"
  echo "  all         -> all targets"
  echo "  tests       -> targets needed by run_tests.sh (sim + mac)"
  echo ""
  echo "Profile:"
  echo "  release (default) -> ships: optimized, no debug assertions"
  echo "  dev               -> local iteration: fast to build, debug"
  echo "                       assertions ON, tokio-metrics ON. A dev"
  echo "                       build turns every internal invariant into"
  echo "                       an abort() and must never be distributed."
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
        tests) BUILD_SIM=true; BUILD_MAC=true ;;
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
  rm -rf "${TARGET_DIR:?}"
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

# `dev-ios` inherits Cargo's `dev` defaults, so debug assertions are live —
# and it pairs them with `panic = "abort"`. Any debug_assert! in dash-spv,
# key-wallet or platform-wallet therefore terminates the host process rather
# than degrading. That is what a dev build is for locally, and exactly why one
# must not reach testers; a shipped dev build has already crashed a TestFlight
# release this way. Loud on purpose, since the failure is invisible until a
# device aborts weeks later.
if [ "$PROFILE" = "dev-ios" ]; then
  log_warn "dev profile: debug assertions ON (panic = abort) — NOT distributable"
  log_warn "  build shipping artifacts with: $0 --target all --profile release"
fi

if [ "$PRUNE_CARGO_TARGETS" = "1" ]; then
  STAGING_DIR="$(mktemp -d "${TMPDIR:-/tmp}/dash-sdk-ffi.XXXXXX")"
  trap cleanup_staging_dir EXIT

  # Persistent self-hosted runners may contain incomplete or obsolete builds
  # from an earlier job. Start the bounded build with only Cargo's shared host
  # cache, then prune each Apple target after staging its final artifacts.
  rm -rf \
    "${TARGET_DIR:?}/aarch64-apple-ios" \
    "${TARGET_DIR:?}/aarch64-apple-ios-sim" \
    "${TARGET_DIR:?}/aarch64-apple-darwin"
fi

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
  if [ "$PRUNE_CARGO_TARGETS" = "1" ]; then
    stage_target_artifacts "$IOS_TARGET" "$IOS_LIB" "$IOS_HEADERS"
    IOS_LIB="$STAGED_LIB"
    IOS_HEADERS="$STAGED_HEADERS"
  fi
fi

# iOS simulator
if $BUILD_SIM; then
  SIM_TARGET="aarch64-apple-ios-sim"
  log_info "Building iOS simulator ($SIM_TARGET)..."
  cargo build -p "$PACKAGE" --profile "$PROFILE" --target "$SIM_TARGET" --features "$CARGO_FEATURES"
  SIM_LIB="$TARGET_DIR/$SIM_TARGET/$OUTPUT_DIR/librs_unified_sdk_ffi.a"
  SIM_HEADERS="$TARGET_DIR/$SIM_TARGET/$OUTPUT_DIR/include"
  inject_modulemap "$SIM_HEADERS"
  if [ "$PRUNE_CARGO_TARGETS" = "1" ]; then
    stage_target_artifacts "$SIM_TARGET" "$SIM_LIB" "$SIM_HEADERS"
    SIM_LIB="$STAGED_LIB"
    SIM_HEADERS="$STAGED_HEADERS"
  fi
fi

# macOS
if $BUILD_MAC; then
  MAC_TARGET="aarch64-apple-darwin"
  log_info "Building macOS ($MAC_TARGET)..."
  cargo build -p "$PACKAGE" --profile "$PROFILE" --target "$MAC_TARGET" --features "$CARGO_FEATURES"
  MAC_LIB="$TARGET_DIR/$MAC_TARGET/$OUTPUT_DIR/librs_unified_sdk_ffi.a"
  MAC_HEADERS="$TARGET_DIR/$MAC_TARGET/$OUTPUT_DIR/include"
  inject_modulemap "$MAC_HEADERS"
  if [ "$PRUNE_CARGO_TARGETS" = "1" ]; then
    stage_target_artifacts "$MAC_TARGET" "$MAC_LIB" "$MAC_HEADERS"
    MAC_LIB="$STAGED_LIB"
    MAC_HEADERS="$STAGED_HEADERS"
  fi
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
