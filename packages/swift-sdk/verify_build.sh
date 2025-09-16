#!/usr/bin/env bash
set -euo pipefail

echo "=== SwiftDashSDK Build Verification (iOS) ==="

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

echo "Step 1: Build/Install XCFramework via unified script"
"$SCRIPT_DIR/build_ios.sh"

echo
echo "Step 2: Verify Xcode build (if available)"
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
  echo "✅ Example app builds for iOS Simulator"
else
  echo "⚠️  xcodebuild not found; skipping local build. Run this on macOS with Xcode to verify."
fi

echo
echo "✅ Verification complete"
