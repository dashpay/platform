# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 🚨 CRITICAL: File Structure Rules

**ONLY 3 FILES MATTER FOR PROJECT MANAGEMENT:**

1. **CLAUDE.md** (this file) - Development guidance for Claude Code
2. **PRD.md** - Pure requirements specification (what needs to be built)
3. **PLAN.md** - THE MASTER PLAN FILE at project root (what's outstanding)

## 🔴 MANDATORY RULE FOR CLAUDE CODE

**ALWAYS check and update PLAN.md (project root):**
- ✅ Check off completed items when work is done
- ➕ Add new items when gaps are discovered  
- 🔄 Update status and priorities as needed
- ❌ NEVER create new plan files
- ❌ NEVER create new status/tracking files
- ❌ NEVER create new progress reports

**The PLAN.md is the SINGLE SOURCE OF TRUTH for all outstanding work.**

## Documentation

**IMPORTANT**: For comprehensive API reference and usage examples, see:
- **[AI_REFERENCE.md](AI_REFERENCE.md)** - Complete API reference with all queries and state transitions
- **[docs.html](docs.html)** - User-friendly documentation
- **[index.html](index.html)** - Live interactive demo

When implementing WASM SDK functionality, always refer to AI_REFERENCE.md first for accurate method signatures and examples.

## Important Notes

### Network Connectivity
**THERE ARE NO CORS OR SSL ISSUES WITH THE DASH PLATFORM ENDPOINTS IN WASM-SDK**
- The Dash Platform HTTPS endpoints (e.g., https://52.12.176.90:1443) work perfectly fine from browsers
- These endpoints have proper CORS headers configured
- SSL certificates are valid and accepted by browsers
- If you see connection errors, check:
  - SDK initialization and configuration
  - Parameter validation (identity IDs, contract IDs, etc.)  
  - Whether the SDK is in the correct network mode (testnet vs mainnet)
  - The actual error message details (not just assuming it's CORS/SSL)

## Architecture

The WASM SDK is a WebAssembly build of the Dash SDK that runs in browsers. It provides:

1. **Queries** - Read operations that fetch data from Dash Platform
2. **State Transitions** - Write operations that modify state on Dash Platform

### Key Components

#### Rust Core (WASM Compilation)
- `src/sdk.rs` - Main SDK wrapper with WasmSdk and WasmSdkBuilder
- `src/queries/` - All query implementations (identity, documents, tokens, etc.)
- `src/state_transitions/` - State transition implementations
- `src/context_provider/` - Context providers for trusted/untrusted modes

#### JavaScript Wrapper (Service-Oriented Architecture)
- `src-js/index.js` - Main WasmSDK orchestrator class (delegates to services)
- `src-js/services/identity-service.js` - Identity operations and balance queries  
- `src-js/services/document-service.js` - Document CRUD with advanced querying
- `src-js/services/contract-service.js` - Data contract operations and validation
- `src-js/services/crypto-service.js` - Cryptographic operations (offline capable)
- `src-js/services/system-service.js` - Platform status and system information
- `src-js/services/dpns-service.js` - DPNS validation and homograph protection
- `src-js/config-manager.js` - Configuration validation and network management
- `src-js/resource-manager.js` - WASM memory lifecycle and cleanup automation
- `src-js/error-handler.js` - Structured error handling with security sanitization

#### Development and Documentation
- `index.html` - Interactive web interface for testing all SDK functionality

### Building

Run `./build.sh` to build the WASM module with enhanced JavaScript wrapper integration:

```bash
./build.sh
```

**Build System Features:**
- Unified WASM compilation using `packages/scripts/build-wasm.sh`  
- Automatic services directory integration (`src-js/services/` → `pkg/services/`)
- Package.json generation with all service files included
- JavaScript wrapper deployment with resource management
- Bundle size optimization (13.9MB output, 54% reduction from legacy)
- TypeScript definitions integration
- **Automatic disk cleanup** (4GB → ~1GB target directory, 75% space savings)
- Configurable cleanup options for development vs production builds

**Output Structure:**
```
pkg/
├── index.js                 # Main wrapper (entry point)  
├── services/                # 6 service classes (auto-copied)
├── config-manager.js        # Configuration management
├── resource-manager.js      # WASM resource lifecycle  
├── error-handler.js         # Structured error handling
├── types.d.ts              # TypeScript definitions
├── dash_wasm_sdk.js        # Current WASM bindings
└── dash_dash_wasm_sdk_bg.wasm   # WebAssembly binary (13.6MB)
```

### Modern JavaScript Usage (Recommended)

```javascript
import WasmSDK from 'dash-wasm-sdk';

// Initialize with configuration
const sdk = new WasmSDK({
    network: 'testnet',
    transport: { 
        url: 'https://52.12.176.90:1443/',
        timeout: 30000
    },
    proofs: true,
    debug: true
});

await sdk.initialize();

// Service-based operations (automatically delegates to appropriate service)
const identity = await sdk.getIdentity(identityId);          // IdentityService
const documents = await sdk.getDocuments(contractId, 'note'); // DocumentService  
const contract = await sdk.getDataContract(contractId);      // ContractService
const mnemonic = await sdk.generateMnemonic(12);            // CryptoService (offline)
const status = await sdk.getStatus();                       // SystemService
const isValid = await sdk.dpnsIsValidUsername('alice');     // DPNSService (offline)

// Always cleanup resources
await sdk.destroy();
```

### Web Interface Testing

1. Start web server: `python3 -m http.server 8888`
2. Open http://localhost:8888
3. Select network (testnet/mainnet)  
4. Choose operation type (queries/state transitions)
5. Fill in parameters and execute

### Package Installation Testing

```bash
# Install from NPM
npm install dash-wasm-sdk

# Test in Node.js
node -e "import('dash-wasm-sdk').then(m => console.log('✅ Package imported:', typeof m.default))"

# Build with automatic cleanup (saves 75% disk space)
./build.sh

# Manual cleanup if needed
cargo clean
```

## Project Development Status

### **Current Implementation Status**
- **Infrastructure**: All platform operation methods exist and are accessible
- **Authentication System**: Working (mnemonic→WIF derivation implemented)
- **Response Format**: PRD-compliant format implemented
- **Network Integration**: Testnet connectivity confirmed (3.4B+ credits accessible)
- **Performance**: Exceeds PRD requirements (benchmarked)

### **Outstanding Work for PRD Completion**
**Check PLAN.md for current status and outstanding items.**

**CRITICAL**: Always update PLAN.md when completing work or discovering new requirements.

### **Developer Guidance**
- **Requirements**: Follow `PRD.md` specifications exactly
- **Current Working**: Query operations, SDK initialization, method availability
- **Authentication**: WIF private key format working, mnemonic derivation needs debugging
- **Testing**: Use JavaScript wrapper approach (`WasmSDK` class, not direct WASM)
- **Network**: Use `proofs: false` for reliable testing

### **Key Architecture Insights**
- **State Transitions**: Implemented as WASM instance methods (e.g., `sdk.documentCreate()`)
- **JavaScript Wrapper**: Service-oriented architecture with proper error handling
- **Authentication**: WASM functions expect WIF private keys, not raw mnemonics
- **Network Configuration**: Trusted builder required, proof verification can have issues

## Documentation Maintenance

When adding new platform operations or queries:
1. Update the definitions in `index.html`
2. Run `python3 generate_docs.py` to regenerate documentation
3. Update `WASM_SDK_PRD.md` if requirements change
4. Update `WASM_SDK_IMPLEMENTATION_PLAN.md` if timeline changes
5. The CI will fail if documentation is out of sync

## Common Issues

1. **"time not implemented on this platform"** - Fixed by using `js_sys::Date::now()` in WASM builds
2. **Import errors** - Token functions are methods on WasmSdk, not standalone functions
3. **Network timeouts** - Usually means invalid parameters or identities, NOT network issues

## Query Support

The WASM SDK now fully supports where and orderBy clauses for document queries:

### Where Clauses
- Format: JSON array of clause arrays `[[field, operator, value], ...]`
- Supported operators:
  - `==` or `=` - Equal
  - `>` - Greater than
  - `>=` - Greater than or equals
  - `<` - Less than
  - `<=` - Less than or equals
  - `in` or `In` - In array
  - `startsWith` or `StartsWith` - String prefix match
  - `Between`, `BetweenExcludeBounds`, `BetweenExcludeLeft`, `BetweenExcludeRight` - Range operators

### Order By Clauses
- Format: JSON array of clause arrays `[[field, direction], ...]`
- Direction: `"asc"` or `"desc"`

### Example
```javascript
const whereClause = JSON.stringify([
    ["$ownerId", ">", "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk"],
    ["age", ">=", 18]
]);

const orderBy = JSON.stringify([
    ["$createdAt", "desc"],
    ["name", "asc"]
]);
```