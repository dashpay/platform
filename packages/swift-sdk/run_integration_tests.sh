#!/bin/bash
set -euo pipefail

# Run the Swift SDK integration tests against a local dashmate devnet.
# Restarts dashmate fresh every run, builds the FFI, runs the tests with
# Address Sanitizer, always tears the devnet down afterwards, and exits
# with the test runner's status so CI fails on a test failure.
#
# One-time setup: `yarn dashmate setup --preset local`.

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
REPO_ROOT="$( cd "$SCRIPT_DIR/../.." && pwd )"

cd "$SCRIPT_DIR"

# dashmate pins Node 20–22; bail early instead of failing several
# minutes into the dashmate startup with a cryptic error.
node_major=$(node -p "process.versions.node.split('.')[0]" 2>/dev/null || echo 0)
if [ "$node_major" -gt 22 ] || [ "$node_major" -lt 20 ]; then
  echo "Node $(node --version 2>/dev/null || echo '<missing>') is not supported by dashmate."
  echo "Use Node 20–22 (e.g. \`nvm use 22\`) and re-run."
  exit 1
fi

# Bootstrap JS workspace once. Both are idempotent skip-fast paths.
[ -d "$REPO_ROOT/.yarn/unplugged" ] || (cd "$REPO_ROOT" && yarn install)
[ -f "$REPO_ROOT/packages/wasm-dpp/dist/index.js" ] || (cd "$REPO_ROOT" && yarn build)

cleanup() {
  trap - EXIT INT TERM  # disarm so a second signal / the EXIT can't re-enter

  for pid in "${devnet_pid:-}" "${build_pid:-}"; do
    [ -n "$pid" ] && kill "$pid" 2>/dev/null || true
  done

  echo "tearing down dashmate devnet group…"
  (cd "$REPO_ROOT" && yarn dashmate group stop --group=local --force 2>/dev/null || true)
}

trap cleanup EXIT
trap 'cleanup; exit 130' INT TERM

# Always start from a FRESH devnet
(
echo "restarting dashmate devnet group (fresh, ~1-2 min)…"
(cd "$REPO_ROOT" && yarn dashmate group stop --group=local --force 2>/dev/null || true)
(cd "$REPO_ROOT" && yarn dashmate group start --group=local --wait-for-readiness)
) &
devnet_pid=$!

SKIP_EXAMPLE_APP_BUILD=1 bash build_ios.sh --target mac --profile dev &
build_pid=$!

devnet_rc=0; wait "$devnet_pid" || devnet_rc=$?
build_rc=0;  wait "$build_pid"  || build_rc=$?

if [ "$devnet_rc" -ne 0 ]; then echo "dashmate devnet failed (rc=$devnet_rc)" >&2; exit "$devnet_rc"; fi
if [ "$build_rc" -ne 0 ]; then echo "FFI build failed (rc=$build_rc)" >&2; exit "$build_rc"; fi

DASH_KEYCHAIN_SERVICE="org.dashfoundation.wallet.itest.$(uuidgen)"

RUN_INTEGRATION_TESTS=1 DASH_KEYCHAIN_SERVICE="$DASH_KEYCHAIN_SERVICE" swift test --sanitize=address --filter SwiftDashSDKIntegrationTests "$@"
