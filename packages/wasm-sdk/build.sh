#! /bin/bash
#
# Build WASM-SDK.
#
# EXPERIMENTAL: This script is experimental and may be removed in the future.
#

set -ex -o pipefail

# Disable LTO for WebAssembly builds
export CARGO_PROFILE_RELEASE_LTO=false
export RUSTFLAGS="-C lto=off"

wasm-pack build --target web --release --no-opt
wasm-opt -tnh --flatten --rereloop -Oz -Oz -Oz -o pkg/optimized.wasm pkg/wasm_sdk_bg.wasm
ls -lah pkg
