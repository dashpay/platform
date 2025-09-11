# WASM SDK

Dash Platform WebAssembly SDK for JavaScript and TypeScript. The core is implemented in Rust and compiled to WebAssembly; a thin ESM wrapper exposes a convenient JS API that works in both Node.js and modern browsers.

The SDK provides:
- Cryptographic utilities: mnemonic generation/validation, key derivation, key pair generation, address validation, message signing.
- Platform state transitions and queries: identities, documents, data contracts, tokens, groups, epochs/system, voting, proofs (when supported).
- A builder pattern (`WasmSdkBuilder`) to configure and construct a client for network-backed queries.

This package ships a single-file ESM build (`dist/sdk.js`) with the Wasm inlined and compiled off the main thread in browsers. Advanced users can also opt into separate raw artifacts under `dist/raw/*`.

---

## Install

From npm (consumer apps):

```bash
npm install @dashevo/wasm-sdk
# or
yarn add @dashevo/wasm-sdk
```

From this monorepo (contributors):

```bash
# fast for development
yarn build
# or optimized
yarn run build:release
```

The package is ESM-only ("type": "module"). In CommonJS, use dynamic import().

---

## Usage

Always call `await init()` once before using the API. It is idempotent and safe to call multiple times.

### Node.js (ESM)

```js
import init, * as sdk from '@dashevo/wasm-sdk';

// Initialize Wasm (Node uses an inlined binary; no assets to load)
await init();

// Crypto helpers
const { address } = sdk.generate_key_pair('testnet');
console.log('Address:', address);

// Platform queries via a client
let builder = sdk.WasmSdkBuilder.new_testnet_trusted();
builder = builder.with_settings(5000, 10000, 3, true);
const client = await builder.build();

const DPNS = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
const docs = await sdk.get_documents(client, DPNS, 'domain', null, null, 5, null, null);
console.log('Docs:', docs.length);

client.free();
```

### Browser (bundler, ESM)

```js
import init, * as sdk from '@dashevo/wasm-sdk';

// Initialize Wasm (browser compiles in a Web Worker; no separate .wasm files)
await init();

const ok = sdk.validate_address('yXXX...', 'testnet');
```

### Advanced: raw artifacts (separate .wasm)

If you prefer to manage the `.wasm` file via your bundler, import the raw entry explicitly:

```js
import init, * as sdk from '@dashevo/wasm-sdk/raw';
await init(); // wasm-bindgen will resolve and fetch wasm_sdk_bg.wasm
```

Configure your bundler accordingly. For webpack 5:

```js
// webpack config
module.exports = {
  experiments: { asyncWebAssembly: true },
  module: { rules: [{ test: /\.wasm$/, type: 'asset/resource' }] },
};
```

---

## How bundling works

The publishable build provides two ways to consume the SDK:

1) Single file (default): `import '@dashevo/wasm-sdk'`
- `dist/sdk.js` inlines the Wasm (base64) to avoid asset pipelines and MIME issues.
- Browser: compiles the inlined bytes in a Web Worker to avoid main-thread stalls and 8MB sync limits; then instantiates on the main thread.
- Node: uses the inlined bytes with initSync internally; you still call `await init()` for a consistent API.
- The wrapper imports a sanitized variant of the wasm-bindgen glue so bundlers do not "see" any `new URL('…wasm')` and therefore will not emit a `.wasm` asset.

2) Raw artifacts (opt-in): `import '@dashevo/wasm-sdk/raw'`
- `dist/raw/wasm_sdk.js` (unmodified wasm-bindgen output) plus `dist/raw/wasm_sdk_bg.wasm`.
- Your bundler must serve the `.wasm` with the correct content type and URL rewriting.

Why this design?
- Eliminate flaky Wasm asset handling in test/dev and simplify consumer setup.
- Keep an escape hatch for asset-pipeline users.

---

## Build (contributors)

Prerequisites: Rust toolchain, `wasm-pack`, and (optionally) Binaryen for optimized builds.

Commands (run at repo root or inside `packages/wasm-sdk`):

```bash
# Development build (fast)
yarn workspace @dashevo/wasm-sdk build

# Release build (optimized)
yarn workspace @dashevo/wasm-sdk build:release
```
---

## Test (contributors)

Unit tests (Node + browser/Karma):

```bash
yarn workspace @dashevo/wasm-sdk test:unit
```

Functional tests (networked; some cases may be skipped if offline):

```bash
yarn workspace @dashevo/wasm-sdk test:functional
```

---

## API highlights

Examples (after `await init()`):

```js
// Addresses & keys
const { address, private_key_wif } = sdk.generate_key_pair('mainnet');
sdk.validate_address(address, 'mainnet');
const addr = sdk.pubkey_to_address('02...pubkeyHex', 'testnet');

// Mnemonics & derivation
const m = sdk.generate_mnemonic(12);
const r = sdk.derive_key_from_seed_with_path(m, undefined, "m/44'/5'/0'/0/0", 'mainnet');

// Network client
let b = sdk.WasmSdkBuilder.new_testnet_trusted();
const client = await b.with_settings(5000, 10000, 3, true).build();
const status = await sdk.get_status(client);
client.free();
```

The full surface includes identity, document, contract, token, group, epoch/system, and proof helpers.

---

## Environment & compatibility

- ESM-only package ("type": "module"). Use dynamic import in CJS.
- Node.js: 16+ recommended (18+ preferred).
- Browsers: modern engines with WebAssembly + Web Workers.

---

## Troubleshooting

- "expected magic word 00 61 73 6d …" in browsers:
  - Ensure you import the default entry (`@dashevo/wasm-sdk`), not the raw one, unless you have configured Wasm assets.
  - Always `await init()` before calling functions.

- Karma/webpack serving errors:
  - Not applicable with the default single-file build. If you opt into `@dashevo/wasm-sdk/raw`, configure a `.wasm` asset rule and correct MIME type.

---

## Contributing & License

This package is part of the Dash Platform monorepo. Follow the repository’s contribution guidelines and do not commit secrets. See the root repository for license details.

---

## State transitions

In addition to read-only queries, the SDK exposes helpers to construct and submit state transitions (requires a networked client and valid inputs):

```js
import init, * as sdk from '@dashevo/wasm-sdk';
await init();

const builder = sdk.WasmSdkBuilder.new_testnet_trusted();
const client = await builder.build();

// Identity create (example — requires a valid proof and keys)
const proofJson = JSON.stringify({ /* platform-provided proof object */ });
const assetLockWif = 'Kx...';
const pubKeysJson = JSON.stringify([
  { keyType: 'ECDSA_SECP256K1', purpose: 'AUTHENTICATION', securityLevel: 'MASTER', privateKeyHex: '...' },
]);
await sdk.identityCreate(proofJson, assetLockWif, pubKeysJson)
  .then(() => {/* submitted */})
  .catch((e) => {/* handle error */});

// Token transfer (negative example if parameters are invalid)
await sdk.tokenTransfer('contractId', 0, '1000', 'senderIdentityId', 'recipientIdentityId', assetLockWif, null)
  .catch(() => {});

client.free();
```
