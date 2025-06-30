# Troubleshooting Guide

Common issues and solutions when using the Dash Platform WASM SDK.

## Table of Contents

- [Installation Issues](#installation-issues)
- [Initialization Problems](#initialization-problems)
- [Network Errors](#network-errors)
- [Signing Issues](#signing-issues)
- [Performance Problems](#performance-problems)
- [Browser Compatibility](#browser-compatibility)
- [Debugging Tips](#debugging-tips)

## Installation Issues

### WASM file not found

**Error**: `Failed to load WASM file`

**Solution**:
1. Ensure WASM files are copied to your public directory:
```json
// webpack.config.js
{
  plugins: [
    new CopyPlugin({
      patterns: [
        { from: 'node_modules/@dashevo/wasm-sdk/*.wasm', to: '[name][ext]' }
      ]
    })
  ]
}
```

2. Configure MIME type for WASM files:
```apache
# .htaccess
AddType application/wasm .wasm
```

### Module initialization fails

**Error**: `RuntimeError: unreachable`

**Solution**:
```javascript
// Always initialize before use
import init, { start } from '@dashevo/wasm-sdk';

async function initialize() {
  try {
    await init(); // Initialize WASM module
    await start(); // Initialize SDK runtime
  } catch (error) {
    console.error('Initialization failed:', error);
  }
}
```

## Initialization Problems

### Context provider not set

**Error**: `Context provider required for this operation`

**Solution**:
```javascript
import { WasmSdk, ContextProvider } from '@dashevo/wasm-sdk';

const contextProvider = new ContextProvider();
const sdk = new WasmSdk('testnet', contextProvider);

// Or set it later
sdk.setContextProvider(contextProvider);
```

### Invalid network

**Error**: `Invalid network: evonet`

**Solution**:
```javascript
// Use supported networks
const sdk = new WasmSdk('testnet'); // or 'mainnet'

// For custom networks
const config = new DapiClientConfig('custom');
config.addAddress('https://your-node.com:443');
```

## Network Errors

### CORS issues

**Error**: `Access to fetch at 'https://testnet.dash.org' from origin 'http://localhost:3000' has been blocked by CORS policy`

**Solution**:
1. Use a proxy in development:
```javascript
// vite.config.js
export default {
  server: {
    proxy: {
      '/api': {
        target: 'https://testnet.dash.org',
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/api/, '')
      }
    }
  }
}
```

2. Or configure CORS on your DAPI node

### Connection timeout

**Error**: `Request timeout after 30000ms`

**Solution**:
```javascript
// Increase timeout
const config = new DapiClientConfig('testnet');
config.setTimeout(60000); // 60 seconds
config.setRetries(5);

const client = new DapiClient(config);
```

### WebSocket connection failed

**Error**: `WebSocket connection to 'wss://...' failed`

**Solution**:
```javascript
// Check WebSocket support
if (!window.WebSocket) {
  console.error('WebSocket not supported');
  return;
}

// Handle connection errors
const subClient = new SubscriptionClient('testnet');
try {
  await subClient.connect();
} catch (error) {
  console.error('WebSocket connection failed:', error);
  // Fallback to polling
}
```

## Signing Issues

### Private key not found

**Error**: `Private key not found for ID: 1`

**Solution**:
```javascript
const signer = new WasmSigner();

// Ensure identity ID is set
signer.setIdentityId(identityId);

// Add private key before signing
signer.addPrivateKey(
  1, // key ID must match
  privateKeyBytes,
  'ECDSA_SECP256K1',
  0 // PURPOSE_AUTHENTICATION
);

// Check if key exists
if (!signer.hasKey(1)) {
  throw new Error('Key not added');
}
```

### Invalid signature

**Error**: `State transition signature verification failed`

**Solution**:
```javascript
// Ensure correct key type and purpose
const keyType = 'ECDSA_SECP256K1'; // or 'BLS12_381'
const purpose = 0; // AUTHENTICATION for state transitions

// For BLS signatures, ensure feature is enabled
if (keyType === 'BLS12_381') {
  // Check if BLS is available
  try {
    const sig = await signer.signData(data, keyId);
  } catch (error) {
    console.error('BLS signatures not available:', error);
  }
}
```

## Performance Problems

### Slow operations

**Problem**: Operations taking too long

**Solution**:
```javascript
// Enable caching
await initCache();

// Use batch operations
const identityIds = ['id1', 'id2', 'id3'];
const identities = await batchGetIdentities(sdk, identityIds);

// Monitor performance
initializeMonitoring(true, 1000);
const monitor = await getGlobalMonitor();

// Check operation stats
const stats = await monitor.getOperationStats();
console.log('Slowest operation:', stats);
```

### Memory leaks

**Problem**: Browser memory usage increasing

**Solution**:
```javascript
// Clear caches periodically
await cacheClear();

// Unsubscribe from unused subscriptions
await subscriptionClient.unsubscribeAll();

// Clear monitoring data
const monitor = await getGlobalMonitor();
await monitor.clearMetrics();

// Monitor memory usage
const usage = getResourceUsage();
console.log('Memory:', usage.memory);
```

## Browser Compatibility

### Web Crypto not available

**Error**: `crypto.subtle is undefined`

**Solution**:
```javascript
// Check for Web Crypto support
if (!window.crypto || !window.crypto.subtle) {
  console.error('Web Crypto API not available');
  // Use fallback or polyfill
}

// Ensure HTTPS in production
if (location.protocol !== 'https:' && location.hostname !== 'localhost') {
  console.warn('Web Crypto requires HTTPS');
}
```

### IndexedDB not available

**Error**: `IndexedDB not supported`

**Solution**:
```javascript
// Check IndexedDB support
if (!window.indexedDB) {
  console.warn('IndexedDB not available, caching disabled');
  // Use memory cache fallback
}

// Handle private browsing mode
try {
  await initCache();
} catch (error) {
  console.warn('Cache initialization failed:', error);
  // Continue without caching
}
```

## Debugging Tips

### Enable debug logging

```javascript
// Enable debug mode
window.WASM_SDK_DEBUG = true;

// Or use localStorage
localStorage.setItem('WASM_SDK_DEBUG', 'true');

// Custom logger
window.WASM_SDK_LOGGER = (level, message, data) => {
  console.log(`[${level}] ${message}`, data);
};
```

### Inspect WASM errors

```javascript
try {
  await someOperation();
} catch (error) {
  // Check error type
  console.log('Error name:', error.name);
  console.log('Error message:', error.message);
  console.log('Error stack:', error.stack);
  
  // WASM errors have additional properties
  if (error.category) {
    console.log('Error category:', error.category);
    console.log('Error code:', error.code);
    console.log('Error details:', error.details);
  }
}
```

### Monitor network requests

```javascript
// Intercept fetch requests
const originalFetch = window.fetch;
window.fetch = async (...args) => {
  console.log('Fetch:', args[0]);
  const response = await originalFetch(...args);
  console.log('Response:', response.status);
  return response;
};
```

### Profile performance

```javascript
// Use performance monitoring
const monitor = await getGlobalMonitor();

// Mark operation start
performance.mark('operation-start');

// Perform operation
await someExpensiveOperation();

// Mark operation end
performance.mark('operation-end');

// Measure
performance.measure('operation', 'operation-start', 'operation-end');
const measure = performance.getEntriesByName('operation')[0];
console.log(`Operation took ${measure.duration}ms`);
```

### Common error codes

| Error Code | Description | Solution |
|------------|-------------|----------|
| `NOT_FOUND` | Entity doesn't exist | Check ID is correct |
| `INVALID_ARGUMENT` | Invalid parameter | Validate input data |
| `TIMEOUT` | Request timed out | Increase timeout or retry |
| `RATE_LIMITED` | Too many requests | Implement backoff |
| `INSUFFICIENT_FUNDS` | Not enough credits | Top up identity |
| `SIGNATURE_VERIFICATION_FAILED` | Invalid signature | Check signer setup |

## Getting Help

If you're still experiencing issues:

1. Check the [API Documentation](./API_DOCUMENTATION.md)
2. Search [GitHub Issues](https://github.com/dashpay/platform/issues)
3. Ask on [Discord](https://discord.gg/dash)
4. Create a minimal reproduction example

### Creating a bug report

```javascript
// Minimal reproduction template
import init, { start, WasmSdk } from '@dashevo/wasm-sdk';

async function reproduce() {
  // Initialize
  await init();
  await start();
  
  // Setup
  const sdk = new WasmSdk('testnet');
  
  // Steps to reproduce
  try {
    // Your code here
  } catch (error) {
    console.error('Error:', error);
    console.log('SDK version:', SDK_VERSION);
    console.log('Browser:', navigator.userAgent);
  }
}

reproduce();
```