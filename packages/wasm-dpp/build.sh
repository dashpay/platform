# Building WASM
AR=/usr/local/opt/llvm/bin/llvm-ar CC=/usr/local/opt/llvm/bin/clang cargo build --target=wasm32-unknown-unknown
# Generating bindings
AR=/usr/local/opt/llvm/bin/llvm-ar CC=/usr/local/opt/llvm/bin/clang wasm-bindgen --out-dir=wasm --target=web --omit-default-module-path ../../target/wasm32-unknown-unknown/debug/wasm_dpp.wasm

# Converting wasm into bease64 so it can be bundled
WASM_BUILD_BASE_64=$(base64 wasm/wasm_dpp_bg.wasm)
echo 'module.exports = "'${WASM_BUILD_BASE_64}'"' > wasm/wasm_dpp_bg.js

# The module is in typescript so it's easier to generate typings
# Building a distributable library with Webpack
yarn workspace @dashevo/wasm-dpp webpack
