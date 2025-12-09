#!/usr/bin/env bash
#
# Build wasm-dpp2 using unified build script
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ "${CARGO_BUILD_PROFILE:-}" = "release" ]; then
    exec "$SCRIPT_DIR/build-optimized.sh"
fi

OPT_LEVEL="full"
if [ "${CARGO_BUILD_PROFILE:-}" = "dev" ] || [ "${CI:-}" != "true" ]; then
    OPT_LEVEL="minimal"
fi

exec "$SCRIPT_DIR/../../scripts/build-wasm.sh" --package wasm-dpp2 --opt-level "$OPT_LEVEL"
