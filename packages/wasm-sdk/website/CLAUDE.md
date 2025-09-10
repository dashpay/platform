# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

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

- `src/sdk.rs` - Main SDK wrapper with WasmSdk and WasmSdkBuilder
- `src/queries/` - All query implementations (identity, documents, tokens, etc.)
- `src/state_transitions/` - State transition implementations
- `src/context_provider/` - Context providers for trusted/untrusted modes
- `index.html` - Example web interface for testing SDK functionality

### Building

Run `./build.sh` to build the WASM module. Output goes to `pkg/` directory.

### Testing

1. Start web server: `python3 -m http.server 8888`
2. Open http://localhost:8888
3. Select network (testnet/mainnet)
4. Choose operation type (queries/state transitions)
5. Fill in parameters and execute

## Documentation Maintenance

When adding new queries or state transitions:
1. Update the definitions in `index.html`
2. Run `python3 generate_docs.py` to regenerate documentation
3. The CI will fail if documentation is out of sync

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