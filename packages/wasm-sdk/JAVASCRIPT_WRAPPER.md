# JavaScript Wrapper - Modern API Guide

The Dash Platform WASM SDK includes a modern JavaScript wrapper (`WasmSDK`) that provides a clean, Promise-based API over the raw WebAssembly bindings. This guide covers everything you need to know to use the wrapper effectively.

## Table of Contents

- [Quick Start](#quick-start)
- [Installation](#installation)
- [Configuration](#configuration)
- [API Reference](#api-reference)
- [Error Handling](#error-handling)
- [Resource Management](#resource-management)
- [Performance Guidelines](#performance-guidelines)
- [Troubleshooting](#troubleshooting)
- [Migration Guide](#migration-guide)

## Quick Start

Get started with the modern WasmSDK in under 2 minutes:

```javascript
import { WasmSDK } from '@dashevo/dash-wasm-sdk';

// 1. Create and configure SDK
const sdk = new WasmSDK({
    network: 'testnet',
    transport: {
        url: 'https://52.12.176.90:1443/',
        timeout: 30000
    },
    proofs: true
});

// 2. Initialize
await sdk.initialize();

// 3. Use the API
const identity = await sdk.getIdentity('4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF');
const documents = await sdk.getDocuments(contractId, 'note', {
    where: [['ownerId', '=', identityId]],
    limit: 10
});

// 4. Always cleanup
await sdk.destroy();
```

## Installation

### NPM/Yarn Installation

```bash
# NPM
npm install @dashevo/dash-wasm-sdk

# Yarn
yarn add @dashevo/dash-wasm-sdk
```

### Browser CDN (Alternative)

```html
<script type="module">
import { WasmSDK } from 'https://cdn.jsdelivr.net/npm/@dashevo/dash-wasm-sdk@latest/index.js';
</script>
```

### Environment Requirements

- **Node.js**: 16.x or higher
- **Browser**: Modern browsers with WebAssembly support
- **TypeScript**: Full type definitions included

## Configuration

The WasmSDK constructor accepts a comprehensive configuration object:

### Basic Configuration

```javascript
const sdk = new WasmSDK({
    network: 'testnet',           // 'testnet' or 'mainnet'
    transport: {
        url: 'https://52.12.176.90:1443/',
        timeout: 30000,           // Request timeout in ms
        retries: 3                // Number of retry attempts
    },
    proofs: true,                 // Enable cryptographic proof verification
    debug: false                  // Enable debug logging
});
```

### Advanced Configuration

```javascript
const sdk = new WasmSDK({
    network: 'mainnet',
    transport: {
        // Multiple endpoints for failover
        url: [
            'https://seed-1.testnet.networks.dash.org:1443/',
            'https://seed-2.testnet.networks.dash.org:1443/'
        ],
        timeout: 45000,
        retries: 5,
        // Connection pool settings
        maxConnections: 10,
        keepAlive: true
    },
    proofs: true,
    debug: process.env.NODE_ENV === 'development',
    // Resource management settings
    resources: {
        maxAge: 300000,           // Max resource age in ms
        cleanupInterval: 60000    // Cleanup interval in ms
    }
});
```

### Configuration Validation

The SDK validates all configuration options and throws descriptive errors:

```javascript
try {
    const sdk = new WasmSDK({
        network: 'invalid-network',  // This will throw
        transport: { url: 'not-a-url' }
    });
} catch (error) {
    console.error('Configuration error:', error.message);
    // "Invalid network: invalid-network. Must be one of: testnet, mainnet"
}
```

## API Reference

### Core Methods

#### `initialize()`
Initialize the WASM SDK and establish connection.

```javascript
await sdk.initialize();
// SDK is now ready for operations
```

**Returns**: `Promise<void>`  
**Throws**: `WasmInitializationError` if initialization fails

#### `isInitialized()`
Check if SDK is ready for operations.

```javascript
if (sdk.isInitialized()) {
    // Safe to perform operations
    const identity = await sdk.getIdentity(identityId);
}
```

**Returns**: `boolean`

#### `destroy()`
Clean up resources and destroy SDK instance.

```javascript
await sdk.destroy();
// SDK is now destroyed and cannot be used
```

**Returns**: `Promise<void>`

### Query Operations

#### `getIdentity(identityId)`
Retrieve an identity by ID.

```javascript
const identity = await sdk.getIdentity('4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF');
if (identity) {
    console.log('Identity found:', identity);
} else {
    console.log('Identity not found');
}
```

**Parameters**:
- `identityId` (string): Base58-encoded identity ID

**Returns**: `Promise<Object|null>` - Identity object or null if not found

#### `getIdentities(identityIds)`
Retrieve multiple identities by IDs.

```javascript
const identities = await sdk.getIdentities([
    '4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF',
    '6vbqTJxsBnwdEBZsV7HgSsWi7xBJL82MqgJ9QCUaGaZb'
]);
console.log(`Found ${identities.length} identities`);
```

**Parameters**:
- `identityIds` (string[]): Array of Base58-encoded identity IDs

**Returns**: `Promise<Object[]>` - Array of identity objects

#### `getDataContract(contractId)`
Retrieve a data contract by ID.

```javascript
const contract = await sdk.getDataContract('7V5e3dzBRDn7qhbEtaAmJNkqBkE1rCDjNB7YJBxtJzM8');
if (contract) {
    console.log('Contract version:', contract.version);
    console.log('Document types:', Object.keys(contract.documents));
}
```

**Parameters**:
- `contractId` (string): Base58-encoded data contract ID

**Returns**: `Promise<Object|null>` - Data contract object or null if not found

#### `getDocuments(contractId, documentType, options)`
Query documents from a data contract.

```javascript
// Basic query
const documents = await sdk.getDocuments(contractId, 'note', {
    limit: 10
});

// Advanced query with filters
const filteredDocs = await sdk.getDocuments(contractId, 'note', {
    where: [
        ['ownerId', '=', identityId],
        ['createdAt', '>', 1640995200000]
    ],
    orderBy: [['createdAt', 'desc']],
    limit: 50,
    offset: 0
});
```

**Parameters**:
- `contractId` (string): Data contract ID
- `documentType` (string): Type of document to query
- `options` (Object): Query options
  - `where` (Array): Where conditions `[field, operator, value]`
  - `orderBy` (Array): Order by conditions `[field, direction]`
  - `limit` (number): Maximum results (default: 100)
  - `offset` (number): Results to skip (default: 0)

**Returns**: `Promise<Object[]>` - Array of document objects

#### `getDocument(contractId, documentType, documentId)`
Retrieve a specific document by ID.

```javascript
const document = await sdk.getDocument(
    contractId, 
    'note', 
    'ByLJy37CpZAhEoWJEoHyKePkgRq8FRB7oZqBMxNiQN8C'
);
console.log('Document data:', document.data);
```

**Parameters**:
- `contractId` (string): Data contract ID
- `documentType` (string): Document type
- `documentId` (string): Base58-encoded document ID

**Returns**: `Promise<Object|null>` - Document object or null if not found

### State Transition Operations

#### `createIdentity(identityData, privateKey)`
Create a new identity on the platform.

```javascript
const identityData = {
    publicKeys: [{
        id: 0,
        type: 0,
        purpose: 0,
        securityLevel: 0,
        data: publicKeyBytes,
        readOnly: false
    }],
    balance: 1000000 // Credits in duffs
};

const result = await sdk.createIdentity(identityData, privateKeyHex);
console.log('New identity ID:', result.identityId);
```

**Parameters**:
- `identityData` (Object): Identity creation data
- `privateKey` (string): Private key for signing (hex format)

**Returns**: `Promise<Object>` - State transition result

#### `createDataContract(contractData, identityId, privateKey)`
Create a new data contract.

```javascript
const contractData = {
    documents: {
        note: {
            type: 'object',
            properties: {
                message: { type: 'string', maxLength: 256 }
            },
            additionalProperties: false
        }
    }
};

const result = await sdk.createDataContract(contractData, ownerId, privateKey);
console.log('Contract ID:', result.contractId);
```

**Parameters**:
- `contractData` (Object): Data contract definition
- `identityId` (string): Owner identity ID
- `privateKey` (string): Private key for signing

**Returns**: `Promise<Object>` - State transition result

#### `createDocument(documentData, contractId, documentType, identityId, privateKey)`
Create a new document.

```javascript
const documentData = {
    message: 'Hello, Dash Platform!'
};

const result = await sdk.createDocument(
    documentData,
    contractId,
    'note',
    ownerId,
    privateKey
);
console.log('Document ID:', result.documentId);
```

**Parameters**:
- `documentData` (Object): Document data
- `contractId` (string): Data contract ID
- `documentType` (string): Document type from contract
- `identityId` (string): Owner identity ID  
- `privateKey` (string): Private key for signing

**Returns**: `Promise<Object>` - State transition result

### Utility Operations

#### `getPlatformVersion()`
Get current platform version information.

```javascript
const version = await sdk.getPlatformVersion();
console.log('Platform version:', version.version);
console.log('Protocol version:', version.protocolVersion);
```

**Returns**: `Promise<Object>` - Version information

#### `getNetworkStatus()`
Get current network status.

```javascript
const status = await sdk.getNetworkStatus();
console.log('Block height:', status.coreBlockHeight);
console.log('Core version:', status.coreVersion);
```

**Returns**: `Promise<Object>` - Network status information

#### `validateDocument(document, dataContract)`
Validate a document against its data contract.

```javascript
const isValid = await sdk.validateDocument(document, dataContract);
if (!isValid) {
    console.error('Document validation failed');
}
```

**Parameters**:
- `document` (Object): Document to validate
- `dataContract` (Object): Data contract for validation

**Returns**: `Promise<boolean>` - True if valid

## Error Handling

The WasmSDK provides structured error handling with specific error types:

### Error Types

```javascript
import { 
    WasmSDKError,
    WasmInitializationError,
    WasmOperationError,
    WasmConfigurationError,
    WasmTransportError
} from '@dashevo/dash-wasm-sdk';
```

### Error Handling Patterns

#### Basic Error Handling

```javascript
try {
    const identity = await sdk.getIdentity(identityId);
    console.log('Identity retrieved successfully');
} catch (error) {
    if (error instanceof WasmOperationError) {
        console.error('Operation failed:', error.message);
        console.error('Operation name:', error.operation);
        console.error('Context:', error.context);
    } else {
        console.error('Unexpected error:', error.message);
    }
}
```

#### Comprehensive Error Handling

```javascript
async function handleSDKOperation() {
    try {
        await sdk.initialize();
        const result = await sdk.getIdentity(identityId);
        return result;
    } catch (error) {
        switch (error.constructor) {
            case WasmInitializationError:
                console.error('SDK initialization failed:', error.message);
                // Maybe retry initialization or use fallback
                break;
                
            case WasmTransportError:
                console.error('Network error:', error.message);
                // Maybe switch endpoints or retry
                break;
                
            case WasmConfigurationError:
                console.error('Configuration error:', error.message);
                // Fix configuration and retry
                break;
                
            case WasmOperationError:
                console.error('Operation error:', error.message);
                // Handle specific operation failure
                break;
                
            default:
                console.error('Unexpected error:', error.message);
                // Generic error handling
        }
        throw error; // Re-throw if needed
    }
}
```

#### Retry Logic Pattern

```javascript
async function withRetry(operation, maxRetries = 3) {
    let lastError;
    
    for (let i = 0; i < maxRetries; i++) {
        try {
            return await operation();
        } catch (error) {
            lastError = error;
            
            // Only retry on transport errors
            if (error instanceof WasmTransportError) {
                const delay = Math.pow(2, i) * 1000; // Exponential backoff
                console.warn(`Retry ${i + 1}/${maxRetries} after ${delay}ms`);
                await new Promise(resolve => setTimeout(resolve, delay));
                continue;
            }
            
            // Don't retry configuration or validation errors
            throw error;
        }
    }
    
    throw lastError;
}

// Usage
const identity = await withRetry(() => sdk.getIdentity(identityId));
```

## Resource Management

The WasmSDK includes automatic resource management to handle WebAssembly memory properly.

### Automatic Resource Management

```javascript
// Resources are automatically managed
const sdk = new WasmSDK(config);
await sdk.initialize();

// SDK automatically tracks and manages WASM resources
const identity = await sdk.getIdentity(identityId);
const documents = await sdk.getDocuments(contractId, 'note');

// Cleanup happens automatically, but you can force it
await sdk.destroy(); // Cleans up ALL resources
```

### Resource Statistics

```javascript
// Get current resource usage
const stats = sdk.getResourceStats();
console.log('Active resources:', stats.activeCount);
console.log('Total allocated:', stats.totalAllocated);
console.log('Memory usage:', stats.memoryUsage);
```

### Manual Resource Cleanup

```javascript
// Clean up stale resources (optional)
const cleaned = sdk.cleanupResources({
    maxAge: 300000  // Clean resources older than 5 minutes
});
console.log(`Cleaned up ${cleaned} stale resources`);
```

## Performance Guidelines

### Connection Management

```javascript
// Good: Reuse SDK instance
const sdk = new WasmSDK(config);
await sdk.initialize();

// Perform multiple operations with same instance
const identity1 = await sdk.getIdentity(id1);
const identity2 = await sdk.getIdentity(id2);
const documents = await sdk.getDocuments(contractId, 'note');

await sdk.destroy();
```

```javascript
// Avoid: Creating multiple instances
// This is inefficient and wastes resources
const sdk1 = new WasmSDK(config);
const sdk2 = new WasmSDK(config);
const sdk3 = new WasmSDK(config);
```

### Batch Operations

```javascript
// Good: Batch multiple identity requests
const identities = await sdk.getIdentities([id1, id2, id3]);

// Avoid: Multiple individual requests
const identity1 = await sdk.getIdentity(id1);
const identity2 = await sdk.getIdentity(id2);
const identity3 = await sdk.getIdentity(id3);
```

### Query Optimization

```javascript
// Good: Use specific queries with limits
const documents = await sdk.getDocuments(contractId, 'note', {
    where: [['ownerId', '=', identityId]],
    limit: 20,
    offset: 0
});

// Avoid: Fetching all documents without filters
const allDocs = await sdk.getDocuments(contractId, 'note'); // Could be huge
```

### Memory Management

```javascript
// For long-running applications, periodically cleanup
setInterval(() => {
    sdk.cleanupResources({ maxAge: 600000 }); // 10 minutes
}, 300000); // Check every 5 minutes
```

## Troubleshooting

### Common Issues

#### 1. SDK Not Initializing

**Error**: `WasmInitializationError: Failed to load WASM module`

**Solutions**:
```javascript
// Check if WASM is supported
if (!WebAssembly) {
    console.error('WebAssembly not supported in this environment');
}

// Ensure correct import path
import { WasmSDK } from '@dashevo/dash-wasm-sdk';

// Check network configuration
const sdk = new WasmSDK({
    network: 'testnet', // Make sure this is correct
    transport: {
        url: 'https://52.12.176.90:1443/' // Verify URL is accessible
    }
});
```

#### 2. Network Connection Issues

**Error**: `WasmTransportError: Connection timeout`

**Solutions**:
```javascript
// Increase timeout
const sdk = new WasmSDK({
    transport: {
        url: 'https://52.12.176.90:1443/',
        timeout: 60000,  // Increase to 60 seconds
        retries: 5       // More retries
    }
});

// Use multiple endpoints for failover
const sdk = new WasmSDK({
    transport: {
        url: [
            'https://seed-1.testnet.networks.dash.org:1443/',
            'https://seed-2.testnet.networks.dash.org:1443/'
        ]
    }
});
```

#### 3. Resource Leaks

**Error**: Application becomes slow over time

**Solutions**:
```javascript
// Always destroy SDK when done
await sdk.destroy();

// For long-running apps, periodic cleanup
setInterval(() => {
    sdk.cleanupResources();
}, 300000);

// Monitor resource usage
console.log('Resources:', sdk.getResourceStats());
```

#### 4. TypeScript Issues

**Error**: Type definitions not found

**Solutions**:
```typescript
// Make sure types are imported correctly
import { WasmSDK, WasmSDKConfig } from '@dashevo/dash-wasm-sdk';

// Explicit typing
const config: WasmSDKConfig = {
    network: 'testnet',
    transport: {
        url: 'https://52.12.176.90:1443/'
    }
};

const sdk: WasmSDK = new WasmSDK(config);
```

### Debug Mode

Enable debug mode for detailed logging:

```javascript
const sdk = new WasmSDK({
    debug: true,  // Enable debug logging
    // ... other config
});

// You'll see detailed logs like:
// "WasmSDK initialized successfully"
// "Operation get_identity completed in 245ms"
// "Resource cleanup: freed 3 objects"
```

### Performance Monitoring

```javascript
// Monitor operation performance
const startTime = Date.now();
const identity = await sdk.getIdentity(identityId);
const duration = Date.now() - startTime;
console.log(`getIdentity took ${duration}ms`);

// Monitor resource usage
const stats = sdk.getResourceStats();
if (stats.activeCount > 100) {
    console.warn('High resource usage detected');
    sdk.cleanupResources();
}
```

## Migration Guide

### From Raw WASM Bindings

If you're currently using the raw WASM bindings, here's how to migrate:

#### Before (Raw WASM)

```javascript
import init, { WasmSdkBuilder } from '@dashevo/dash-wasm-sdk';

// Complex initialization
await init();
const builder = WasmSdkBuilder.new_testnet();
const wasmSdk = builder.build();

// Manual resource management
try {
    const identity = wasmSdk.get_identity(identityId);
    // Use identity
} finally {
    // Manual cleanup
    if (wasmSdk && typeof wasmSdk.free === 'function') {
        wasmSdk.free();
    }
}
```

#### After (Modern Wrapper)

```javascript
import { WasmSDK } from '@dashevo/dash-wasm-sdk';

// Simple initialization
const sdk = new WasmSDK({ network: 'testnet' });
await sdk.initialize();

// Automatic resource management
const identity = await sdk.getIdentity(identityId);
// Resources automatically managed

// Simple cleanup
await sdk.destroy();
```

### From Other Dash SDKs

If you're migrating from other Dash SDK implementations:

#### Key Differences

1. **Initialization**: Modern config-driven approach
2. **Promises**: All operations return Promises
3. **Error Handling**: Structured error types
4. **Resource Management**: Automatic cleanup
5. **Type Safety**: Full TypeScript support

#### Migration Checklist

- [ ] Replace old SDK imports with `WasmSDK`
- [ ] Update initialization to use configuration object
- [ ] Convert callbacks to Promise-based code
- [ ] Update error handling to use structured errors
- [ ] Remove manual resource cleanup (now automatic)
- [ ] Add TypeScript types if using TypeScript

### Migration Example

```javascript
// Old SDK approach
import DashSDK from 'dash';

const client = new DashSDK.Client({
    network: 'testnet',
    apps: { dpns: { contractId: 'xxx' } }
});

client.getIdentity(identityId, (error, identity) => {
    if (error) {
        console.error('Error:', error);
        return;
    }
    console.log('Identity:', identity);
});
```

```javascript
// New WasmSDK approach
import { WasmSDK } from '@dashevo/dash-wasm-sdk';

const sdk = new WasmSDK({
    network: 'testnet',
    transport: { url: 'https://52.12.176.90:1443/' }
});

await sdk.initialize();

try {
    const identity = await sdk.getIdentity(identityId);
    console.log('Identity:', identity);
} catch (error) {
    console.error('Error:', error);
} finally {
    await sdk.destroy();
}
```

---

## Summary

The WasmSDK JavaScript wrapper provides:

- **Modern API**: Promise-based, configuration-driven
- **Automatic Resource Management**: No manual cleanup needed
- **Structured Error Handling**: Specific error types with context
- **TypeScript Support**: Full type definitions included
- **Performance Optimized**: Connection pooling, batching, caching
- **Developer Friendly**: Comprehensive documentation and examples

Ready to start building? Check out our [framework integration examples](./examples/) for React, Vue, Angular, and vanilla JavaScript implementations.