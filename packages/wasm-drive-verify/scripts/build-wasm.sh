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
OUTPUT_FILE="${OUTPUT_DIR}/wasm_drive_verify_bg.wasm"
BUILD_COMMAND="cargo build --config net.git-fetch-with-cli=true --target=${TARGET} ${PROFILE_ARG}"
BINDGEN_COMMAND="wasm-bindgen --out-dir=${OUTPUT_DIR} --target=web --omit-default-module-path ../../target/${TARGET}/${PROFILE}/wasm_drive_verify.wasm"

if ! [[ -d ${OUTPUT_DIR} ]]; then
  mkdir -p "${OUTPUT_DIR}"
fi

# TODO: Build wasm with build.rs
# Meantime if you want to update wasm-bindgen you also need to update version in:
#  - packages/wasm-drive-verify/Cargo.toml
#  - Dockerfile
if ! [[ -x "$(command -v wasm-bindgen)" ]]; then
  echo "Wasm-bindgen CLI is not installed."
  exit 1
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

if command -v wasm-opt &> /dev/null; then
  echo "Optimizing wasm using Binaryen"
  wasm-opt -tnh --flatten --rereloop -Oz --gufa -Oz --gufa -Oz  "$OUTPUT_FILE" -o "$OUTPUT_FILE"
else
  echo "wasm-opt command not found. Skipping wasm optimization."
fi