#!/bin/bash
set -euo pipefail

# This script  builds the sdk using `build_ios.sh` for all targets,
# then runs the DashSDKFFI tests, and finally runs the
# ExampleApp tests on a selected iOS Simulator

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR" || exit 1

# Provision an unlocked keychain for CI.
#
# The `WalletStorage` tests write to the login keychain via `SecItemAdd`.
# On a headless self-hosted runner (agent connected over SSH, no Aqua GUI
# session) that call can fail with `errAuthorizationInternal` (-60008)
# because the Security authorization subsystem has no session to service
# the request, which surfaces as `keychainError(-60008)` and fails the
# suite intermittently (it only passes when the runner happens to have a
# live GUI session).
#
# Create a dedicated, explicitly-unlocked, no-auto-lock keychain and make
# it the default for the duration of the run so `SecItemAdd` targets a
# keychain that needs no interactive authorization. Gated to CI so it
# never touches a developer's login keychain, and the previous default is
# restored on exit so the persistent runner is left unchanged.
if [ -n "${CI:-}${GITHUB_ACTIONS:-}" ]; then
  CI_KEYCHAIN="$HOME/Library/Keychains/dash-ci-tests.keychain-db"
  PREV_DEFAULT_KEYCHAIN="$(security default-keychain -d user | sed -E 's/^[[:space:]]*"?//;s/"?[[:space:]]*$//')"
  restore_default_keychain() {
    if [ -n "${PREV_DEFAULT_KEYCHAIN:-}" ] && [ -e "$PREV_DEFAULT_KEYCHAIN" ]; then
      security default-keychain -s "$PREV_DEFAULT_KEYCHAIN" || true
    fi
  }
  trap restore_default_keychain EXIT
  security create-keychain -p "" "$CI_KEYCHAIN" 2>/dev/null || true
  security set-keychain-settings "$CI_KEYCHAIN"   # no auto-lock timeout
  security unlock-keychain -p "" "$CI_KEYCHAIN"
  security default-keychain -s "$CI_KEYCHAIN"
fi

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

bash build_ios.sh --target all --profile dev

swift package clean
swift build
swift test

xcodebuild test \
  -project SwiftExampleApp/SwiftExampleApp.xcodeproj \
  -scheme SwiftExampleApp \
  -skip-testing:SwiftExampleAppUITests \
  -destination "platform=iOS Simulator,name=$SIM_NAME"
