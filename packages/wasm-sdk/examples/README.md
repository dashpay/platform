# Command Line Examples

This directory contains command line scripts demonstrating how to use the Dash Platform WASM SDK from Node.js.

## Available Scripts

### Identity Lookup CLI

**File:** `identity-lookup.mjs`

A complete command line tool for looking up identity information from Dash Platform.

**Features:**
- ✅ Uses modern JavaScript wrapper (`WasmSDK`)
- ✅ Configurable proof verification (enabled by default)
- ✅ Complete identity data with proper key mapping
- ✅ `.env` file configuration support
- ✅ Rich JSON output with formatted display
- ✅ Command line proof control (`--no-proofs` flag)

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
   Error: Cannot find module '../pkg/wasm_sdk.js'
   ```
   - **Solution**: Run from the correct directory (wasm-sdk root)
   - Ensure the package is built: `./build.sh`

2. **WASM File Not Found**
   ```bash
   Error: ENOENT: no such file or directory, open '../pkg/wasm_sdk_bg.wasm'
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