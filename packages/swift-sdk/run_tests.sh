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
# touches a developer's keychain configuration; the selected restorable
# baseline is restored on exit.
if [ -n "${CI:-}${GITHUB_ACTIONS:-}" ]; then
  normalize_security_keychain_path() {
    sed -E 's/^[[:space:]]*"?//;s/"?[[:space:]]*$//'
  }

  canonicalize_existing_keychain() {
    candidate_path="$1"
    if [ -z "$candidate_path" ] || [ -L "$candidate_path" ] \
      || [ ! -f "$candidate_path" ]; then
      return 1
    fi

    candidate_dir="$(dirname "$candidate_path")"
    candidate_name="$(basename "$candidate_path")"
    canonical_dir="$(cd "$candidate_dir" 2>/dev/null && pwd -P)" || return 1
    canonical_path="$canonical_dir/$candidate_name"
    if [ -L "$canonical_path" ] || [ ! -f "$canonical_path" ]; then
      return 1
    fi

    printf '%s\n' "$canonical_path"
  }

  canonicalize_recovery_keychain() {
    canonical_path="$(canonicalize_existing_keychain "$1")" || return 1
    case "$canonical_path" in
      "$CANONICAL_USER_KEYCHAIN_DIR"/*)
        ;;
      *)
        return 1
        ;;
    esac
    if [ ! -O "$canonical_path" ] || [ ! -w "$canonical_path" ]; then
      return 1
    fi

    printf '%s\n' "$canonical_path"
  }

  keychain_array_contains() {
    searched_path="$1"
    shift
    for existing_path in "$@"; do
      if [ "$existing_path" = "$searched_path" ]; then
        return 0
      fi
    done
    return 1
  }

  RAW_USER_KEYCHAINS_OUTPUT="$(security list-keychains -d user)"
  RAW_USER_KEYCHAINS=()
  while IFS= read -r keychain_path; do
    keychain_path="$(printf '%s\n' "$keychain_path" | normalize_security_keychain_path)"
    if [ -n "$keychain_path" ]; then
      RAW_USER_KEYCHAINS+=("$keychain_path")
    fi
  done <<< "$RAW_USER_KEYCHAINS_OUTPUT"
  if [ "${#RAW_USER_KEYCHAINS[@]}" -eq 0 ]; then
    echo "Cannot repair an empty user keychain search list" >&2
    exit 1
  fi

  if ! CANONICAL_USER_KEYCHAIN_DIR="$(
    cd "$HOME/Library/Keychains" 2>/dev/null && pwd -P
  )"; then
    echo "The user keychain directory is unavailable" >&2
    exit 1
  fi

  RAW_DEFAULT_KEYCHAIN_OUTPUT=""
  DEFAULT_QUERY_STATUS=0
  if RAW_DEFAULT_KEYCHAIN_OUTPUT="$(security default-keychain -d user 2>&1)"; then
    DEFAULT_QUERY_STATUS=0
  else
    DEFAULT_QUERY_STATUS=$?
  fi

  RAW_DEFAULT_KEYCHAIN=""
  if [ "$DEFAULT_QUERY_STATUS" -eq 0 ]; then
    RAW_DEFAULT_KEYCHAIN="$(
      printf '%s\n' "$RAW_DEFAULT_KEYCHAIN_OUTPUT" \
        | normalize_security_keychain_path
    )"
  else
    case "$DEFAULT_QUERY_STATUS" in
      37|44)
        ;;
      *)
        printf '%s\n' "$RAW_DEFAULT_KEYCHAIN_OUTPUT" >&2
        exit "$DEFAULT_QUERY_STATUS"
        ;;
    esac
  fi

  BASELINE_DEFAULT_KEYCHAIN=""
  BASELINE_USER_KEYCHAINS=()
  BASELINE_COMMITTED=0
  DEFAULT_IS_COHERENT=0

  if [ "$DEFAULT_QUERY_STATUS" -eq 0 ] && [ -n "$RAW_DEFAULT_KEYCHAIN" ]; then
    if canonical_default="$(
      canonicalize_existing_keychain "$RAW_DEFAULT_KEYCHAIN"
    )"; then
      raw_index=0
      while [ "$raw_index" -lt "${#RAW_USER_KEYCHAINS[@]}" ]; do
        raw_path="${RAW_USER_KEYCHAINS[$raw_index]}"
        if canonical_raw_path="$(canonicalize_existing_keychain "$raw_path")" \
          && [ "$canonical_raw_path" = "$canonical_default" ]; then
          DEFAULT_IS_COHERENT=1
          break
        fi
        raw_index=$((raw_index + 1))
      done
    fi
  fi

  RECOVERY_USER_KEYCHAINS=()
  if [ "$DEFAULT_IS_COHERENT" -eq 1 ]; then
    BASELINE_DEFAULT_KEYCHAIN="$RAW_DEFAULT_KEYCHAIN"
    BASELINE_USER_KEYCHAINS=("${RAW_USER_KEYCHAINS[@]}")
    BASELINE_COMMITTED=1
  else
    LOGIN_KEYCHAIN="$HOME/Library/Keychains/login.keychain-db"
    if canonical_login="$(canonicalize_recovery_keychain "$LOGIN_KEYCHAIN")"; then
      RECOVERY_USER_KEYCHAINS+=("$canonical_login")
    fi

    raw_index=0
    while [ "$raw_index" -lt "${#RAW_USER_KEYCHAINS[@]}" ]; do
      raw_path="${RAW_USER_KEYCHAINS[$raw_index]}"
      if canonical_raw_path="$(canonicalize_recovery_keychain "$raw_path")"; then
        if [ "${#RECOVERY_USER_KEYCHAINS[@]}" -eq 0 ] \
          || ! keychain_array_contains \
            "$canonical_raw_path" "${RECOVERY_USER_KEYCHAINS[@]}"; then
          RECOVERY_USER_KEYCHAINS+=("$canonical_raw_path")
        fi
      fi
      raw_index=$((raw_index + 1))
    done

    if [ "${#RECOVERY_USER_KEYCHAINS[@]}" -eq 0 ]; then
      echo "No eligible user keychain is available for default recovery" >&2
      exit 1
    fi
  fi

  CI_KEYCHAIN_DIR="$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/dash-ci-keychain.XXXXXX")"
  CI_KEYCHAIN="$CI_KEYCHAIN_DIR/tests.keychain-db"
  CI_KEYCHAIN_MAY_EXIST=0
  CI_DEFAULT_MAY_HAVE_CHANGED=0
  CI_SEARCH_LIST_MAY_HAVE_CHANGED=0
  REPAIR_LIST_MAY_HAVE_CHANGED=0
  REPAIR_CONTAINMENT_ACTIVE=0

  cleanup_ci_keychain() {
    original_status=$?
    cleanup_status=0
    trap - EXIT

    if [ "${BASELINE_COMMITTED:-0}" -eq 1 ]; then
      if [ "${CI_DEFAULT_MAY_HAVE_CHANGED:-0}" -eq 1 ]; then
        if ! security default-keychain \
          -d user -s "$BASELINE_DEFAULT_KEYCHAIN"; then
          cleanup_status=1
        fi
      fi
      if [ "${CI_SEARCH_LIST_MAY_HAVE_CHANGED:-0}" -eq 1 ]; then
        if ! security list-keychains \
          -d user -s "${BASELINE_USER_KEYCHAINS[@]}"; then
          cleanup_status=1
        fi
      fi
    elif [ "${REPAIR_LIST_MAY_HAVE_CHANGED:-0}" -eq 1 ] \
      && [ "${REPAIR_CONTAINMENT_ACTIVE:-0}" -eq 0 ]; then
      if ! security list-keychains \
        -d user -s "${RAW_USER_KEYCHAINS[@]}"; then
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

  if [ "$BASELINE_COMMITTED" -eq 0 ]; then
    REPAIR_LIST_MAY_HAVE_CHANGED=1
    security list-keychains \
      -d user -s "${RECOVERY_USER_KEYCHAINS[@]}"

    REPAIR_CONTAINMENT_ACTIVE=1
    security default-keychain \
      -d user -s "${RECOVERY_USER_KEYCHAINS[0]}"

    REPAIRED_USER_KEYCHAINS_OUTPUT="$(security list-keychains -d user)"
    REPAIRED_USER_KEYCHAINS=()
    while IFS= read -r keychain_path; do
      keychain_path="$(
        printf '%s\n' "$keychain_path" | normalize_security_keychain_path
      )"
      if [ -n "$keychain_path" ]; then
        if ! canonical_path="$(canonicalize_existing_keychain "$keychain_path")"; then
          echo "Repaired keychain search list contains an invalid path" >&2
          exit 1
        fi
        REPAIRED_USER_KEYCHAINS+=("$canonical_path")
      fi
    done <<< "$REPAIRED_USER_KEYCHAINS_OUTPUT"

    if [ "${#REPAIRED_USER_KEYCHAINS[@]}" \
      -ne "${#RECOVERY_USER_KEYCHAINS[@]}" ]; then
      echo "Repaired keychain search list did not match the selected baseline" >&2
      exit 1
    fi
    repaired_index=0
    while [ "$repaired_index" -lt "${#RECOVERY_USER_KEYCHAINS[@]}" ]; do
      if [ "${REPAIRED_USER_KEYCHAINS[$repaired_index]}" \
        != "${RECOVERY_USER_KEYCHAINS[$repaired_index]}" ]; then
        echo "Repaired keychain search list did not match the selected baseline" >&2
        exit 1
      fi
      repaired_index=$((repaired_index + 1))
    done

    REPAIRED_DEFAULT_KEYCHAIN="$(
      security default-keychain -d user | normalize_security_keychain_path
    )"
    if ! canonical_repaired_default="$(
      canonicalize_existing_keychain "$REPAIRED_DEFAULT_KEYCHAIN"
    )" || [ "$canonical_repaired_default" != "${RECOVERY_USER_KEYCHAINS[0]}" ]; then
      echo "Repaired default keychain did not match the selected baseline" >&2
      exit 1
    fi

    BASELINE_DEFAULT_KEYCHAIN="${RECOVERY_USER_KEYCHAINS[0]}"
    BASELINE_USER_KEYCHAINS=("${RECOVERY_USER_KEYCHAINS[@]}")
    BASELINE_COMMITTED=1
    REPAIR_LIST_MAY_HAVE_CHANGED=0
  fi

  CI_KEYCHAIN_PASSWORD="$(openssl rand -hex 32)"
  CI_KEYCHAIN_MAY_EXIST=1
  security create-keychain -p "$CI_KEYCHAIN_PASSWORD" "$CI_KEYCHAIN"
  security unlock-keychain -p "$CI_KEYCHAIN_PASSWORD" "$CI_KEYCHAIN"
  security set-keychain-settings -u -t 7200 "$CI_KEYCHAIN"
  CI_SEARCH_LIST_MAY_HAVE_CHANGED=1
  security list-keychains \
    -d user -s "$CI_KEYCHAIN" "${BASELINE_USER_KEYCHAINS[@]}"
  CI_DEFAULT_MAY_HAVE_CHANGED=1
  security default-keychain -d user -s "$CI_KEYCHAIN"

  SELECTED_DEFAULT_KEYCHAIN="$(
    security default-keychain -d user | normalize_security_keychain_path
  )"
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
    -w)"
  if [ "$STORED_SMOKE_VALUE" != "$KEYCHAIN_SMOKE_VALUE" ]; then
    echo "CI test keychain read-back did not match the stored value" >&2
    exit 1
  fi
  security delete-generic-password \
    -a "$KEYCHAIN_SMOKE_ACCOUNT" \
    -s "$KEYCHAIN_SMOKE_SERVICE"
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
