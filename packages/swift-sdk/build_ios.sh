#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
REPO_ROOT="$( cd "$SCRIPT_DIR/../.." && pwd )"

echo "=== SwiftDashSDK iOS Build (Unified) ==="

echo "1) Building Rust FFI (rs-sdk-ffi)"
pushd "$REPO_ROOT/packages/rs-sdk-ffi" >/dev/null
if [[ ! -x ./build_ios.sh ]]; then
  echo "❌ Missing rs-sdk-ffi/build_ios.sh"
  exit 1
fi
./build_ios.sh
popd >/dev/null

# Expected output from rs-sdk-ffi
UNIFIED_DIR="$REPO_ROOT/packages/rs-sdk-ffi/build/DashUnifiedSDK.xcframework"
SDKFFI_DIR="$REPO_ROOT/packages/rs-sdk-ffi/build/DashSDKFFI.xcframework"
if [[ -d "$UNIFIED_DIR" ]]; then
  SRC_XCFRAMEWORK_DIR="$UNIFIED_DIR"
elif [[ -d "$SDKFFI_DIR" ]]; then
  SRC_XCFRAMEWORK_DIR="$SDKFFI_DIR"
else
  echo "❌ rs-sdk-ffi build did not produce an XCFramework (expected DashUnifiedSDK.xcframework or DashSDKFFI.xcframework)"
  exit 1
fi

echo "2) Installing XCFramework into Swift package"
DEST_XCFRAMEWORK_DIR="$SCRIPT_DIR/DashSDKFFI.xcframework"
rm -rf "$DEST_XCFRAMEWORK_DIR"
cp -R "$SRC_XCFRAMEWORK_DIR" "$DEST_XCFRAMEWORK_DIR"

# Verify required SPV symbols are present in the binary
mapfile -t SIM_LIBS < <(find "$DEST_XCFRAMEWORK_DIR" -maxdepth 2 -type f -name "*.a" -path "*simulator*" 2>/dev/null)
if [[ ${#SIM_LIBS[@]} -eq 0 ]]; then
  echo "❌ Missing simulator static libraries inside $DEST_XCFRAMEWORK_DIR (searched for *.a under *simulator* slices)"
  exit 1
fi
echo "   - Verifying required SPV symbols are present in XCFramework libs"
# Prefer ripgrep if available; fall back to grep for portability
# Avoid -q with pipefail, which can cause nm to SIGPIPE and fail the check.
if command -v rg >/dev/null 2>&1; then
  SEARCH_CMD=(rg -F)    # fixed-string match
else
  SEARCH_CMD=(grep -F)  # fixed-string match
fi

CHECK_OK=1
check_symbol() {
  local symbol="$1"
  for lib in "${SIM_LIBS[@]}"; do
    if nm -gU "$lib" 2>/dev/null | "${SEARCH_CMD[@]}" "_${symbol}" >/dev/null; then
      return 0
    fi
  done
  return 1
}

if ! check_symbol "dash_spv_ffi_config_add_peer"; then
  echo "❌ Missing symbol: dash_spv_ffi_config_add_peer across simulator libs: ${SIM_LIBS[*]}"
  CHECK_OK=0
fi

if ! check_symbol "dash_spv_ffi_config_set_restrict_to_configured_peers"; then
  echo "❌ Missing symbol: dash_spv_ffi_config_set_restrict_to_configured_peers across simulator libs: ${SIM_LIBS[*]}"
  CHECK_OK=0
fi

if [[ $CHECK_OK -ne 1 ]]; then
  echo "   Please ensure dash-spv-ffi exports these symbols and is included in the XCFramework."
  exit 1
fi

echo "3) Verifying Swift builds (if Xcode available)"
if command -v xcodebuild >/dev/null 2>&1; then
  set +e
  xcodebuild -project "$SCRIPT_DIR/SwiftExampleApp/SwiftExampleApp.xcodeproj" \
             -scheme SwiftExampleApp \
             -sdk iphonesimulator \
             -destination 'platform=iOS Simulator,name=iPhone 16' \
             -quiet build
  XC_STATUS=$?
  set -e
  if [[ $XC_STATUS -ne 0 ]]; then
    echo "❌ Xcode build failed"
    exit $XC_STATUS
  fi
  echo "✅ Xcode build succeeded"
else
  echo "⚠️  xcodebuild not found; skipping local build. Run this script on a macOS host with Xcode to fully verify."
fi

echo "✅ Done. XCFramework installed at $DEST_XCFRAMEWORK_DIR"
