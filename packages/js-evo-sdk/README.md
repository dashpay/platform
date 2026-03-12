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
console.log('Current epoch:', epoch);
```

## Facades

The SDK organises its API into domain-specific facades, each accessible as a property on the `EvoSDK` instance:

| Facade | Description |
|--------|-------------|
| [`sdk.addresses`](src/addresses/facade.ts) | Query balances, transfer credits, withdraw to L1 |
| [`sdk.identities`](src/identities/facade.ts) | Fetch, create, update, and top up identities |
| [`sdk.documents`](src/documents/facade.ts) | Query, create, replace, delete, and transfer documents |
| [`sdk.contracts`](src/contracts/facade.ts) | Fetch, publish, and update data contracts |
| [`sdk.tokens`](src/tokens/facade.ts) | Mint, burn, transfer, freeze tokens and query balances |
| [`sdk.dpns`](src/dpns/facade.ts) | Register and resolve Dash Platform names |
| [`sdk.epoch`](src/epoch/facade.ts) | Query epoch information and evonode proposed blocks |
| [`sdk.protocol`](src/protocol/facade.ts) | Protocol version upgrade state and voting |
| [`sdk.stateTransitions`](src/state-transitions/facade.ts) | Broadcast and wait for state transitions |
| [`sdk.system`](src/system/facade.ts) | System status, quorum info, and total credits |
| [`sdk.group`](src/group/facade.ts) | Group membership, actions, and contested resources |
| [`sdk.voting`](src/voting/facade.ts) | Contested resource vote states and polls |

A `wallet` namespace is also exported with utilities for mnemonic generation, key derivation, address validation, and message signing.

## Contributing

Feel free to dive in! [Open an issue](https://github.com/dashpay/platform/issues/new/choose) or submit PRs.

## License

[MIT](LICENSE) &copy; Dash Core Group, Inc.
