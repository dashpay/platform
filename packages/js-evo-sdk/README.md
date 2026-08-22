# Evo SDK

[![NPM Version](https://img.shields.io/npm/v/@dashevo/evo-sdk)](https://www.npmjs.com/package/@dashevo/evo-sdk)
[![Build Status](https://github.com/dashpay/platform/actions/workflows/release.yml/badge.svg)](https://github.com/dashpay/platform/actions/workflows/release.yml)
[![Release Date](https://img.shields.io/github/release-date/dashpay/platform)](https://github.com/dashpay/platform/releases/latest)
[![standard-readme compliant](https://img.shields.io/badge/readme%20style-standard-brightgreen)](https://github.com/RichardLitt/standard-readme)

TypeScript SDK for building applications on Dash Platform

Evo SDK provides a high-level, strongly-typed interface for interacting with [Dash Platform](https://dashplatform.readme.io/docs/introduction-what-is-dash-platform/) on [supported networks](https://github.com/dashpay/platform/#supported-networks). It wraps the WebAssembly-based [@dashevo/wasm-sdk](../wasm-sdk/) in ergonomic facades covering identities, documents, data contracts, tokens, DPNS, and more. The SDK works in both Node.js and modern browsers.

## Table of Contents

- [Install](#install)
- [Usage](#usage)
- [Facades](#facades)
- [Contributing](#contributing)
- [License](#license)

## Install

```sh
npm install @dashevo/evo-sdk
```

The package is ESM-only (`"type": "module"`). In CommonJS projects, use dynamic `import()`. Requires Node.js >= 18.18.

## Usage

Trusted mode is required for all queries. It pre-fetches quorum public keys so the SDK can verify Platform proofs.

```typescript
import { EvoSDK } from '@dashevo/evo-sdk';

const sdk = EvoSDK.testnetTrusted(); // or mainnetTrusted()
await sdk.connect();

const epoch = await sdk.epoch.current();
console.log('Current epoch:', epoch.index);
```

### Configuration

`EvoSDK` accepts the following options:

| Option | Type | Default | Notes |
|--------|------|---------|-------|
| `network` | `'testnet' \| 'mainnet' \| 'local' \| 'devnet'` | `'testnet'` | Target network. |
| `trusted` | `boolean` | `false` | When `true`, pre-fetches quorum keys for proof verification. Required for default query methods. |
| `addresses` | `string[]` | — | Seed masternode addresses. Required for non-trusted devnet; optional for other networks (replaces built-in defaults). |
| `devnetName` | `string` | — | Short name of the devnet (e.g. `'paloma'`). Required when `network: 'devnet'` and `trusted: true` (used to derive the quorum URL); ignored otherwise — only valid when `network === 'devnet'`. |
| `quorumUrl` | `string` | — | Override the trusted-context quorum base URL. Only meaningful when `trusted: true`. Useful for staging endpoints or devnets where the public DNS isn't deployed yet. |
| `proofs` | `boolean` | `true` | Setting to `false` disables proof requests where supported, but unproved mode is limited — several query paths (e.g. document fetches) force proofs regardless, and some query builders reject the unproved path. Mainly intended for mock/offline replay. |
| `version` | `number` | latest | Platform protocol version. |
| `logs` | `string` | — | Tracing/log filter for the underlying Wasm SDK. Accepts simple levels (`'info'`, `'debug'`, …) or a full `EnvFilter` string. |
| `settings` | `{ connectTimeoutMs?, timeoutMs?, retries?, banFailedAddress? }` | — | DAPI client transport settings. |

Preset factories are available as convenience: `EvoSDK.testnet()`, `EvoSDK.mainnet()`, `EvoSDK.testnetTrusted()`, `EvoSDK.mainnetTrusted()`, `EvoSDK.local()`, `EvoSDK.localTrusted()` (the last two target a dashmate local node), and the devnet factories `EvoSDK.devnet(name, options)` / `EvoSDK.devnetTrusted(name, options)`.

```typescript
// Trusted devnet — quorum URL auto-derived from the devnet name.
const sdk = EvoSDK.devnetTrusted('paloma');
await sdk.connect();

// Non-trusted devnet — explicit addresses required (no quorum context).
const local = EvoSDK.devnet('paloma', {
  addresses: ['https://10.0.0.5:1443'],
});
await local.connect();
```

Static helpers are also exported:

- `await EvoSDK.setLogLevel(filter)` — configure the underlying Wasm SDK's tracing globally.
- `await EvoSDK.getLatestVersionNumber()` — return the latest Platform protocol version supported by the bundled Wasm SDK.
- `await EvoSDK.maxRankedLimit()` — the hard ceiling on a [ranked / having-range](#ranked-queries) `limit`.
- `await EvoSDK.rankedAverageScale()` — the fixed-point divisor for the `avg` axis of a ranked / having-range result.

## Facades

The SDK organises its API into domain-specific facades, each accessible as a property on the `EvoSDK` instance:

| Facade | Description |
|--------|-------------|
| [`sdk.addresses`](src/addresses/facade.ts) | Query balances, transfer credits, withdraw to L1 |
| [`sdk.identities`](src/identities/facade.ts) | Fetch, create, update, and top up identities |
| [`sdk.documents`](src/documents/facade.ts) | Query, create, replace, delete, and transfer documents; aggregate `count` / `sum` / `average` over indexed fields; `ranked` top-K and `having` range queries over ranked indexes |
| [`sdk.contracts`](src/contracts/facade.ts) | Fetch, publish, and update data contracts |
| [`sdk.tokens`](src/tokens/facade.ts) | Mint, burn, transfer, freeze tokens and query balances |
| [`sdk.dpns`](src/dpns/facade.ts) | Register and resolve Dash Platform names |
| [`sdk.epoch`](src/epoch/facade.ts) | Query epoch information and evonode proposed blocks |
| [`sdk.protocol`](src/protocol/facade.ts) | Protocol version upgrade state and voting |
| [`sdk.stateTransitions`](src/state-transitions/facade.ts) | Broadcast and wait for state transitions |
| [`sdk.system`](src/system/facade.ts) | System status, quorum info, and total credits |
| [`sdk.group`](src/group/facade.ts) | Group membership, actions, and contested resources |
| [`sdk.voting`](src/voting/facade.ts) | Contested resource vote states and polls |
| [`sdk.shielded`](src/shielded/facade.ts) | Query shielded pool state, encrypted notes, anchors, and nullifier status |

A `wallet` namespace is also exported with utilities for BIP39 mnemonic generation and validation, BIP44/DIP9/DIP13 key derivation (path helpers included), extended-key conversion (`xprvToXpub`, `deriveChildPublicKey`), key-pair generation and import (`generateKeyPair`, `keyPairFromWif`, `keyPairFromHex`), public-key-to-address conversion, address validation, message signing, and Dashpay contact-key derivation. See [`src/wallet/functions.ts`](src/wallet/functions.ts) for the full list.

## Ranked queries

From protocol version 14, a contract index can declare `rankedCountable`, `rankedSummable` or `rankedAverageable`. Against such an index the SDK can answer "which groups score highest?" with a proof, in `O(log n + k)`, without walking every group:

```ts
// The three best restaurants by average grade.
const page = await sdk.documents.ranked({
  dataContractId: RESTAURANTS,
  documentTypeName: 'review',
  groupBy: 'restaurantId',
  aggregate: { type: 'avg', property: 'grade' },
  limit: 3,
});

for (const entry of page.entries) {
  // `value` is exact fixed point for the avg axis — divide by `page.valueScale`,
  // never by a hardcoded constant. `valueAsNumber` is a lossy display helper.
  console.log(entry.rank, entry.groupValue, Number(entry.value) / Number(page.valueScale));
}
```

`limit` is required and capped at `await EvoSDK.maxRankedLimit()` (a hard reject, not a clamp). `offset` skips ranks — `{ limit: 1, offset: 4 }` is "the 5th best" — and has no ceiling, because the skipped region is attested rather than walked.

`sdk.documents.having()` bounds the same axis by value instead of by position (`{ operator: '>', value: 100 }`), and `rankedWithProof` / `havingWithProof` return the proof and block metadata alongside the result.

## Document references (`refersTo`)

Also from protocol version 14, an identifier property can declare what it points at. This is a write-time consensus constraint — nothing resolves a reference for a reader — but a fetched contract can be asked what it declares:

```ts
const contract = await sdk.contracts.fetch(contractId);

for (const ref of contract.documentTypeReferences('note')) {
  // { path: 'author', type: 'identityPublicKey', keyIdProperty: 'authorKeyId' }
  console.log(ref.path, ref.type);
}

// Every document type that declares at least one reference.
contract.documentReferences;
```

Declarations are only parsed from protocol version 14 onward; a contract deserialized against an earlier version reports none even when its raw schema carries the keyword.

When a write is rejected because a reference does not resolve, the consensus code reaches JS as `error.code`:

```ts
import { DocumentReferenceErrorCode } from '@dashevo/evo-sdk';

try {
  await sdk.documents.create({ document, identityKey, signer });
} catch (e) {
  if (e.code === DocumentReferenceErrorCode.ReferencedIdentityKeyDisabled) {
    // the referenced key exists but was disabled
  }
}
```

## Contributing

Feel free to dive in! [Open an issue](https://github.com/dashpay/platform/issues/new/choose) or submit PRs.

## License

[MIT](LICENSE) &copy; Dash Core Group, Inc.
