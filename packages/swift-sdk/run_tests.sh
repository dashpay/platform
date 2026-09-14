#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR" || exit 1

# Provision an unlocked keychain for CI.
#
# The `WalletStorage` tests write to the user default keychain via `SecItemAdd`.
# On a headless self-hosted runner (agent connected over SSH, no Aqua GUI
# session) that call can fail with `errAuthorizationInternal` (-60008)
# because the Security authorization subsystem has no session to service
# the request, which surfaces as `keychainError(-60008)` and fails the
# suite intermittently (it only passes when the runner happens to have a
# live GUI session).
# Reusing a fixed CI keychain can also preserve stale permissions between
# jobs and surface as `errSecWrPerm` (-61).
#
# Create a dedicated, explicitly-unlocked keychain and make it the user
# default for the duration of the run so `SecItemAdd` targets a keychain
# that needs no interactive authorization. Add it to the user search list
# so later reads and deletes find the same items. Gated to CI so it never
# touches a developer's keychain configuration; the previous default and
# search list are restored on exit.
if [ -n "${CI:-}${GITHUB_ACTIONS:-}" ]; then
  PREV_DEFAULT_KEYCHAIN="$(security default-keychain -d user | sed -E 's/^[[:space:]]*"?//;s/"?[[:space:]]*$//')"
  PREV_USER_KEYCHAINS_OUTPUT="$(security list-keychains -d user)"
  PREV_USER_KEYCHAINS=()
  while IFS= read -r keychain_path; do
    keychain_path="$(printf '%s\n' "$keychain_path" | sed -E 's/^[[:space:]]*"?//;s/"?[[:space:]]*$//')"
    if [ -n "$keychain_path" ]; then
      PREV_USER_KEYCHAINS+=("$keychain_path")
    fi
  done <<< "$PREV_USER_KEYCHAINS_OUTPUT"
  if [ "${#PREV_USER_KEYCHAINS[@]}" -eq 0 ]; then
    echo "No user keychain search list is configured" >&2
    exit 1
  fi

  CI_KEYCHAIN_DIR="$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/dash-ci-keychain.XXXXXX")"
  CI_KEYCHAIN="$CI_KEYCHAIN_DIR/tests.keychain-db"
  CI_KEYCHAIN_MAY_EXIST=0
  CI_DEFAULT_MAY_HAVE_CHANGED=0
  CI_SEARCH_LIST_MAY_HAVE_CHANGED=0

  cleanup_ci_keychain() {
    original_status=$?
    cleanup_status=0
    trap - EXIT

    if [ "${CI_DEFAULT_MAY_HAVE_CHANGED:-0}" -eq 1 ]; then
      if ! security default-keychain -d user -s "$PREV_DEFAULT_KEYCHAIN"; then
        cleanup_status=1
      fi
    fi
    if [ "${CI_SEARCH_LIST_MAY_HAVE_CHANGED:-0}" -eq 1 ]; then
      if ! security list-keychains -d user -s "${PREV_USER_KEYCHAINS[@]}"; then
        cleanup_status=1
      fi
    fi
    if [ "${CI_KEYCHAIN_MAY_EXIST:-0}" -eq 1 ]; then
      if ! security delete-keychain "$CI_KEYCHAIN"; then
        cleanup_status=1
      fi
    fi
    if [ -d "${CI_KEYCHAIN_DIR:-}" ]; then
      if ! rmdir "$CI_KEYCHAIN_DIR"; then
        cleanup_status=1
      fi
    fi

    if [ "$original_status" -ne 0 ]; then
      exit "$original_status"
    fi
    if [ "$cleanup_status" -ne 0 ]; then
      exit "$cleanup_status"
    fi
  }

  trap cleanup_ci_keychain EXIT
  CI_KEYCHAIN_DIR="$(cd "$CI_KEYCHAIN_DIR" && pwd -P)"
  CI_KEYCHAIN="$CI_KEYCHAIN_DIR/tests.keychain-db"
  chmod 700 "$CI_KEYCHAIN_DIR"

  CI_KEYCHAIN_PASSWORD="$(openssl rand -hex 32)"
  CI_KEYCHAIN_MAY_EXIST=1
  security create-keychain -p "$CI_KEYCHAIN_PASSWORD" "$CI_KEYCHAIN"
  security unlock-keychain -p "$CI_KEYCHAIN_PASSWORD" "$CI_KEYCHAIN"
  security set-keychain-settings -u -t 7200 "$CI_KEYCHAIN"
  CI_SEARCH_LIST_MAY_HAVE_CHANGED=1
  security list-keychains -d user -s "$CI_KEYCHAIN" "${PREV_USER_KEYCHAINS[@]}"
  CI_DEFAULT_MAY_HAVE_CHANGED=1
  security default-keychain -d user -s "$CI_KEYCHAIN"

  SELECTED_DEFAULT_KEYCHAIN="$(security default-keychain -d user | sed -E 's/^[[:space:]]*"?//;s/"?[[:space:]]*$//')"
  if [ "$SELECTED_DEFAULT_KEYCHAIN" != "$CI_KEYCHAIN" ]; then
    echo "Failed to select the CI test keychain as the user default" >&2
    exit 1
  fi

  KEYCHAIN_SMOKE_ACCOUNT="${GITHUB_ACTOR:-dash-ci}"
  KEYCHAIN_SMOKE_SERVICE="dash-ci-keychain-${GITHUB_RUN_ID:-$$}-${GITHUB_RUN_ATTEMPT:-0}"
  KEYCHAIN_SMOKE_VALUE="writable"
  security add-generic-password \
    -a "$KEYCHAIN_SMOKE_ACCOUNT" \
    -s "$KEYCHAIN_SMOKE_SERVICE" \
    -w "$KEYCHAIN_SMOKE_VALUE" \
    "$CI_KEYCHAIN"
  STORED_SMOKE_VALUE="$(security find-generic-password \
    -a "$KEYCHAIN_SMOKE_ACCOUNT" \
    -s "$KEYCHAIN_SMOKE_SERVICE" \
    -w \
    "$CI_KEYCHAIN")"
  if [ "$STORED_SMOKE_VALUE" != "$KEYCHAIN_SMOKE_VALUE" ]; then
    echo "CI test keychain read-back did not match the stored value" >&2
    exit 1
  fi
  security delete-generic-password \
    -a "$KEYCHAIN_SMOKE_ACCOUNT" \
    -s "$KEYCHAIN_SMOKE_SERVICE" \
    "$CI_KEYCHAIN"
  unset CI_KEYCHAIN_PASSWORD STORED_SMOKE_VALUE KEYCHAIN_SMOKE_VALUE
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

bash build_ios.sh --target tests --profile dev

swift test

xcodebuild test \
  -project SwiftExampleApp/SwiftExampleApp.xcodeproj \
  -scheme SwiftExampleApp \
  -skip-testing:SwiftExampleAppUITests \
  -destination "platform=iOS Simulator,name=$SIM_NAME"
