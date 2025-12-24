#!/usr/bin/env bash
#
# Build wasm-drive-verify using unified build script
#

set -euo pipefail

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Initialize CARGO_BUILD_PROFILE with default value if not set
CARGO_BUILD_PROFILE="${CARGO_BUILD_PROFILE:-dev}"

# Determine optimization level based on environment
OPT_LEVEL="minimal"
if [ "${CARGO_BUILD_PROFILE}" = "release" ] || [ "${GITHUB_EVENT_NAME:-}" = "release" ] || [ "${GITHUB_EVENT_NAME:-}" = "workflow_dispatch" ]; then
    OPT_LEVEL="full"
fi

# Call unified build script
exec "$SCRIPT_DIR/../scripts/build-wasm.sh" --package wasm-drive-verify --opt-level "$OPT_LEVEL"
