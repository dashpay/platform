#! /bin/bash
#
# Build WASM-SDK.
#
# EXPERIMENTAL: This script is experimental and may be removed in the future.
#

set -ex -o pipefail

wasm-pack build --target web --release --no-opt
wasm-opt -tnh --flatten --rereloop -Oz --gufa -Oz --gufa -Oz -o pkg/optimized.wasm pkg/wasm_sdk_bg.wasm
ls -lah pkg
