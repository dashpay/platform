# Building WASM
AR=/usr/local/opt/llvm/bin/llvm-ar CC=/usr/local/opt/llvm/bin/clang cargo build --target=wasm32-unknown-unknown
# Generating bindings
AR=/usr/local/opt/llvm/bin/llvm-ar CC=/usr/local/opt/llvm/bin/clang wasm-bindgen --out-dir=wasm --target=web --omit-default-module-path ../../target/wasm32-unknown-unknown/debug/wasm_dpp.wasm

# Converting wasm into bease64 so it can be bundled
WASM_BUILD_BASE_64=$(base64 wasm/wasm_dpp_bg.wasm)
echo 'module.exports = "'${WASM_BUILD_BASE_64}'"' > wasm/wasm_dpp_bg.js

mkdir -p "dist/wasm"

# The module is in typescript so it's easier to generate typings
yarn workspace @dashevo/wasm-dpp tsc -d mod.ts

echo "Copying typings to the dist"
cp wasm/wasm_dpp.d.ts dist/wasm/index.d.ts
cp mod.d.ts dist/index.d.ts

# Make webpack happy and typings work
echo "{ \"main\": \"wasm_dpp.js\" }" > wasm/package.json

# Building a distributable library with Webpack
yarn workspace @dashevo/wasm-dpp webpack
