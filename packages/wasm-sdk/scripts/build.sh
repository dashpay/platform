#!/usr/bin/env bash
#
# Build WASM-SDK using unified build script
#
# EXPERIMENTAL: This script is experimental and may be removed in the future.
#

set -euo pipefail

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# If building with release profile, use the optimized build script
if [ "${CARGO_BUILD_PROFILE:-}" = "release" ]; then
    # exec replaces the current shell process; the script will not continue after this.
    exec "$SCRIPT_DIR/build-optimized.sh"
fi

# Determine optimization level based on environment
OPT_LEVEL="full"
if [ "${CARGO_BUILD_PROFILE:-}" = "dev" ] || [ "${CI:-}" != "true" ]; then
    OPT_LEVEL="minimal"
fi

# Functional tests use the mock-only key helper, so add `mocks` to any caller
# features. build-wasm.sh never passes --no-default-features, so the default
# contracts stay on.
export CARGO_BUILD_FEATURES="mocks${CARGO_BUILD_FEATURES:+,$CARGO_BUILD_FEATURES}"

exec "$SCRIPT_DIR/../../scripts/build-wasm.sh" --package wasm-sdk --opt-level "$OPT_LEVEL"
