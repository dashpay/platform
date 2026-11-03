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

// Optional: enable logs (silent by default)
await sdk.WasmSdk.setLogLevel('warn'); // or 'info' | 'debug' | full filter

// Platform queries via a client
let builder = sdk.WasmSdkBuilder.testnetTrusted();
builder = builder.withSettings(5000, 10000, 3, true);
const client = await builder.withLogs('warn').build();

const DPNS = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
const docs = await client.getDocuments({
  dataContractId: DPNS,
  documentTypeName: 'domain',
  limit: 5,
});
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

### Quorum keys: trusted or verified

Proof verification needs the public key of the Platform quorum that signed each response. A builder takes them from one of two contexts:

- `WasmTrustedContext` (`await WasmTrustedContext.prefetchTestnet()`, then `builder.withTrustedContext(context)`) fetches the quorum lists from the network's quorum service and trusts what it returns. It works on every network, including devnets and local.
- `WasmVerifiedContext` (mainnet and testnet only) proves each key with a DIP 175 quorum proof from a checkpoint built into the SDK. The quorum service then only relays the proofs; a relay that lies can delay requests but cannot make the SDK accept a forged key. Nothing is fetched up front: a key is proven the first time a response signed by its quorum is verified.

```js
const context = sdk.WasmVerifiedContext.testnet(); // or .mainnet(), optionally with relay URLs
const client = sdk.WasmSdkBuilder.testnet().withVerifiedContext(context).build();

// ... after queries ran: proofs grow with the age of the state they start from,
// so save where they got to (e.g. on page hide)...
localStorage.setItem('dash-continuation', JSON.stringify(context.continuation()));
// ...and resume from it next time (a `QuorumProofContinuation`: `{ anchor, states }`).
const saved = JSON.parse(localStorage.getItem('dash-continuation'));
const next = sdk.WasmVerifiedContext.testnetFromContinuation(saved /* , ['https://relay.example'] */);
```

A continuation's `anchor` is the newest verified state more than one DKG interval (24 blocks) below the lowest Core height of a response the SDK verified with the context's keys (the context's own starting point if none was verified, or if no kept state is that far below it, as with a recent custom checkpoint). A Platform quorum signed that height, so Platform reached it, and Platform's height only rises: the next session can still prove the heights Platform reports. The margin is there because a key verified at a height may rest on evidence up to one DKG interval below it (the grace for the block on which Platform still signs with a quorum Core has just retired), and the next session needs a state below that evidence to prove the key again. A response that fails verification never moves the anchor, and neither do keys proven only through `fetchQuorumPublicKey`. `states` are the newer verified states, so proofs stay short. Saved before any response was verified, a continuation is just the starting point and its states. Save the whole continuation rather than `latestVerifiedState()`: that state is often above Platform's height, and no key at or below a lone starting state can be proven. `mainnetWithCheckpoint` / `testnetWithCheckpoint` start from a single `QuorumProofState` you checked independently, in the form of the `target` Dash Core's `getquorumproofchain` returns. Checkpoints and continuations are trust configuration: only pass back what a context saved.

Relays are 1 to 8 base URLs whose `/proofs` endpoint serves the proofs; each must be `https` (plain `http` only on a loopback host). As with the Rust SDK's builder, the context attached last is the one the SDK uses. `build()` throws when the Rust SDK refuses the configuration: a verified context of another network than the builder's, or one with `withProofs(false)`. Contracts a verified SDK caches are persisted apart from the trusted ones (`dash-sdk.contracts.<network>.verified`). The verified context discovers no masternode addresses, so the builder keeps the addresses it has (its preset or `withAddresses` list, or those a trusted context attached before it discovered). The public mainnet quorum service does not serve proofs yet, so on mainnet pass relays that do.

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
let b = sdk.WasmSdkBuilder.testnetTrusted();
const client = await b.withSettings(5000, 10000, 3, true).withLogs('info').build();
const status = await client.getStatus();
// Enumerate contracts one page at a time, ascending by id (pass the last key as startAfter)
const contracts = await client.getDataContractsByRange({ limit: 100 });
// Check that contracts held locally are still current (versions only; the contracts themselves only with includeContracts)
const versions = await client.getDataContractsLatestVersions({ contractIds: ['GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec'] });
// Contract groups: a group's owner/admins/name, its members one kind at a time (page with nextStartAfter), and the groups a contract is in
const contractGroupId = '8Y4NJ3bDgNvWm6tMMXAdEWQEvkqbCiCSwK9KpQTPMFZ2'; // a contract group id, not a contract id
const groupInfo = await client.getContractGroupInfo(contractGroupId);
const members = await client.getContractGroupMembers({ contractGroupId, kind: 'contracts', limit: 50 });
const memberships = await client.getContractGroupsForContract('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
// What is left of the budgets of an identity's keys: a bigint per budgeted key (0n = spent), null for a key without a budget
const budgets = await client.getIdentityKeysRemainingBudgets('5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS', [3, 4]);
// Raise the limits of a key: add credits to its budget, or move its expiry later (signed by a MASTER key or an unlimited CRITICAL key)
const key = await client.identityUpdateKeyLimits({ identity, keyId: 5, addBudget: 100000000n, signer });
// Seed a contract the app already holds (a bundled snapshot); pair with the versions check above
client.addKnownContract(contract);
client.free();
```

The full surface includes identity, document, contract, token, group, epoch/system, and proof helpers.

### Logging

By default, the SDK is silent. You can enable tracing logs globally or via the builder:

```js
// Globally
await sdk.WasmSdk.setLogLevel('info');

// Or on the builder (applies immediately)
await sdk.WasmSdkBuilder.new_testnet().withLogs('wasm_sdk=debug,rs_dapi_client=warn').build();
```

Accepted values are simple levels ('off'|'error'|'warn'|'info'|'debug'|'trace') or a full EnvFilter string.

---

## Environment & compatibility

- ESM-only package ("type": "module"). Use dynamic import in CJS.
- Node.js: >= 22.
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

const builder = sdk.WasmSdkBuilder.testnetTrusted();
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
