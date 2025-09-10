# 🚀 WASM SDK Examples - Comprehensive Showcase

## 📚 Complete Example Script Library

This directory contains a comprehensive collection of example scripts demonstrating all aspects of the Dash Platform WASM SDK JavaScript wrapper. Each script is production-ready and showcases best practices.

---

## 🎯 Core Operation Examples

### 🔑 **Key Management** (`key-management.mjs`)
**Complete cryptographic operations demonstration**
- Mnemonic generation (12, 15, 18, 21, 24 words)
- Seed generation with/without passphrases  
- Key derivation with multiple path types (BIP44, DIP9, DIP13)
- Random key pair generation
- Address operations and validation
- Message signing workflows

```bash
node examples/key-management.mjs --network=testnet --debug
```

### 👤 **Identity Operations** (`identity-operations.mjs`)
**Comprehensive identity management and queries**
- Identity lookup and information retrieval
- Balance and revision queries
- Key management and nonce operations
- Multi-identity batch operations
- Token-related identity operations
- Public key hash operations

```bash
node examples/identity-operations.mjs [identity-id] --network=testnet --no-proofs
```

### 📄 **Contract & Document Lookup** (`contract-lookup.mjs`)
**Advanced contract and document exploration**
- Contract information retrieval
- Document type discovery
- Advanced document queries with where/orderBy clauses
- Pagination and bulk document retrieval
- Structured JSON response handling

```bash
node examples/contract-lookup.mjs [contract-id] [document-type] --no-proofs --debug
```

### 🌐 **DPNS Management** (`dpns-management.mjs`)
**Complete Dash Platform Name Service operations**
- Username validation and format checking
- Homograph safety conversion
- Contest detection and analysis
- Name resolution and availability checking
- Bulk validation workflows

```bash
node examples/dpns-management.mjs [username] --network=testnet --debug
```

### ⚙️ **System Monitoring** (`system-monitoring.mjs`)
**Platform status and monitoring operations**
- Real-time platform status
- Epoch and blockchain information
- Quorum and consensus monitoring
- Platform economics tracking
- Low-level state tree access

```bash
node examples/system-monitoring.mjs --network=testnet --no-proofs
```

### 🪙 **Token Operations** (`token-operations.mjs`)
**Complete token ecosystem exploration**
- Token status and metadata queries
- Direct purchase price information
- Token-contract relationship mapping
- Identity token balance operations
- Token distribution monitoring

```bash
node examples/token-operations.mjs [token-id] --network=testnet
```

---

## 🛠️ Use Case Examples

### 📱 **Social Media App** (`social-media-app.mjs`)
**Complete DashPay social application**
- User profile management
- Contact and friend systems
- Social discovery and networking
- Secure messaging workflows
- Social analytics and insights

```bash
node examples/social-media-app.mjs [identity-id] --network=testnet
```

### 🌐 **Domain Registry** (`domain-registry.mjs`)
**Production DPNS domain management system**
- Domain validation pipelines
- Registry exploration and analytics
- Subdomain hierarchy analysis
- Ownership pattern analysis
- Complete domain registry dashboard

```bash
node examples/domain-registry.mjs [domain-name] --network=testnet
```

### 💼 **Wallet Integration** (`wallet-integration.mjs`)
**Full-featured wallet application**
- Multi-address wallet creation
- Platform identity integration
- Security features and authentication
- Dashboard data collection
- Production-ready patterns

```bash
node examples/wallet-integration.mjs --network=testnet --debug
```

---

## 📖 Tutorial Examples

### 🚀 **Getting Started** (`getting-started.mjs`)
**Beginner-friendly comprehensive tutorial**
- Step-by-step SDK initialization
- Basic cryptographic operations
- Platform queries and exploration
- Error handling patterns
- Resource management

```bash
node examples/getting-started.mjs --network=testnet --debug
```

### 🎓 **Advanced Patterns** (`advanced-patterns.mjs`)
**Production-ready advanced techniques**
- Parallel operations and performance
- Batch processing strategies
- Robust error handling
- Pagination patterns
- Production deployment patterns

```bash
node examples/advanced-patterns.mjs --network=testnet --no-proofs
```

### 👤 **Identity Lookup** (`identity-lookup.mjs`)
**Focused identity exploration tool**
- Identity information retrieval
- Balance and key analysis
- DPNS username resolution
- Identity verification workflows

**Usage:**

```bash
# Use identity from .env file (proofs enabled by default)
node examples/identity-lookup.mjs

# Specify custom identity ID
node examples/identity-lookup.mjs <identity-id>

# Disable proof verification for faster lookups
node examples/identity-lookup.mjs <identity-id> --no-proofs

# Examples
node examples/identity-lookup.mjs DcoJJ3W9JauwLD51vzNuXJ9vnaZT7mprVm7wbgVYifNq
node examples/identity-lookup.mjs DcoJJ3W9JauwLD51vzNuXJ9vnaZT7mprVm7wbgVYifNq --no-proofs
```

**Proof Verification Control:**

The script supports both proof verification modes:
- **Default:** Proofs enabled (cryptographic verification)
- **Fast Mode:** Use `--no-proofs` flag for faster lookups without verification

**Environment Configuration:**

Uses modern JavaScript wrapper with `.env` configuration:
```bash
# .env file contents
NETWORK=testnet
IDENTITY_ID=DcoJJ3W9JauwLD51vzNuXJ9vnaZT7mprVm7wbgVYifNq
LOG_LEVEL=debug
```

**Sample Output:**
```
🔍 Dash Platform WASM SDK - Identity Lookup CLI
==================================================
🎯 Target Identity: DcoJJ3W9dauwLD51vzNuXJ9vna2T7mprVm7wbgVYifNq
🌐 Network: testnet

📦 Initializing WASM SDK for TESTNET...
✅ WASM module loaded
🔒 Prefetching trusted quorums...
✅ Trusted quorums prefetched
🛠️ Creating trusted SDK builder...
✅ SDK initialized for testnet with trusted mode

🔍 Looking up identity: DcoJJ3W9dauwLD51vzNuXJ9vna2T7mprVm7wbgVYifNq
✅ Identity found!

📋 Identity Information:
   ID: DcoJJ3W9dauwLD51vzNuXJ9vna2T7mprVm7wbgVYifNq
   Balance: 1000000 credits
   Revision: 1
   Public Keys: 2

🔑 Public Keys:
   Key 1: ECDSA_SECP256K1 (ID: 0)
   Key 2: BLS12_381 (ID: 1)

💰 Getting identity balance...
✅ Balance: {"balance": 1000000, "revision": 1}

🔑 Getting identity keys...
✅ Found 2 public keys

🎉 Identity lookup completed successfully!
```

## Requirements

### Node.js Version
- **Node.js 16.x or higher** (required for ES modules)
- **NPM or Yarn** for package management

### Dependencies
All dependencies are included in the WASM SDK package:
- `@dashevo/dash-wasm-sdk` (this package)
- Built-in Node.js modules: `fs`, `crypto`, `path`, `url`

## Installation

From the wasm-sdk directory:

```bash
# Make the script executable (optional)
chmod +x examples/identity-lookup-cli.mjs

# Run directly
node examples/identity-lookup-cli.mjs <identity-id>
```

## Error Handling

The script handles common errors gracefully:

### Network Issues
```
❌ Identity lookup failed: fetch failed
🌐 Error: Network connectivity issue
   Check internet connection and try again.
```

### Invalid Identity ID
```
❌ Identity not found
```

### Configuration Issues
```
❌ Identity lookup failed: Non-trusted mode is not supported in WASM
🔧 Error: Still using non-trusted mode!
   This indicates the trusted mode fix is not working correctly.
```

## Troubleshooting

### Common Issues

1. **Module Import Errors**
   ```bash
   Error: Cannot find module '../pkg/dash_wasm_sdk.js'
   ```
   - **Solution**: Run from the correct directory (wasm-sdk root)
   - Ensure the package is built: `./build.sh`

2. **WASM File Not Found**
   ```bash
   Error: ENOENT: no such file or directory, open '../pkg/dash_wasm_sdk_bg.wasm'
   ```
   - **Solution**: Build the package: `./build.sh`
   - Check that `pkg/` directory exists

3. **Network Timeout**
   ```bash
   Error: request timeout
   ```
   - **Solution**: Check internet connectivity
   - Try with a different identity ID
   - Verify testnet is operational

## Development

### Modifying the Script

To customize the script for your needs:

1. **Change Network**: Modify the `initializeSdk('testnet')` call
2. **Add More Queries**: Import additional functions from WASM SDK
3. **Custom Output**: Modify the display functions

### Adding New CLI Examples

Follow this pattern for new command line examples:
1. Set up Node.js crypto polyfill
2. Load WASM binary with `readFileSync`
3. Initialize with `await init(wasmBuffer)`
4. Use trusted builders with quorum prefetching
5. Handle errors gracefully
6. Clean up resources

## Integration

This script demonstrates modern JavaScript wrapper usage for Node.js applications:

```javascript
import { WasmSDK } from '@dashevo/dash-wasm-sdk';

// With proof verification (default)
const sdk = new WasmSDK({
    network: 'testnet',
    proofs: true  // Default: proof verification enabled
});

await sdk.initialize();
const identity = await sdk.getIdentity(identityId);

// Without proof verification (faster)
const fastSdk = new WasmSDK({
    network: 'testnet', 
    proofs: false  // Disable for faster lookups
});

await fastSdk.initialize();
const identityFast = await fastSdk.getIdentity(identityId);
```

## Security Notes

- ✅ **No private keys**: This script only performs read operations
- ✅ **No sensitive data**: Identity IDs are public information
- ✅ **Network security**: Uses HTTPS endpoints with certificate validation
- ✅ **Proof verification**: Validates cryptographic proofs for data integrity