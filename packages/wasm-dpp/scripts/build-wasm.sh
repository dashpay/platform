#!/usr/bin/env bash
# shellcheck disable=SC2312
set -ex

TARGET=wasm32-unknown-unknown

# "--profile release" is equivalent of "--release", see
# https://github.com/rust-lang/cargo/blob/13413c64ff88dd6c2824e9eb9374fc5f10895d28/src/cargo/util/command_prelude.rs#L426
CARGO_BUILD_PROFILE="${CARGO_BUILD_PROFILE:-dev}"
PROFILE_ARG="--profile ${CARGO_BUILD_PROFILE}"
PROFILE="${CARGO_BUILD_PROFILE}"
if [[ "${CARGO_BUILD_PROFILE}" == "dev" ]]; then
  PROFILE=debug
fi

OUTPUT_DIR="${PWD}/wasm"
# shellcheck disable=SC2034
OUTPUT_FILE="${OUTPUT_DIR}/wasm_dpp_bg.wasm"
BUILD_COMMAND="cargo build --config net.git-fetch-with-cli=true --target=${TARGET} ${PROFILE_ARG}"
BINDGEN_COMMAND="wasm-bindgen --out-dir=${OUTPUT_DIR} --target=web --omit-default-module-path ../../target/${TARGET}/${PROFILE}/wasm_dpp.wasm"

if ! [[ -d ${OUTPUT_DIR} ]]; then
  mkdir -p "${OUTPUT_DIR}"
fi

# TODO: Build wasm with build.rs
# Meantime if you want to update wasm-bindgen you also need to update version in:
#  - packages/wasm-dpp/Cargo.toml
#  - Dockerfile
if ! [[ -x "$(command -v wasm-bindgen)" ]]; then
  echo "Wasm-bindgen CLI ${WASM_BINDGEN_VERSION} is not installed. Installing"
  cargo install --config net.git-fetch-with-cli=true --profile "${CARGO_BUILD_PROFILE}" -f "wasm-bindgen-cli@0.2.86"
fi

# On a mac, bundled clang won't work - you need to install LLVM manually through brew,
# and then set the correct env for the build to work
if [[ "${OSTYPE}" == "darwin"* ]]; then
  AR_PATH=$(command -v llvm-ar)
  CLANG_PATH=$(command -v clang)
  AR=${AR_PATH} CC=${CLANG_PATH} ${BUILD_COMMAND}
  AR=${AR_PATH} CC=${CLANG_PATH} ${BINDGEN_COMMAND}
else
  ${BUILD_COMMAND}
  ${BINDGEN_COMMAND}
fi

# EMCC_CFLAGS="-s ERROR_ON_UNDEFINED_SYMBOLS=0 --no-entry" cargo build --target=wasm32-unknown-emscripten --release
# EMCC_CFLAGS="-s ERROR_ON_UNDEFINED_SYMBOLS=0 --no-entry" wasm-bindgen --out-dir=wasm --target=web --omit-default-module-path ../../target/wasm32-unknown-emscripten/release/wasm_dpp.wasm

# TODO: Must be somehow preinstalled?
#if [ "$PROFILE" == "release" ]; then
#  echo "Optimizing wasm using Binaryen"
#  wasm-opt -Os "$OUTPUT_FILE" -o "$OUTPUT_FILE"
#fi
