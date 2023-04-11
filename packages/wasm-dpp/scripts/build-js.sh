#!/usr/bin/env bash

## Paths to distributive that exposed to library consumers
DIST_DIR="$PWD/dist"
DIST_WASM_DIR="$DIST_DIR/wasm"
DIST_WASM_BINARY_BASE_64="$DIST_WASM_DIR/wasm_dpp_bg.js"

## Paths to wasm files produced by wasm-bindgen
WASM_DIR="$PWD/wasm"
WASM_TYPINGS_PATH="$WASM_DIR/wasm_dpp.d.ts"
WASM_JS_CODE_PATH="$WASM_DIR/wasm_dpp.js"
WASM_BINARY_PATH="$WASM_DIR/wasm_dpp_bg.wasm"

## Paths to TypeScript source files
LIB_DIR="$PWD/lib"
LIB_WASM_DIR="$LIB_DIR/wasm"

# Create directory in TS source files to save wasm TS typings
mkdir -p $LIB_WASM_DIR

# Create directory in dist to save transpiled wasm code and TS typings
mkdir -p $DIST_WASM_DIR

## Converting wasm into base64 and saving it to dist folder
echo "Converting wasm binary into base64 module"
WASM_BUILD_BASE_64=$(base64 -i "$WASM_BINARY_PATH")
echo 'module.exports = "'${WASM_BUILD_BASE_64}'"' > "$DIST_WASM_BINARY_BASE_64"

## Transpile ES Modules code to Common JS
## and save directly to dist folder to avoid re-generating TS declarations
echo "Transpiling wasm ES Modules to CommonJS"
yarn babel "$WASM_JS_CODE_PATH" --out-dir "$DIST_WASM_DIR"

## Copying wasm typings to dist and source folders
## In dist folder they provide typings for external consumers
## In source folder they provide typings for TS compiler
echo "Copying wasm typings"
cp "$WASM_TYPINGS_PATH" "$DIST_WASM_DIR"
cp "$WASM_TYPINGS_PATH" "$LIB_WASM_DIR"

echo "Building TypeScript code"
yarn tsc

echo "Cleaning up intermediate wasm build"
rm -rf $WASM_DIR
