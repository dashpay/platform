#!/bin/sh
set -e
set -u

mkdir -p ./dist/wasm/
mkdir -p ./lib/wasm/

# Paths to wasm files produced by wasm-bindgen
#   ./wasm/wasm_dpp.d.ts
#   ./wasm/wasm_dpp.js
#   ./wasm/wasm_dpp_bg.wasm

CARGO_BUILD_PROFILE="${CARGO_BUILD_PROFILE:-dev}"
PROFILE="${CARGO_BUILD_PROFILE}"
if test "${CARGO_BUILD_PROFILE}" = "dev"; then
    PROFILE=debug
fi
if ! test -f ./wasm/wasm_dpp_bg.wasm; then
    wasm-bindgen --out-dir=./wasm/ --target=web --omit-default-module-path ../../target/wasm32-unknown-unknown/"${PROFILE}"/wasm_dpp.wasm
fi

echo 'Converting wasm binary into base64 module'
WASM_BUILD_BASE_64="$(base64 -w 0 './wasm/wasm_dpp_bg.wasm')"
echo 'module.exports = "'"${WASM_BUILD_BASE_64}"'"' > './dist/wasm/wasm_dpp_bg.js'

## save directly to dist folder to avoid re-generating TS declarations
echo 'Transpiling wasm ES Modules to CommonJS'
yarn babel './wasm/wasm_dpp.js' --out-dir './dist/wasm/'

## In dist folder they provide typings for external consumers
## In source folder they provide typings for TS compiler
echo 'Copying wasm typings'
cp -RPp './wasm/wasm_dpp.d.ts' './dist/wasm/'
mkdir -p './lib/wasm'
cp -RPp './wasm/wasm_dpp.d.ts' './lib/wasm/'

echo 'Building TypeScript code'
yarn tsc

echo 'Cleaning up intermediate wasm build'
rm -rf './wasm/'
