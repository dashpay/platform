# Dash Kotlin SDK (Android)

Android SDK and example app for Dash Platform — the Kotlin counterpart of
[`packages/swift-sdk`](../swift-sdk). It combines:

- **Core SDK** (SPV wallet): key derivation, accounts, UTXOs, SPV sync
- **Platform SDK**: identities, DPNS names, data contracts, documents,
  tokens, DashPay
- **KotlinExampleApp**: a Jetpack Compose app demonstrating the full surface,
  a one-for-one port of SwiftExampleApp

## Layout

```
kotlin-sdk/
├── sdk/                 # Android library (org.dashfoundation.dashsdk)
├── KotlinExampleApp/    # Compose example app (org.dashfoundation.example)
├── build_android.sh     # builds libdash_sdk_jni.so via cargo-ndk
└── gradle…              # Gradle 8.14 / AGP 8.9 / Kotlin 2.1
```

The native layer is [`packages/rs-unified-sdk-jni`](../rs-unified-sdk-jni), a Rust JNI shim
over `rs-sdk-ffi`, `platform-wallet-ffi` and `key-wallet-ffi`. There is no C
glue and no generated headers on Android — the shim calls the FFI crates as
ordinary Rust dependencies and exposes JNI symbols directly.

## Prerequisites

- JDK 17+
- Android SDK with platform 35 + NDK r28+ (`sdkmanager "ndk;28.1.13356709"`)
- Rust (workspace toolchain) + `cargo install cargo-ndk`
- Rust targets: `rustup target add aarch64-linux-android x86_64-linux-android`

## Build

```bash
# 1. Native library → sdk/src/main/jniLibs/{arm64-v8a,x86_64}/
./build_android.sh --verify

# 2. SDK + example app
./gradlew :sdk:assembleDebug :app:assembleDebug

# 3. Tests
./gradlew :sdk:testDebugUnitTest                 # JVM/Robolectric
./gradlew :sdk:connectedDebugAndroidTest         # instrumented (emulator)
```

Supported ABIs: `arm64-v8a` (devices) and `x86_64` (emulator). minSdk 29,
targetSdk 35. The `shielded` (Orchard) feature is enabled by default; build
with `./build_android.sh --no-shielded` for a slim transparent-only library.

## Architecture

See [CLAUDE.md](CLAUDE.md) for the layering rules (persist / load / bridge —
all orchestration lives in the Rust `platform-wallet` crate) and
[../swift-sdk/CLAUDE.md](../swift-sdk/CLAUDE.md) for the reference doctrine.
