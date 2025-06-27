# Web Service Provider Quick Start

This guide shows how to quickly get started with the web service provider for fetching platform state and quorum keys.

## Basic Setup

```typescript
import { createSDK } from '@dashpay/dash-sdk';

// The SDK uses web service provider by default
const sdk = createSDK({
  network: 'testnet'
});

await sdk.initialize();

// Get platform state
const provider = sdk.getContextProvider();
const blockHeight = await provider.getLatestPlatformBlockHeight();
console.log(`Current block height: ${blockHeight}`);
```

## Direct Provider Usage

For more control, use the provider directly:

```typescript
import { WebServiceProvider } from '@dashpay/dash-sdk/providers';

// Create provider
const provider = new WebServiceProvider({
  network: 'testnet',
  cacheDuration: 60000  // 1 minute cache
});

// Check availability
const isAvailable = await provider.isAvailable();
if (!isAvailable) {
  console.error('Web service is not available');
  return;
}

// Fetch platform state
const [height, time, version] = await Promise.all([
  provider.getLatestPlatformBlockHeight(),
  provider.getLatestPlatformBlockTime(),
  provider.getLatestPlatformVersion()
]);

console.log(`Block height: ${height}`);
console.log(`Block time: ${new Date(time).toISOString()}`);
console.log(`Platform version: ${version}`);
```

## Working with Quorum Keys

```typescript
// Fetch all quorum keys
const quorumKeys = await provider.getQuorumKeys();
console.log(`Total quorums: ${quorumKeys.size}`);

// Get specific quorum
const quorumHash = 'abc123...';
const quorum = await provider.getQuorum(quorumHash);
if (quorum) {
  console.log(`Quorum type: ${quorum.quorumPublicKey.type}`);
  console.log(`Public key: ${quorum.quorumPublicKey.publicKey}`);
}

// Get active quorums only
const activeQuorums = await provider.getActiveQuorums();
console.log(`Active quorums: ${activeQuorums.length}`);
```

## Priority-Based Fallback

Set up multiple providers with automatic fallback:

```typescript
import { ProviderFactory } from '@dashpay/dash-sdk/providers';

// Create provider with fallback
const provider = await ProviderFactory.createWithWebService({
  network: 'testnet',
  webservice: {
    cacheDuration: 30000,
    retryAttempts: 2
  }
});

// SDK will automatically fall back to centralized provider if web service fails
const sdk = createSDK({
  network: 'testnet',
  contextProvider: provider
});
```

## Error Handling

```typescript
try {
  const quorumKeys = await provider.getQuorumKeys();
  // Process quorum keys
} catch (error) {
  if (error.message.includes('HTTP')) {
    console.error('Network error:', error.message);
  } else if (error.message.includes('timeout')) {
    console.error('Request timed out');
  } else {
    console.error('Unknown error:', error);
  }
}
```

## Monitoring Provider Events

```typescript
import { ProviderFactory } from '@dashpay/dash-sdk/providers';

const provider = await ProviderFactory.createWithWebService({
  network: 'testnet'
});

// Monitor events
provider.on('provider:used', (name, method) => {
  console.log(`Used ${name} for ${method}`);
});

provider.on('provider:error', (name, error) => {
  console.error(`Provider ${name} failed:`, error.message);
});

provider.on('provider:fallback', (from, to) => {
  console.log(`Falling back from ${from} to ${to}`);
});
```

## Performance Tips

1. **Enable Caching**: Cache responses to reduce network calls
   ```typescript
   const provider = new WebServiceProvider({
     cacheDuration: 300000  // 5 minutes
   });
   ```

2. **Batch Requests**: Fetch multiple values together
   ```typescript
   const [height, time, keys] = await Promise.all([
     provider.getLatestPlatformBlockHeight(),
     provider.getLatestPlatformBlockTime(),
     provider.getQuorumKeys()
   ]);
   ```

3. **Handle Failures Gracefully**: Use fallback providers
   ```typescript
   const provider = await ProviderFactory.create({
     providers: ['webservice', 'centralized'],
     usePriority: true,
     fallbackEnabled: true
   });
   ```

## Next Steps

- Read the full [Provider Guide](./providers.md) for advanced usage
- Check out [example code](../examples/webservice-quorum.ts)
- Learn about [Bluetooth provider](./bluetooth-guide.md) for mobile signing
- Explore the [API Reference](./api/providers.md)