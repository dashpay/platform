# Getting Started

## Installation

```sh
npm install @dashevo/evo-sdk
```

The package is **ESM-only** (`"type": "module"`) and written in TypeScript with
full type definitions included. In CommonJS projects use a dynamic `import()`:

```js
const { EvoSDK } = await import('@dashevo/evo-sdk');
```

Requirements: Node.js ≥ 18.18 or any modern browser with WebAssembly support.

## Quick start

```typescript
import { EvoSDK } from '@dashevo/evo-sdk';

// Pick your network: testnetTrusted() for development, mainnetTrusted() for production
const sdk = EvoSDK.testnetTrusted();

// Query the current epoch (connect() is called automatically on first use)
const epoch = await sdk.epoch.current();
console.log('Current epoch:', epoch.index);

// Fetch an existing identity by its base58 ID
const identity = await sdk.identities.fetch('4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF');
console.log('Balance:', identity?.getBalance());
```

## Connecting

Calling `connect()` explicitly is **optional**. The SDK connects automatically
when you call any facade method for the first time. However, you can call
`connect()` explicitly if you want to control when the WASM module is
initialized and quorum keys are prefetched:

```typescript
const sdk = EvoSDK.testnetTrusted();
await sdk.connect(); // optional — triggers WASM init and quorum prefetch now
```

Calling `connect()` more than once is a no-op.

### Factory helpers

| Helper | Equivalent |
|--------|-----------|
| `EvoSDK.testnet()` | `new EvoSDK({ network: 'testnet' })` |
| `EvoSDK.mainnet()` | `new EvoSDK({ network: 'mainnet' })` |
| `EvoSDK.testnetTrusted()` | `new EvoSDK({ network: 'testnet', trusted: true })` |
| `EvoSDK.mainnetTrusted()` | `new EvoSDK({ network: 'mainnet', trusted: true })` |
| `EvoSDK.local()` | `new EvoSDK({ network: 'local' })` |
| `EvoSDK.localTrusted()` | `new EvoSDK({ network: 'local', trusted: true })` |

### Custom addresses

To connect to specific masternodes (useful for testing or private networks):

```typescript
const sdk = EvoSDK.withAddresses(
  ['https://52.12.176.90:1443'],
  'testnet',
);
await sdk.connect();
```

## Connection options

All factory helpers and the constructor accept an optional `ConnectionOptions`
object:

```typescript
const sdk = EvoSDK.testnetTrusted({
  settings: {
    connectTimeoutMs: 5000,
    timeoutMs: 10000,
    retries: 3,
    banFailedAddress: true,
  },
  logs: 'info',         // 'off' | 'error' | 'warn' | 'info' | 'debug' | 'trace'
  proofs: true,          // request proofs with every query
  version: 8,           // pin a specific protocol version
});
```

### Logging

The SDK delegates logging to the underlying Rust/WASM layer. Pass a simple
level string or a full `EnvFilter` directive:

```typescript
// Simple level
const sdk = EvoSDK.testnetTrusted({ logs: 'debug' });

// Granular filter
const sdk = EvoSDK.testnetTrusted({ logs: 'wasm_sdk=debug,rs_dapi_client=warn' });

// Change level at runtime (static, affects all instances)
await EvoSDK.setLogLevel('trace');
```
