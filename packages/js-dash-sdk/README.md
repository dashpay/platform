# Dash SDK

A modular JavaScript/TypeScript SDK for interacting with Dash Platform, built on WebAssembly for optimal performance and minimal bundle size.

## Features

- 🚀 **WebAssembly-based** - Built on top of the Rust implementation for maximum performance
- 📦 **Modular architecture** - Import only what you need for smaller bundle sizes
- 🔒 **Type-safe** - Full TypeScript support with comprehensive type definitions
- 🌐 **Multiple connectivity options** - Web service, Bluetooth, centralized service with automatic fallback
- 🎯 **Tree-shaking friendly** - ES modules support for optimal bundling
- 📱 **Mobile wallet support** - Bluetooth connectivity for secure mobile signing

## Installation

```bash
npm install dash
# or
yarn add dash
```

## Quick Start

```typescript
import { createSDK } from 'dash';

// Initialize SDK
const sdk = createSDK({
  network: 'testnet',
  // Optional: provide custom context provider
  // contextProvider: new CentralizedProvider({ url: 'https://your-provider.com' })
});

await sdk.initialize();

// Use the SDK
const identity = await sdk.identities.get('identityId...');
const contract = await sdk.contracts.get('contractId...');
```

## Modular Usage

Import only the modules you need:

```typescript
// Core only
import { SDK, CentralizedProvider } from 'dash/core';

// Individual modules
import { IdentityModule } from 'dash/identities';
import { ContractModule } from 'dash/contracts';
import { DocumentModule } from 'dash/documents';
import { NamesModule } from 'dash/names';

// Create SDK with only needed modules
const sdk = new SDK({ network: 'testnet' });
await sdk.initialize();

const identities = new IdentityModule(sdk);
const identity = await identities.get('...');
```

## API Reference

### Core

#### SDK Initialization

```typescript
const sdk = createSDK({
  network: 'mainnet' | 'testnet' | 'devnet',
  contextProvider?: ContextProvider,
  wallet?: WalletOptions,
  apps?: Record<string, AppDefinition>,
  retries?: number,
  timeout?: number
});

await sdk.initialize();
```

### Identities

```typescript
// Get identity
const identity = await sdk.identities.get(identityId);

// Get balance
const balance = await sdk.identities.getBalance(identityId);

// Update identity
await sdk.identities.update(identityId, {
  addKeys: [{ /* key definition */ }],
  disableKeys: [keyId]
});

// Credit operations
await sdk.identities.creditTransfer(identityId, {
  recipientId: '...',
  amount: 100000
});

await sdk.identities.creditWithdrawal(identityId, {
  amount: 50000,
  coreFeePerByte: 1,
  pooling: 'if-needed'
});
```

### Contracts

```typescript
// Create contract
const contract = await sdk.contracts.create({
  ownerId: identityId,
  schema: {},
  documentSchemas: {
    myDocument: {
      type: 'object',
      properties: {
        name: { type: 'string' },
        age: { type: 'integer' }
      },
      required: ['name']
    }
  }
});

// Publish contract
await sdk.contracts.publish(contract);

// Get contract
const existingContract = await sdk.contracts.get(contractId);

// Get contract history
const history = await sdk.contracts.getHistory(contractId);
```

### Documents

```typescript
// Create document
const document = await sdk.documents.create(
  contractId,
  ownerId,
  'myDocument',
  { name: 'Alice', age: 30 }
);

// Query documents
const documents = await sdk.documents.query({
  dataContractId: contractId,
  type: 'myDocument',
  where: [
    ['>=', 'age', 25],
    ['<=', 'age', 35]
  ],
  orderBy: [['age', 'desc']],
  limit: 10
});

// Batch operations
await sdk.documents.broadcast(contractId, ownerId, {
  create: [
    { type: 'myDocument', data: { name: 'Bob', age: 25 } }
  ],
  replace: [
    { id: documentId, type: 'myDocument', data: { name: 'Alice', age: 31 }, revision: 2 }
  ],
  delete: [
    { id: otherDocumentId, type: 'myDocument' }
  ]
});
```

### Names (DPNS)

```typescript
// Register name
await sdk.names.register({
  label: 'myname',
  ownerId: identityId,
  records: {
    dashUniqueIdentityId: identityId
  }
});

// Resolve name
const name = await sdk.names.resolve('myname');

// Search names
const names = await sdk.names.search('prefix', {
  limit: 25
});

// Update name records
await sdk.names.update('myname', ownerId, {
  dashUniqueIdentityId: newIdentityId
});
```

## Network Configuration

The SDK supports multiple networks:

```typescript
// Mainnet
const sdk = createSDK({ network: 'mainnet' });

// Testnet (default)
const sdk = createSDK({ network: 'testnet' });

// Custom network
const sdk = createSDK({
  network: { name: 'local', type: 'devnet' },
  contextProvider: new CentralizedProvider({
    url: 'http://localhost:3000'
  })
});
```

## Context Providers

Context providers supply network state information. The SDK includes multiple providers with automatic fallback support:

### Available Providers

- **WebServiceProvider** - Connects to quorum service endpoints for state and quorum keys
- **BluetoothProvider** - Mobile device connection for state and signing
- **CentralizedProvider** - Basic HTTP provider (fallback)
- **PriorityContextProvider** - Manages multiple providers with automatic fallback

### Default Configuration

By default, the SDK uses priority-based provider selection:

```typescript
// Default setup: WebService → Centralized fallback
const sdk = createSDK({ network: 'testnet' });
```

### Web Service Provider

Connects to Dash quorum service endpoints:

```typescript
import { WebServiceProvider } from 'dash/providers';

const provider = new WebServiceProvider({
  network: 'mainnet',              // or 'testnet'
  cacheDuration: 60000,            // Cache for 1 minute
  retryAttempts: 3,                // Retry failed requests
  timeout: 30000                   // 30 second timeout
});

// Get quorum keys
const quorumKeys = await provider.getQuorumKeys();
```

### Bluetooth Provider

Connect to mobile wallets for signing:

```typescript
import { BluetoothProvider } from 'dash/bluetooth';

const provider = new BluetoothProvider({
  requireAuthentication: true,
  autoReconnect: true
});

await provider.connect();
```

### Priority Provider

Use multiple providers with automatic fallback:

```typescript
import { ProviderFactory } from 'dash/providers';

// Bluetooth priority with web service fallback
const provider = await ProviderFactory.createWithBluetooth({
  network: 'testnet',
  bluetooth: {
    requireAuthentication: true
  },
  webservice: {
    cacheDuration: 60000
  },
  fallbackEnabled: true
});

// Monitor provider usage
provider.on('provider:used', (name, method) => {
  console.log(`${name} used for ${method}`);
});

provider.on('provider:fallback', (from, to) => {
  console.log(`Fallback from ${from} to ${to}`);
});
```

### Hybrid Setup

Use different providers for different operations:

```typescript
// Web service for state, Bluetooth for signing
const sdk = createSDK({
  network: 'testnet',
  contextProvider: webServiceProvider,  // For platform state
  wallet: {
    bluetooth: true                     // For transaction signing
  }
});
```

For detailed provider documentation, see the [Provider Guide](./docs/providers.md).

## Error Handling

The SDK provides typed errors for better error handling:

```typescript
import { 
  NotFoundError, 
  InsufficientBalanceError,
  StateTransitionError 
} from 'dash/utils';

try {
  await sdk.identities.get('...');
} catch (error) {
  if (error instanceof NotFoundError) {
    console.log('Identity not found');
  } else if (error instanceof StateTransitionError) {
    console.log('State transition failed:', error.code);
  }
}
```

## License

MIT