# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

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

## Common Issues

1. **"time not implemented on this platform"** - Fixed by using `js_sys::Date::now()` in WASM builds
2. **Import errors** - Token functions are methods on WasmSdk, not standalone functions
3. **Network timeouts** - Usually means invalid parameters or identities, NOT network issues