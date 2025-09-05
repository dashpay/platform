# Dash Platform WASM JS SDK

This package provides WebAssembly bindings for the Dash Platform SDK, allowing JavaScript and TypeScript applications in browsers to interact with Dash Platform.

## Overview

The WASM JS SDK provides:
- **Queries**: Read-only operations to fetch data from Dash Platform  
- **State Transitions**: Write operations to modify state on Dash Platform
- **Modern JavaScript API**: Clean wrapper with configuration-driven initialization
- **TypeScript Support**: Full type definitions with JSDoc documentation
- **Resource Management**: Automatic WASM memory management and cleanup

## Installation

```bash
npm install @dashevo/dash-wasm-sdk
```

## Usage

### Modern API (Recommended)

```javascript
import { WasmSDK } from '@dashevo/dash-wasm-sdk';

// Create and configure SDK
const sdk = new WasmSDK({
    network: 'testnet',
    transport: {
        url: 'https://52.12.176.90:1443/',
        timeout: 30000
    },
    proofs: true,
    debug: false
});

// Initialize and use
await sdk.initialize();

// Query operations
const identity = await sdk.getIdentity(identityId);
const documents = await sdk.getDocuments(contractId, 'note', {
    where: [['ownerId', '=', identityId]],
    limit: 10
});

// Always cleanup when done
await sdk.destroy();
```

### TypeScript Support

```typescript
import { WasmSDK, WasmSDKConfig, Identity, Document } from '@dashevo/dash-wasm-sdk';

const config: WasmSDKConfig = {
    network: 'testnet',
    transport: { timeout: 30000 },
    proofs: true
};

const sdk = new WasmSDK(config);
await sdk.initialize();

const identity: Identity | null = await sdk.getIdentity(identityId);
const documents: Document[] = await sdk.getDocuments(contractId, 'note');
```

### Configuration Options

The SDK supports flexible configuration:

```javascript
// Testnet with default settings
const sdk = new WasmSDK({ network: 'testnet' });

// Mainnet with custom endpoint
const sdk = new WasmSDK({
    network: 'mainnet',
    transport: {
        url: 'https://my-custom-node.example.com:1443/',
        timeout: 60000,
        retries: 5
    }
});

// Multiple endpoints with failover
const sdk = new WasmSDK({
    transport: {
        urls: [
            'https://primary.example.com:1443/',
            'https://fallback.example.com:1443/'
        ]
    }
});
```

### Command Line Usage

For Node.js applications and command line scripts:

```bash
# Identity lookup using .env configuration (proofs enabled by default)
node examples/identity-lookup.mjs

# Specify custom identity
node examples/identity-lookup.mjs <identity-id>

# Disable proof verification for faster lookups
node examples/identity-lookup.mjs <identity-id> --no-proofs
```

**Node.js Script Example (JavaScript Wrapper - Recommended):**
```javascript
import { WasmSDK } from '@dashevo/dash-wasm-sdk';

// Create and configure SDK (proofs enabled by default)
const sdk = new WasmSDK({
    network: 'testnet',
    proofs: true,  // Default: proof verification enabled
    transport: { timeout: 60000 }
});

// Initialize and use
await sdk.initialize();

// Lookup identity (wrapper handles proof verification internally)
const identity = await sdk.getIdentity(identityId);
console.log('Identity:', identity.toJSON());

// Example with proof verification disabled
const fastSdk = new WasmSDK({
    network: 'testnet',
    proofs: false  // Faster lookups without proof verification
});
await fastSdk.initialize();
const identity2 = await fastSdk.getIdentity(identityId);
```

### Legacy API (Raw WASM Bindings)

```javascript
import init, { WasmSdkBuilder } from '@dashevo/dash-wasm-sdk';

// Initialize WASM module
await init();

// Create SDK instance using builder pattern
const sdk = WasmSdkBuilder.new_testnet_trusted().build();

// Example query
const identity = await sdk.get_identity("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec");
```

## Documentation

- **[User Documentation](docs.html)**: Comprehensive guide for all queries and state transitions
- **[AI Reference](AI_REFERENCE.md)**: Quick reference optimized for AI assistants and developers
- **[Live Demo](index.html)**: Interactive interface to test all SDK functionality

## Development

### Building

The SDK requires Rust and wasm-pack:

```bash
# Install wasm-pack if not already installed
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Build the WASM module
./build.sh
```

### Documentation

**IMPORTANT**: Documentation must be kept in sync with `index.html`. When adding or modifying queries/state transitions in `index.html`, you MUST update the documentation:

```bash
# Regenerate documentation after changes to index.html
python3 generate_docs.py

# Check if documentation is up to date
python3 check_documentation.py
```

The CI will fail if documentation is out of sync with the code.

### Adding New Features

1. Add the query/transition definition to `index.html`
2. Implement the corresponding method in the Rust code
3. Regenerate documentation: `python3 generate_docs.py`
4. Test your changes using the web interface

### CI/CD

This package has automated checks for:
- Documentation completeness (all queries/transitions must be documented)
- Documentation freshness (docs must be regenerated when index.html changes)

The checks run on:
- Pull requests that modify relevant files
- Pushes to master and release branches

## Architecture

### File Structure

- `src/`: Rust source code
  - `queries/`: Query implementations
  - `state_transitions/`: State transition implementations
  - `sdk.rs`: Main SDK interface
- `index.html`: Interactive demo and test interface
- `docs.html`: User-friendly documentation
- `AI_REFERENCE.md`: Developer/AI reference documentation
- `generate_docs.py`: Documentation generator
- `check_documentation.py`: Documentation validation

### Key Concepts

1. **Queries**: Read operations that don't modify state
   - Identity queries (balance, keys, nonce)
   - Document queries (with where/orderBy support)
   - Data contract queries
   - Token queries
   - System queries

2. **State Transitions**: Operations that modify platform state
   - Identity operations (create, update, transfer)
   - Document operations (create, update, delete)
   - Token operations (mint, burn, transfer)
   - Voting operations

3. **Proofs**: Cryptographic proofs can be requested for most queries to verify data authenticity

## Testing

The web interface (`index.html`) provides comprehensive testing capabilities:
- Network selection (mainnet/testnet)
- Query execution with parameter validation
- State transition testing with authentication
- Proof verification toggle

## Contributing

1. Make your changes
2. Update documentation if needed: `python3 generate_docs.py`
3. Run tests
4. Submit a pull request

## License

See the main platform repository for license information.