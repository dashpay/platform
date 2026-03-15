# SDK Support

Dash Platform provides SDKs for multiple languages and environments so
developers can build applications on whatever stack they prefer.

## Available SDKs

| SDK | Language | Status | Package | Use case |
|-----|----------|--------|---------|----------|
| **Rust SDK** | Rust | Available now | [`rs-sdk`](https://github.com/dashpay/platform/tree/master/packages/rs-sdk) | Server-side applications, full-node tooling, direct protocol access |
| **JavaScript SDK** | JavaScript / TypeScript | Available now | [`js-dash-sdk`](https://github.com/dashpay/platform/tree/master/packages/js-dash-sdk) | Node.js backends, scripts, CLI tools |
| **iOS SDK** | Swift | Coming in v3.1 | [`swift-sdk`](https://github.com/dashpay/platform/tree/master/packages/swift-sdk) | iOS and macOS applications |
| **Android SDK** | Kotlin | Coming in v3.2 | -- | Android applications |

### Supporting packages

| Package | Purpose |
|---------|---------|
| [`rs-sdk-ffi`](https://github.com/dashpay/platform/tree/master/packages/rs-sdk-ffi) | C FFI layer over the Rust SDK; used by the Swift SDK, the Android SDK, and any language that can call C |

## Choosing an SDK

**Building a server or CLI tool?** Use the **Rust SDK** for maximum
performance and direct access to all protocol features, or the **JavaScript
SDK** if your stack is Node.js.

**Building an iOS or macOS app?** Use the **Swift SDK** (v3.1+), which wraps
the Rust SDK through an FFI layer and provides native Swift types.

**Building an Android app?** The **Android SDK** (v3.2+) will wrap the same
FFI layer with native Kotlin types.

**Building for another language?** The **FFI layer** (`rs-sdk-ffi`) exposes a
C-compatible interface that can be called from Python, C#, or any language
with C interop support.

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
