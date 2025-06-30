# Context Providers Guide

This guide explains how to use context providers in the Dash SDK, including the web service provider and priority-based provider system.

## Overview

Context providers supply platform state information to the SDK. The SDK supports multiple provider types:

- **WebServiceProvider**: Fetches state and quorum keys from web service endpoints
- **CentralizedProvider**: Basic HTTP provider for platform state
- **BluetoothProvider**: Mobile device connection for state and signing
- **PriorityContextProvider**: Manages multiple providers with automatic fallback

## Web Service Provider

The WebServiceProvider connects to Dash quorum service endpoints to fetch platform state and quorum keys.

### Configuration

```typescript
import { WebServiceProvider } from '@dashpay/dash-sdk/providers';

// Default configuration (testnet)
const provider = new WebServiceProvider();

// Custom configuration
const provider = new WebServiceProvider({
  network: 'mainnet',              // 'mainnet' | 'testnet'
  url: 'https://custom.service',   // Custom endpoint URL
  timeout: 30000,                  // Request timeout in ms
  retryAttempts: 3,                // Number of retry attempts
  retryDelay: 1000,                // Initial retry delay in ms
  cacheDuration: 60000,            // Cache duration in ms
  headers: {                       // Custom HTTP headers
    'X-API-Key': 'your-key'
  }
});
```

### Endpoints

The provider uses these default endpoints:

- **Mainnet**: `https://quorum.networks.dash.org`
- **Testnet**: `https://quorum.testnet.networks.dash.org`

### Features

#### Platform State

```typescript
// Get latest platform block height
const height = await provider.getLatestPlatformBlockHeight();

// Get latest platform block time
const time = await provider.getLatestPlatformBlockTime();

// Get core chain locked height
const coreHeight = await provider.getLatestPlatformCoreChainLockedHeight();

// Get platform version
const version = await provider.getLatestPlatformVersion();

// Check if provider is available
const isAvailable = await provider.isAvailable();
```

#### Quorum Keys

```typescript
// Get all quorum keys
const quorumKeys = await provider.getQuorumKeys();
// Returns: Map<string, QuorumInfo>

// Get specific quorum
const quorum = await provider.getQuorum('quorumHash');
// Returns: QuorumInfo | null

// Get active quorums
const activeQuorums = await provider.getActiveQuorums();
// Returns: QuorumInfo[]
```

### Response Format

The service returns quorum data in this format:

```json
{
  "quorumHash1": {
    "publicKey": "base64EncodedPublicKey",
    "version": 1,
    "type": "BLS"
  },
  "quorumHash2": {
    "publicKey": "anotherPublicKey",
    "version": 2,
    "type": "ECDSA"
  }
}
```

## Priority Context Provider

The PriorityContextProvider manages multiple providers and automatically falls back to lower-priority providers when higher-priority ones fail.

### Basic Usage

```typescript
import { PriorityContextProvider } from '@dashpay/dash-sdk/providers';

const priorityProvider = new PriorityContextProvider({
  providers: [
    {
      provider: bluetoothProvider,
      priority: 100,              // Highest priority
      name: 'Bluetooth'
    },
    {
      provider: webServiceProvider,
      priority: 80,
      name: 'WebService'
    },
    {
      provider: centralizedProvider,
      priority: 50,               // Lowest priority
      name: 'Centralized'
    }
  ],
  fallbackEnabled: true,          // Enable automatic fallback
  cacheResults: true,             // Cache successful responses
  logErrors: true                 // Log provider errors
});
```

### Provider Factory

Use the ProviderFactory for simplified provider creation:

```typescript
import { ProviderFactory } from '@dashpay/dash-sdk/providers';

// Create with Bluetooth priority
const provider = await ProviderFactory.createWithBluetooth({
  network: 'testnet',
  bluetooth: {
    requireAuthentication: true,
    autoReconnect: true
  },
  webservice: {
    cacheDuration: 60000
  },
  fallbackEnabled: true
});

// Create with Web Service priority
const provider = await ProviderFactory.createWithWebService({
  network: 'mainnet',
  webservice: {
    url: 'https://custom-quorum.service'
  }
});

// Create custom provider configuration
const provider = await ProviderFactory.create({
  providers: ['webservice', 'centralized'],
  network: 'testnet',
  usePriority: true,
  priorityOptions: {
    fallbackEnabled: true,
    cacheResults: true
  }
});
```

### Events

The priority provider emits events for monitoring:

```typescript
// Provider used successfully
provider.on('provider:used', (name: string, method: string) => {
  console.log(`${name} used for ${method}`);
});

// Provider error
provider.on('provider:error', (name: string, error: Error) => {
  console.log(`${name} failed: ${error.message}`);
});

// Fallback occurred
provider.on('provider:fallback', (from: string, to: string) => {
  console.log(`Falling back from ${from} to ${to}`);
});

// All providers failed
provider.on('all:failed', (method: string, errors: Map<string, Error>) => {
  console.log(`All providers failed for ${method}`);
});
```

### Metrics

Track provider performance:

```typescript
const metrics = provider.getMetrics();
// Returns: Map<string, ProviderMetrics>

for (const [name, stats] of metrics) {
  console.log(`${name}:`);
  console.log(`  Success: ${stats.successCount}`);
  console.log(`  Errors: ${stats.errorCount}`);
  console.log(`  Avg Response: ${stats.averageResponseTime}ms`);
}
```

### Dynamic Provider Management

```typescript
// Add provider at runtime
provider.addProvider(
  new WebServiceProvider({ network: 'testnet' }),
  120,                    // Priority
  'BackupWebService'      // Name
);

// Remove provider
provider.removeProvider('BackupWebService');

// Get active provider
const activeProvider = await provider.getActiveProvider();
console.log(`Current provider: ${activeProvider?.name}`);

// Clear cache
provider.clearCache();
```

## SDK Integration

The SDK uses the priority provider by default:

```typescript
import { createSDK } from '@dashpay/dash-sdk';

// Default: WebService → Centralized fallback
const sdk = createSDK({
  network: 'testnet'
});

// Custom provider
const sdk = createSDK({
  network: 'testnet',
  contextProvider: customProvider
});
```

## Hybrid Setup

Use different providers for different operations:

```typescript
// Bluetooth for signing, Web Service for state
const contextProvider = await ProviderFactory.createWithWebService({
  network: 'testnet'
});

const sdk = createSDK({
  network: 'testnet',
  contextProvider: contextProvider,  // Web service for state
  wallet: {
    bluetooth: true                  // Bluetooth for signing
  }
});
```

## Best Practices

### 1. Provider Selection

Choose providers based on your use case:

- **Web Service**: Best for read-only operations and quorum key management
- **Bluetooth**: Required for signing operations and mobile wallet integration
- **Centralized**: Simple fallback for basic platform state

### 2. Error Handling

Always handle provider failures:

```typescript
try {
  const height = await provider.getLatestPlatformBlockHeight();
} catch (error) {
  if (error.message.includes('All providers failed')) {
    // Handle complete failure
  } else {
    // Handle specific error
  }
}
```

### 3. Caching

Configure caching based on your needs:

```typescript
const provider = new WebServiceProvider({
  cacheDuration: 5000     // Short cache for real-time apps
});

const provider = new WebServiceProvider({
  cacheDuration: 300000   // Long cache for less frequent updates
});
```

### 4. Monitoring

Monitor provider health in production:

```typescript
// Check availability periodically
setInterval(async () => {
  const available = await provider.isAvailable();
  if (!available) {
    console.warn('Provider unavailable');
  }
}, 60000);

// Track metrics
provider.on('provider:used', (name, method) => {
  telemetry.track('provider.used', { name, method });
});
```

### 5. Network Configuration

Always specify the correct network:

```typescript
// Production
const provider = new WebServiceProvider({ network: 'mainnet' });

// Development
const provider = new WebServiceProvider({ network: 'testnet' });
```

## Troubleshooting

### Provider Not Available

If the web service is not available:

1. Check network connectivity
2. Verify the service endpoint is correct
3. Check if the service is operational
4. Enable fallback to use alternative providers

### Slow Response Times

To improve performance:

1. Enable caching with appropriate duration
2. Use priority providers to try fastest first
3. Adjust timeout values
4. Consider using multiple providers in parallel

### Authentication Issues

For Bluetooth providers:

1. Ensure device pairing is complete
2. Check authentication requirements
3. Verify security credentials
4. Enable auto-reconnect for stability

## Examples

See the [examples directory](../examples/) for complete working examples:

- `webservice-quorum.ts`: Web service provider usage
- `priority-providers.ts`: Priority provider configuration
- `bluetooth-connection.ts`: Bluetooth provider setup