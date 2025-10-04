# @dashevo/wasm-dpp2

Internal build of the Dash Platform Protocol v2 WebAssembly bindings.

## Scripts

- `yarn build` – run the unified WASM build and bundle artifacts into `dist/`
- `yarn build:release` – same as `build` but forces full release optimisations
- `yarn test` – execute unit + browser (Karma) tests
- `yarn lint` – lint test sources

The build scripts defer to `packages/scripts/build-wasm.sh` to keep behaviour
consistent with other WASM packages such as `@dashevo/wasm-sdk`.
