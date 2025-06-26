# Dash Platform WASM SDK

A comprehensive WebAssembly SDK for interacting with Dash Platform from browser environments. This SDK provides full access to Dash Platform features including identity management, document operations, state transitions, and real-time monitoring.

## Features

- 🌐 **Full browser compatibility** - Works in any modern web browser
- 🔐 **Complete identity management** - Create, fund, and manage identities
- 📄 **Document operations** - Create, update, delete, and query documents
- 🔄 **State transitions** - Full support for all platform state transitions
- 📡 **Real-time subscriptions** - WebSocket support for live updates
- 🔑 **BIP39 mnemonic support** - HD wallet derivation and key management
- 📊 **Performance monitoring** - Built-in metrics and health checks
- 💾 **Smart caching** - Automatic caching for improved performance
- 🛡️ **Proof verification** - Cryptographic proof validation
- 🔒 **Browser crypto integration** - Native Web Crypto API support

## Installation

```bash
npm install @dashevo/wasm-sdk
```

Or include directly in your HTML:

```html
<script type="module">
  import init, { WasmSdk } from './wasm_sdk.js';
  await init();
</script>
```

## Quick Start

### Initialize the SDK

```javascript
import init, { start, WasmSdk } from '@dashevo/wasm-sdk';

// Initialize the WASM module
await init();
await start();

// Create SDK instance
const sdk = new WasmSdk('testnet'); // or 'mainnet'
```

## Core Features

### Identity Management

```javascript
import { 
  getIdentityInfo, 
  getIdentityBalance,
  checkIdentityExists,
  topUpIdentity,
  WasmSigner 
} from '@dashevo/wasm-sdk';

// Get identity information
const info = await getIdentityInfo(sdk, 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
console.log(`Balance: ${info.balance}, Revision: ${info.revision}`);

// Check if identity exists
const exists = await checkIdentityExists(sdk, identityId);

// Top up identity balance
const signer = new WasmSigner();
signer.setIdentityId(fundingIdentityId);
signer.addPrivateKey(keyId, privateKeyBytes, 'ECDSA_SECP256K1', 0);

await topUpIdentity(sdk, identityId, 10000000, signer); // 0.1 DASH
```

### Document Operations

```javascript
import { 
  createDocument,
  updateDocument,
  deleteDocument,
  DocumentQuery 
} from '@dashevo/wasm-sdk';

// Create a document
const doc = await createDocument(
  sdk,
  contractId,
  identityId,
  'profile',
  {
    displayName: 'Alice',
    bio: 'Dash Platform developer'
  },
  signer
);

// Query documents
const query = new DocumentQuery(contractId, 'profile');
query.where('age', '>', 18);
query.orderBy('createdAt', 'desc');
query.limit(10);

const results = await sdk.platform.documents.get(query);
```

### BIP39 Mnemonic & HD Keys

```javascript
import { 
  Mnemonic, 
  MnemonicStrength,
  WordListLanguage,
  deriveChildKey 
} from '@dashevo/wasm-sdk';

// Generate new mnemonic
const mnemonic = Mnemonic.generate(MnemonicStrength.Words24, WordListLanguage.English);
console.log(`Mnemonic: ${mnemonic.phrase()}`);

// Create from existing phrase
const restored = Mnemonic.fromPhrase(
  "abandon ability able about above absent absorb abstract absurd abuse access accident",
  WordListLanguage.English
);

// Derive HD keys
const seed = mnemonic.toSeed("optional passphrase");
const hdKey = mnemonic.toHDPrivateKey("optional passphrase", "testnet");

// Derive specific keys for identity
const authKey = await deriveChildKey(
  mnemonic.phrase(),
  "passphrase",
  "m/9'/5'/3'/0/0", // Authentication key path
  "testnet"
);
```

### Real-time Subscriptions

```javascript
import { SubscriptionClient } from '@dashevo/wasm-sdk';

// Create subscription client
const subClient = new SubscriptionClient('testnet');

// Subscribe to document updates
const subscriptionId = await subClient.subscribeToDocuments(
  contractId,
  'profile',
  (update) => {
    console.log('Document updated:', update);
  }
);

// Subscribe to identity updates
await subClient.subscribeToIdentity(
  identityId,
  (update) => {
    console.log('Identity updated:', update);
  }
);

// Unsubscribe when done
await subClient.unsubscribe(subscriptionId);
```

### Performance Monitoring

```javascript
import { 
  initializeMonitoring, 
  getGlobalMonitor,
  performHealthCheck 
} from '@dashevo/wasm-sdk';

// Initialize monitoring
await initializeMonitoring(true, 1000); // max 1000 metrics

// Track operations
const monitor = await getGlobalMonitor();
monitor.startOperation('fetch_1', 'FetchIdentity');
// ... perform operation
monitor.endOperation('fetch_1', true, null);

// Get statistics
const stats = await monitor.getOperationStats();
console.log('Operation stats:', stats);

// Health check
const health = await performHealthCheck(sdk);
console.log(`System health: ${health.status}`);
```

### Contract History & Migration

```javascript
import { 
  getContractHistory,
  getSchemaChanges,
  getMigrationGuide 
} from '@dashevo/wasm-sdk';

// Get contract version history
const history = await getContractHistory(sdk, contractId);

// Compare schema changes
const changes = await getSchemaChanges(sdk, contractId, 1, 2);

// Get migration guide
const guide = await getMigrationGuide(sdk, contractId, 1, 2);
console.log('Migration guide:', guide);
```

### Advanced Features

#### Prefunded Specialized Balances

```javascript
import { 
  topUpIdentity,
  transferCredits,
  batchTopUp 
} from '@dashevo/wasm-sdk';

// Transfer credits between identities
await transferCredits(
  sdk,
  fromIdentityId,
  toIdentityId,
  1000000, // credits
  signer
);

// Batch top up multiple identities
const identityIds = ['id1', 'id2', 'id3'];
await batchTopUp(sdk, fundingIdentityId, identityIds, 1000000, signer);
```

#### Browser Crypto Integration

```javascript
import { BrowserSigner } from '@dashevo/wasm-sdk';

const browserSigner = new BrowserSigner();

// Generate key pair using Web Crypto API
const publicKey = await browserSigner.generateKeyPair('ECDSA_SECP256K1', 1);

// Sign data with browser-stored key
const signature = await browserSigner.signWithStoredKey(data, 1);
```

## Error Handling

The SDK provides comprehensive error handling with categorized errors:

```javascript
try {
  // SDK operations
} catch (error) {
  if (error.name === 'DapiClientError') {
    // Network or API errors
  } else if (error.name === 'StateTransitionError') {
    // State transition validation errors
  } else if (error.name === 'ProofVerificationError') {
    // Cryptographic proof errors
  }
}
```

## Configuration

### SDK Configuration

```javascript
const sdk = new WasmSdk('testnet', {
  dapiAddresses: [
    'https://testnet-1.dash.org:443',
    'https://testnet-2.dash.org:443'
  ],
  timeout: 30000,
  retries: 3,
  cacheEnabled: true,
  monitoringEnabled: true
});
```

### DAPI Client Configuration

```javascript
import { DapiClient, DapiClientConfig } from '@dashevo/wasm-sdk';

const config = new DapiClientConfig('testnet');
config.setTimeout(5000);
config.setRetries(3);
config.addAddress('https://custom-node.dash.org:443');

const client = new DapiClient(config);
```

## Testing

The SDK includes comprehensive test suites:

```bash
# Run all tests
npm test

# Run unit tests only
npm run test:unit

# Run integration tests
npm run test:integration

# Run specific test suite
npm run test -- --test monitoring_tests

# Run tests with coverage
npm run test:coverage
```

## Performance Optimization

The SDK includes several optimization features:

1. **Automatic Caching** - Frequently accessed data is cached
2. **Connection Pooling** - Reuses WebSocket connections
3. **Batch Operations** - Group multiple operations for efficiency
4. **Lazy Loading** - Load only what's needed

See the [Optimization Guide](./OPTIMIZATION_GUIDE.md) for details.

## Examples

Check out the [examples directory](./examples/) for complete working examples:

- [Identity Creation](./examples/identity-creation-example.js)
- [Document Operations](./examples/state-transition-example.js)
- [BLS Signatures](./examples/bls-signatures-example.js)
- [Contract Caching](./examples/contract-cache-example.js)
- [Group Actions](./examples/group-actions-example.js)

## API Reference

Complete API documentation:

- [API Reference](./API_REFERENCE.md) - Detailed API documentation
- [TypeScript Definitions](./wasm-sdk.d.ts) - TypeScript type definitions
- [Usage Examples](./USAGE_EXAMPLES.md) - Common usage patterns

## Troubleshooting

### Common Issues

1. **WASM not loading**: Ensure your web server serves `.wasm` files with `application/wasm` MIME type
2. **Network errors**: Check CORS settings and network connectivity
3. **Memory issues**: Monitor browser memory usage, use cleanup methods

### Debug Mode

Enable debug logging:

```javascript
// Enable debug mode
window.WASM_SDK_DEBUG = true;

// Or use environment variable
process.env.WASM_SDK_DEBUG = 'true';
```

## Contributing

We welcome contributions! Please see our [Contributing Guide](../../CONTRIBUTING.md) for details.

### Development Setup

```bash
# Clone the repository
git clone https://github.com/dashpay/platform.git
cd platform/packages/wasm-sdk

# Install dependencies
npm install

# Build development version
npm run build:dev

# Watch for changes
npm run watch
```

## Security

- All cryptographic operations use standard, audited libraries
- Private keys never leave the browser
- WebSocket connections use TLS
- See [Security Policy](../../SECURITY.md) for reporting vulnerabilities

## License

MIT License - see [LICENSE](../../LICENSE) for details

## Support

- [Documentation](https://docs.dash.org/projects/platform)
- [Discord](https://discord.gg/dash)
- [GitHub Issues](https://github.com/dashpay/platform/issues)