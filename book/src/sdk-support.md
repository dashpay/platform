# SDK Support

Dash Platform provides SDKs for multiple languages and environments so
developers can build applications on whatever stack they prefer.

## Available SDKs

| SDK | Language | Package | Use case |
|-----|----------|---------|----------|
| **Rust SDK** | Rust | [`rs-sdk`](https://github.com/dashpay/platform/tree/master/packages/rs-sdk) | Server-side applications, full-node tooling, direct protocol access |
| **JavaScript SDK** | JavaScript / TypeScript | [`js-dash-sdk`](https://github.com/dashpay/platform/tree/master/packages/js-dash-sdk) | Node.js backends, scripts, CLI tools |
| **WASM SDK** | JavaScript / TypeScript (browser) | [`wasm-sdk`](https://github.com/dashpay/platform/tree/master/packages/wasm-sdk) | Browser-based dApps, client-side applications |
| **iOS SDK** | Swift | [`swift-sdk`](https://github.com/dashpay/platform/tree/master/packages/swift-sdk) | iOS and macOS applications |

### Supporting packages

| Package | Purpose |
|---------|---------|
| [`rs-sdk-ffi`](https://github.com/dashpay/platform/tree/master/packages/rs-sdk-ffi) | C FFI layer over the Rust SDK; used by the Swift SDK and any language that can call C |
| [`wasm-dpp2`](https://github.com/dashpay/platform/tree/master/packages/wasm-dpp2) | WASM bindings for Dash Platform Protocol types; used by the WASM SDK |
| [`js-evo-sdk`](https://github.com/dashpay/platform/tree/master/packages/js-evo-sdk) | Legacy JavaScript SDK (being replaced by `js-dash-sdk`) |

## Choosing an SDK

**Building a server or CLI tool?** Use the **Rust SDK** for maximum
performance and direct access to all protocol features, or the **JavaScript
SDK** if your stack is Node.js.

**Building a web app?** Use the **WASM SDK**. It compiles the Rust core to
WebAssembly so you get the same protocol logic in the browser with TypeScript
type definitions.

**Building an iOS or macOS app?** Use the **Swift SDK**, which wraps the Rust
SDK through an FFI layer and provides native Swift types.

**Building for another language?** The **FFI layer** (`rs-sdk-ffi`) exposes a
C-compatible interface that can be called from Python, Kotlin, C#, or any
language with C interop support.

## What every SDK provides

All SDKs share the same underlying Rust implementation, so behavior is
consistent across platforms:

- **Identity management** -- create, top up, and manage identities with
  hierarchical key support
- **Data contract deployment** -- define and publish JSON Schema-based data
  contracts
- **Document operations** -- create, update, delete, and query documents with
  proof verification
- **Token operations** -- query balances, supply, statuses, and
  pre-programmed distributions
- **Name registration** -- register and resolve DPNS usernames
- **Proof verification** -- every query response can be cryptographically
  verified against the platform state root
