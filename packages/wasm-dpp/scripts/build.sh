TARGET=wasm32-unknown-unknown
PROFILE=release

OUTPUT_DIR="$PWD/wasm"
OUTPUT_FILE="$OUTPUT_DIR/wasm_dpp_bg.wasm"
OUTPUT_FILE_JS="$OUTPUT_DIR/wasm_dpp_bg.js"

BUILD_COMMAND="cargo build --target=$TARGET --$PROFILE"
BINDGEN_COMMAND="wasm-bindgen --out-dir=$OUTPUT_DIR --target=web --omit-default-module-path ../../target/$TARGET/$PROFILE/wasm_dpp.wasm"

if [[ "$OSTYPE" == "darwin"* ]]; then
    AR=/usr/local/opt/llvm/bin/llvm-ar CC=/usr/local/opt/llvm/bin/clang $BUILD_COMMAND
    AR=/usr/local/opt/llvm/bin/llvm-ar CC=/usr/local/opt/llvm/bin/clang $BINDGEN_COMMAND
else
    $BUILD_COMMAND
    $BINDGEN_COMMAND
fi

# EMCC_CFLAGS="-s ERROR_ON_UNDEFINED_SYMBOLS=0 --no-entry" cargo build --target=wasm32-unknown-emscripten --release
# EMCC_CFLAGS="-s ERROR_ON_UNDEFINED_SYMBOLS=0 --no-entry" wasm-bindgen --out-dir=wasm --target=web --omit-default-module-path ../../target/wasm32-unknown-emscripten/release/wasm_dpp.wasm

# TODO: DO THIS ONLY ON RELEASE! REQUIRES BINARYEN
# echo "Optimizing wasm using Binaryen"
# wasm-opt -Os $OUTPUT_FILE -o $OUTPUT_FILE

# Converting wasm into bease64 so it can be bundled
echo "Converting wasm binary into base64 module for bundling with Webpack"
WASM_BUILD_BASE_64=$(base64 $OUTPUT_FILE)
echo 'module.exports = "'${WASM_BUILD_BASE_64}'"' > "$OUTPUT_FILE_JS"
