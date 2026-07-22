# CLAUDE.md — Kotlin SDK

This file guides Claude Code when working in `packages/kotlin-sdk`. It is the
Android port of the doctrine in `packages/swift-sdk/CLAUDE.md` — read that
file too; the architectural rules are identical.

## The Kotlin SDK does exactly three things

1. **Persist data** — write Room rows + Keystore-encrypted secrets in
   response to Rust state (via the persistence callback bridge).
2. **Load data** — expose Room `Flow` queries for reactive UI updates.
3. **Bridge** — thin JNI wrappers around functions defined in the Rust
   libraries (`rs-sdk-ffi`, `platform-wallet-ffi`, `key-wallet-ffi`).

## Forbidden in Kotlin (belongs in Rust `platform-wallet`)

- No derivation-path building.
- No policy-loop orchestration (gap limits, key indices, discovery walks).
- No pulling mnemonics/seeds across the JNI boundary for "finishing" in
  Kotlin.
- No re-implementing protocol constants.
- No JNI functions that stitch together existing Rust calls — add the
  composite to Rust instead.

**Exception:** Android Keystore/DataStore writes — Rust derives key material,
Kotlin encrypts and stores it (Keystore keys are non-exportable, so secrets
are data encrypted under Keystore-wrapped AES keys).

## Architecture

- `packages/rs-unified-sdk-jni` — Rust cdylib exposing JNI symbols. It calls the
  `extern "C"` entry points of the FFI crates **as rlib dependencies**, so
  `DashSDKResult` never crosses JNI by value. Errors throw
  `org.dashfoundation.dashsdk.ffi.DashSDKException(code, message)`; panics
  are caught at every export (`support::guard`) — the JNI library must never
  abort the app process (workspace profiles `*-android` keep
  `panic = "unwind"`).
- Handles cross JNI as `jlong`; Kotlin owners are `AutoCloseable` with a
  `Cleaner` backstop.
- Rust→Kotlin callbacks (signer, mnemonic resolver, persistence, events) may
  fire on Tokio worker threads; trampolines attach with
  `attach_current_thread_as_daemon()` and hold Kotlin bridge objects as
  `GlobalRef`s.
- `PlatformWalletManager` is network-locked at construction. Network switch =
  destroy + new instance (`WalletManagerStore`), never reconfiguration.

## Building

```bash
# Native library (both ABIs, release, shielded on):
./build_android.sh --verify

# Gradle:
./gradlew :sdk:assembleDebug :sdk:testDebugUnitTest
./gradlew :app:assembleDebug            # KotlinExampleApp
./gradlew :sdk:connectedDebugAndroidTest  # needs emulator/device
```

- ABIs: `arm64-v8a` + `x86_64` only (no 32-bit; halo2/Orchard is 64-bit).
- NDK r28+ required (16 KB page alignment). minSdk 29, target/compile 35.
- The `shielded` cargo feature is ON by default for iOS parity.

### Repo-on-exFAT gotcha

This repo often lives on an exFAT volume where macOS materializes xattrs as
`._*` AppleDouble files, which breaks proto-glob build scripts
(tenderdash-proto). `build_android.sh` auto-creates and mounts an APFS sparse
image (`dash-build.sparseimage` → `/Volumes/DashBuild`) and points
`CARGO_TARGET_DIR` at it. Do the same for manual `cargo` commands, and
relocate Gradle build dirs too (AAPT resource merging breaks on `._*` files
the same way):

```bash
export CARGO_TARGET_DIR=/Volumes/DashBuild/cargo-target
export DASH_GRADLE_BUILD_ROOT=/Volumes/DashBuild/gradle-build
```

## Testing

- JVM/Robolectric unit tests: Room DAOs, converters, error mapping.
- Instrumented tests (`sdk/src/androidTest`): FFI smoke test
  (`FfiSmokeTest`) is the A-M1 gate — library loads, version resolves, SDK
  handle round-trips.
- Testnet integration tests are tagged and opt-in (`-Ptestnet=true`).

## Keeping parity with iOS

The reference implementation is `packages/swift-sdk` + its SwiftExampleApp.
When porting behavior, cite the Swift source file in the KDoc. Reuse iOS
accessibility identifier strings verbatim as Compose `testTag`s for
cross-platform UAT parity.
