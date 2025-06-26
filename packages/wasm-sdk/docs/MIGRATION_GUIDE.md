# Migration Guide

This guide helps developers migrate from other Dash Platform SDKs to the WASM SDK.

## Table of Contents

- [Migrating from dash-sdk](#migrating-from-dash-sdk)
- [Migrating from dapi-client](#migrating-from-dapi-client)
- [Key Differences](#key-differences)
- [Common Migration Patterns](#common-migration-patterns)
- [Breaking Changes](#breaking-changes)

## Migrating from dash-sdk

### Before (dash-sdk)

```javascript
const Dash = require('dash');

const client = new Dash.Client({
  network: 'testnet',
  wallet: {
    mnemonic: 'your mnemonic here',
  },
});

// Get identity
const identity = await client.platform.identities.get('identityId');

// Create document
const document = await client.platform.documents.create(
  'dpns.domain',
  identity,
  {
    label: 'my-name',
    normalizedLabel: 'my-name',
    normalizedParentDomainName: 'dash',
    preorderSalt: Buffer.from('salt'),
    records: {
      dashUniqueIdentityId: identity.getId(),
    },
  },
);
```

### After (wasm-sdk)

```javascript
import { WasmSdk, WasmSigner, createDocument } from '@dashevo/wasm-sdk';

const sdk = new WasmSdk('testnet');
const signer = new WasmSigner();

// Set up signer
signer.setIdentityId(identityId);
signer.addPrivateKey(keyId, privateKeyBytes, 'ECDSA_SECP256K1', 0);

// Get identity
const identity = await getIdentityInfo(sdk, identityId);

// Create document
const doc = await createDocument(
  sdk,
  'dpns-contract-id',
  identityId,
  'domain',
  {
    label: 'my-name',
    normalizedLabel: 'my-name',
    normalizedParentDomainName: 'dash',
    preorderSalt: 'salt',
    records: {
      dashUniqueIdentityId: identityId,
    },
  },
  signer
);
```

### Key Changes

1. **Initialization**: No wallet configuration in constructor
2. **Signing**: Explicit signer setup required
3. **Async everywhere**: All operations are async
4. **Modular imports**: Import only what you need
5. **Binary data**: Use Uint8Array instead of Buffer

## Migrating from dapi-client

### Before (dapi-client)

```javascript
const DAPIClient = require('@dashevo/dapi-client');

const client = new DAPIClient({
  seeds: ['seed1.testnet.networks.dash.org'],
  network: 'testnet',
});

// Get identity
const response = await client.platform.getIdentity(identityId);
const identity = Identity.fromBuffer(response.identity);

// Broadcast state transition
const result = await client.platform.broadcastStateTransition(
  stateTransition.toBuffer()
);
```

### After (wasm-sdk)

```javascript
import { DapiClient, DapiClientConfig } from '@dashevo/wasm-sdk';

const config = new DapiClientConfig('testnet');
const client = new DapiClient(config);

// Get identity
const identity = await client.getIdentity(identityId);

// Broadcast state transition
const result = await client.broadcastStateTransition(stateTransitionBytes);
```

### Key Changes

1. **Configuration**: Use DapiClientConfig class
2. **No protobuf**: Direct JSON responses
3. **Simplified API**: Methods return parsed data
4. **WebSocket support**: Built-in subscription support

## Key Differences

### 1. Transport Layer

**Old SDKs**: Use gRPC for communication
**WASM SDK**: Uses HTTP/WebSocket for browser compatibility

### 2. Cryptography

**Old SDKs**: Node.js crypto libraries
**WASM SDK**: WebAssembly crypto + Web Crypto API

### 3. State Transition Creation

**Old SDKs**:
```javascript
const stateTransition = identityTopUpTransition.sign(
  identity,
  privateKey
);
```

**WASM SDK**:
```javascript
const stateTransition = await createIdentityTopUpTransition(
  sdk,
  identityId,
  amount,
  signer
);
```

### 4. Error Handling

**Old SDKs**:
```javascript
try {
  await client.platform.identities.get(id);
} catch (e) {
  if (e.code === 5) { // NOT_FOUND
    // Handle not found
  }
}
```

**WASM SDK**:
```javascript
try {
  await getIdentityInfo(sdk, id);
} catch (error) {
  if (error.name === 'DapiClientError' && error.code === 'NOT_FOUND') {
    // Handle not found
  }
}
```

## Common Migration Patterns

### Pattern 1: Identity Creation

**Old**:
```javascript
const identity = await client.platform.identities.register(
  assetLockProof,
  privateKey
);
```

**New**:
```javascript
const publicKeys = [{
  id: 0,
  type: 0, // ECDSA_SECP256K1
  purpose: 0, // AUTHENTICATION
  securityLevel: 0, // MASTER
  data: publicKeyBytes,
  readOnly: false
}];

const stateTransition = createIdentityStateTransition(
  assetLockProofBytes,
  publicKeys
);

await broadcastStateTransition(sdk, stateTransition);
```

### Pattern 2: Document Queries

**Old**:
```javascript
const documents = await client.platform.documents.get(
  'dpns.domain',
  {
    where: [
      ['normalizedParentDomainName', '==', 'dash'],
      ['normalizedLabel', '==', 'alice'],
    ],
  }
);
```

**New**:
```javascript
const query = new DocumentQuery('dpns-contract-id', 'domain');
query.where('normalizedParentDomainName', '==', 'dash');
query.where('normalizedLabel', '==', 'alice');

const documents = await sdk.platform.documents.get(query);
```

### Pattern 3: Wallet Integration

**Old**:
```javascript
const client = new Dash.Client({
  wallet: {
    mnemonic: 'your mnemonic',
    adapter: CustomAdapter,
  }
});
```

**New**:
```javascript
// Generate keys from mnemonic
const mnemonic = Mnemonic.fromPhrase(phrase, WordListLanguage.English);
const seed = mnemonic.toSeed(passphrase);

// Derive keys using BIP44 paths
const authKey = await deriveChildKey(
  mnemonic.phrase(),
  passphrase,
  "m/9'/5'/3'/0/0",
  network
);

// Set up signer
const signer = new WasmSigner();
signer.addPrivateKey(0, authKey.privateKey, 'ECDSA_SECP256K1', 0);
```

## Breaking Changes

### 1. No Automatic Signing

The WASM SDK requires explicit signing setup:

```javascript
// Must create and configure signer
const signer = new WasmSigner();
signer.setIdentityId(identityId);
signer.addPrivateKey(keyId, privateKey, keyType, purpose);
```

### 2. Binary Data Format

Use Uint8Array instead of Buffer:

```javascript
// Old
const data = Buffer.from('hello');

// New
const data = new TextEncoder().encode('hello');
```

### 3. No Built-in Wallet

The SDK doesn't include wallet functionality:

```javascript
// Implement your own wallet logic
class MyWallet {
  async getPrivateKey(keyId) {
    // Your implementation
  }
  
  async signData(data, keyId) {
    const privateKey = await this.getPrivateKey(keyId);
    return sign(data, privateKey);
  }
}
```

### 4. Async Module Initialization

Always initialize the WASM module before use:

```javascript
import init, { start, WasmSdk } from '@dashevo/wasm-sdk';

// Required initialization
await init();
await start();

// Now you can use the SDK
const sdk = new WasmSdk('testnet');
```

### 5. Different Default Networks

```javascript
// Old SDKs
new Dash.Client(); // defaults to 'evonet'

// WASM SDK
new WasmSdk(); // throws error - network required
new WasmSdk('testnet'); // explicit network
```

## Tips for Smooth Migration

1. **Start with initialization**: Get the WASM module loading working first
2. **Update data types**: Convert Buffer to Uint8Array throughout
3. **Implement signing**: Set up your signer before attempting operations
4. **Test error handling**: Error formats have changed
5. **Use TypeScript**: The SDK has comprehensive type definitions
6. **Enable monitoring**: Use built-in monitoring during migration to debug issues

## Need Help?

- Check the [API Documentation](./API_DOCUMENTATION.md)
- See [Usage Examples](../USAGE_EXAMPLES.md)
- Visit [GitHub Issues](https://github.com/dashpay/platform/issues)