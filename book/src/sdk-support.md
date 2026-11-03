# SDK Support

Dash Platform provides SDKs for multiple languages and environments so
developers can build applications on whatever stack they prefer.

## Available SDKs

| SDK | Language | Status | Package | Use case |
|-----|----------|--------|---------|----------|
| **Rust SDK** | Rust | Available now | [`rs-sdk`](https://github.com/dashpay/platform/tree/master/packages/rs-sdk) | Server-side applications, full-node tooling, direct protocol access |
| **JavaScript SDK** | JavaScript / TypeScript | Available now | [`js-evo-sdk`](https://github.com/dashpay/platform/tree/master/packages/js-evo-sdk) | Node.js backends, scripts, CLI tools |
| **iOS SDK** | Swift | Available; iOS 18+ and macOS 15+ via Swift Package Manager. The `DashSDKFFI.xcframework` binary target is built locally by `build_ios.sh` | [`swift-sdk`](https://github.com/dashpay/platform/tree/master/packages/swift-sdk) | iOS and macOS applications |
| **Android SDK** | Kotlin | Available; an AAR is attached to every platform GitHub release. Maven coordinates `org.dashj:dash-sdk-android` are the publishing target described in the package's `PUBLISHING.md` | [`kotlin-sdk`](https://github.com/dashpay/platform/tree/master/packages/kotlin-sdk) | Android applications |

### Supporting packages

| Package | Purpose |
|---------|---------|
| [`rs-sdk-ffi`](https://github.com/dashpay/platform/tree/master/packages/rs-sdk-ffi) | C FFI layer over the Rust SDK; used by the Swift SDK, the Android SDK, and any language that can call C |
| [`rs-platform-wallet-ffi`](https://github.com/dashpay/platform/tree/master/packages/rs-platform-wallet-ffi) | C FFI layer over the platform wallet (persistence, key management, shielded pool) |
| [`rs-unified-sdk-ffi`](https://github.com/dashpay/platform/tree/master/packages/rs-unified-sdk-ffi) | Unified C ABI combining the SDK, wallet and core wallet FFI crates; packaged as `DashSDKFFI.xcframework` for the Swift SDK |
| [`rs-unified-sdk-jni`](https://github.com/dashpay/platform/tree/master/packages/rs-unified-sdk-jni) | JNI shim over the same FFI crates, loaded by the Kotlin SDK as `libdash_sdk_jni.so` |

## Choosing an SDK

**Building a server or CLI tool?** Use the **Rust SDK** for maximum
performance and direct access to all protocol features, or the **JavaScript
SDK** if your stack is Node.js.

**Building an iOS or macOS app?** Use the **Swift SDK**, which wraps the
Rust SDK through an FFI layer and provides native Swift types.

**Building an Android app?** Use the **Android SDK**, which wraps the same
FFI crates through a JNI shim with native Kotlin types.

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
