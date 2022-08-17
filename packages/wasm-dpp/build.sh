AR=/usr/local/opt/llvm/bin/llvm-ar CC=/usr/local/opt/llvm/bin/clang cargo build --target=wasm32-unknown-unknown
AR=/usr/local/opt/llvm/bin/llvm-ar CC=/usr/local/opt/llvm/bin/clang wasm-bindgen --out-dir=wasm --target=web --omit-default-module-path ../../target/wasm32-unknown-unknown/debug/wasm_dpp.wasm

WASM_BUILD_BASE_64=$(base64 wasm/wasm_dpp_bg.wasm)

# fs.readFile/fetch of `x11-hash.wasm` isn't suitable for bundling into libraries
# Produce JS file with the wasm build base64 instead
echo 'module.exports = "'${WASM_BUILD_BASE_64}'"' > wasm/wasm_dpp_bg.js