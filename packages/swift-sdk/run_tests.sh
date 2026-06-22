#!/bin/bash
set -euo pipefail

# This script  builds the sdk using `build_ios.sh` for all targets,
# then runs the DashSDKFFI tests, and finally runs the
# ExampleApp tests on a selected iOS Simulator

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR" || exit 1

# Pick a concrete iOS Simulator for the `xcodebuild test` run. A name
# can be pinned via `SIM_NAME`; otherwise grab the first available
SIM_NAME="${SIM_NAME:-}"
if [ -z "$SIM_NAME" ]; then
  SIM_NAME="$(xcrun simctl list devices available \
    | grep -oE 'iPhone [0-9][^(]*' | head -1 | sed 's/ *$//')"
fi
if [ -z "$SIM_NAME" ]; then
  echo "No available iPhone simulator found for the simulator test run" >&2
  exit 1
fi

bash build_swift.sh --target all --profile dev

swift package clean
swift build
swift test

xcodebuild test \
  -project SwiftExampleApp/SwiftExampleApp.xcodeproj \
  -scheme SwiftExampleApp \
  -skip-testing:SwiftExampleAppUITests \
  -destination "platform=iOS Simulator,name=$SIM_NAME"
