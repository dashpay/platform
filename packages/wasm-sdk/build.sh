#!/usr/bin/env bash
#
# Build WASM-SDK using unified build script
#
# EXPERIMENTAL: This script is experimental and may be removed in the future.
#

set -euo pipefail

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Determine optimization level based on environment
OPT_LEVEL="full"
if [ "${CARGO_BUILD_PROFILE:-}" = "dev" ] || [ "${CI:-}" != "true" ]; then
    OPT_LEVEL="minimal"
fi

# Call unified build script with only the contracts we need
export CARGO_BUILD_FEATURES="dpns-contract,dashpay-contract,wallet-utils-contract,keywords-contract"
exec "$SCRIPT_DIR/../scripts/build-wasm.sh" --package wasm-sdk --opt-level "$OPT_LEVEL"
