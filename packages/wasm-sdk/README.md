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
npm install dash-wasm-sdk
```

## Usage

### Modern API (Recommended)

```javascript
import WasmSDK from 'dash-wasm-sdk';

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
import WasmSDK, { WasmSDKConfig, Identity, Document } from 'dash-wasm-sdk';

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
import WasmSDK from 'dash-wasm-sdk';

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
import init, { WasmSdkBuilder } from 'dash-wasm-sdk';

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

The SDK uses an enhanced unified build system that creates a complete JavaScript library with service-oriented architecture:

```bash
# Install wasm-pack if not already installed
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Build the WASM SDK with JavaScript wrapper integration
./build.sh
```

### Build System Features

The enhanced build system includes:

- **Service Architecture Integration**: Automatically copies 6 service classes (Identity, Document, Contract, Crypto, System, DPNS)
- **Package Configuration**: Auto-generates package.json with all service files included
- **Bundle Optimization**: Optimized from 28MB to 13.9MB (54% reduction)  
- **JavaScript Wrapper**: Modern ES6 wrapper with TypeScript definitions
- **Resource Management**: Automatic WASM memory management and cleanup

### Build Output Structure

After building, the `pkg/` directory contains:

```
pkg/
├── index.js                 # Main JavaScript wrapper entry point
├── services/                # Service class implementations
│   ├── identity-service.js  # Identity operations
│   ├── document-service.js  # Document operations  
│   ├── contract-service.js  # Data contract operations
│   ├── crypto-service.js    # Cryptographic operations
│   ├── system-service.js    # System queries and status
│   └── dpns-service.js      # DPNS validation and resolution
├── config-manager.js        # Configuration management
├── resource-manager.js      # WASM resource lifecycle
├── error-handler.js         # Structured error handling
├── types.d.ts              # TypeScript definitions
├── dash_wasm_sdk.js        # WASM bindings
├── dash_wasm_sdk_bg.wasm   # WebAssembly binary
└── package.json            # NPM package configuration
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

### Service-Oriented Design

The WASM SDK uses a modern service-oriented architecture that separates concerns across focused service classes:

```javascript
import WasmSDK from 'dash-wasm-sdk';

const sdk = new WasmSDK({ network: 'testnet' });
await sdk.initialize();

// Each service handles specific operations:
// Identity Service - Identity management and queries
const identity = await sdk.getIdentity(identityId);
const balance = await sdk.getIdentityBalance(identityId);

// Document Service - Document operations with advanced querying  
const documents = await sdk.getDocuments(contractId, 'note', {
    where: [['ownerId', '=', identityId]],
    orderBy: [['createdAt', 'desc']],
    limit: 20
});

// Contract Service - Data contract operations
const contract = await sdk.getDataContract(contractId);

// Crypto Service - Cryptographic operations (offline)
const mnemonic = await sdk.generateMnemonic(12);
const isValid = await sdk.validateMnemonic(mnemonic);

// System Service - Platform status and information
const status = await sdk.getStatus();
const version = await sdk.getPlatformVersion();

// DPNS Service - Domain name validation and resolution (offline)
const validUsername = await sdk.dpnsIsValidUsername('alice');
const homographSafe = await sdk.dpnsConvertToHomographSafe('Alice');
```

### Service Classes

1. **IdentityService** (`services/identity-service.js`): Identity operations, balance queries, key management
2. **DocumentService** (`services/document-service.js`): Document CRUD with advanced querying capabilities
3. **ContractService** (`services/contract-service.js`): Data contract operations and validation
4. **CryptoService** (`services/crypto-service.js`): Cryptographic operations (mnemonic, keys, signing)
5. **SystemService** (`services/system-service.js`): Platform status, version, and system queries
6. **DPNSService** (`services/dpns-service.js`): Domain name validation and homograph protection

### File Structure

#### Rust Source Code
- `src/`: Core Rust implementation
  - `queries/`: Query implementations for all platform operations
  - `state_transitions/`: State transition implementations  
  - `sdk.rs`: Main WASM SDK interface
  - `wallet/`: Cryptographic operations and key management

#### JavaScript Wrapper (Generated)
- `src-js/`: Modern JavaScript wrapper source
  - `index.js`: Main WasmSDK orchestrator class
  - `services/`: Service class implementations (6 services)
  - `config-manager.js`: Configuration validation and management
  - `resource-manager.js`: WASM memory and resource lifecycle
  - `error-handler.js`: Structured error handling with security
  - `types.d.ts`: TypeScript definitions

#### Documentation and Tools
- `index.html`: Interactive demo and test interface
- `docs.html`: User-friendly documentation (auto-generated)
- `AI_REFERENCE.md`: Developer/AI reference documentation  
- `JAVASCRIPT_WRAPPER.md`: JavaScript wrapper comprehensive guide
- `generate_docs.py`: Documentation generator
- `check_documentation.py`: Documentation validation

#### Build System
- `build.sh`: Enhanced unified build script with service integration
- `build-optimized.sh`: Production build with size optimization
- `Cargo.toml`: Rust package configuration with WASM metadata

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