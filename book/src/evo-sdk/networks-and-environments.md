# Networks and Environments

The Evo SDK supports three built-in network configurations plus custom
addresses for private or development networks.

## Built-in networks

| Network | Factory | DAPI discovery | Use case |
|---------|---------|---------------|----------|
| **Testnet** | `EvoSDK.testnetTrusted()` | Automatic via seed nodes | Development and testing |
| **Mainnet** | `EvoSDK.mainnetTrusted()` | Automatic via seed nodes | Production applications |
| **Local** | `EvoSDK.localTrusted()` | `127.0.0.1:1443` | Docker-based local development |

For each network, the SDK discovers DAPI endpoints from seed nodes and rotates
between them automatically. Failed nodes are temporarily banned so the SDK
retries against healthy nodes.

## Local development with Docker

When running a local Platform network via
[dashmate](https://github.com/dashpay/platform/tree/master/packages/dashmate),
use the `local` network:

```typescript
const sdk = EvoSDK.localTrusted();
await sdk.connect();
```

This connects to `https://127.0.0.1:1443` by default. If your local setup uses
different ports, use custom addresses:

```typescript
const sdk = EvoSDK.withAddresses(
  ['https://127.0.0.1:2443'],
  'local',
);
await sdk.connect();
```

## Custom masternode addresses

For private devnets, specific nodes, or debugging:

```typescript
const sdk = EvoSDK.withAddresses(
  [
    'https://52.12.176.90:1443',
    'https://34.217.100.50:1443',
  ],
  'testnet',
);
await sdk.connect();
```

When custom addresses are provided, the SDK does not perform automatic node
discovery — it uses only the addresses you supply.

## Browser vs Node.js

The SDK works identically in both environments. The underlying WASM module
handles platform differences transparently.

**Node.js considerations:**

- Requires Node.js ≥ 18.18 (for WebAssembly and `fetch` support)
- ESM-only package — use `import`, not `require`
- No additional polyfills needed

**Browser considerations:**

- Works in any browser with WebAssembly support (all modern browsers)
- The WASM module is loaded asynchronously on first `connect()` call
- Total bundle size includes the compiled Rust SDK (~2-4 MB gzipped)
- gRPC calls use `grpc-web` over HTTPS, compatible with standard CORS
