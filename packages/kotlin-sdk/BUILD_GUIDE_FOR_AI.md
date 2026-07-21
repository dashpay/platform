# Build Guide for AI Assistants — Kotlin SDK / KotlinExampleApp

The Android analog of `packages/swift-sdk/BUILD_GUIDE_FOR_AI.md`. Read
`packages/kotlin-sdk/CLAUDE.md` first for the architectural doctrine;
this file is only about getting builds, tests, and the emulator running.

## Overview

Two Gradle modules under `packages/kotlin-sdk`:

- `:sdk` — Android library: JNI shims (`ffi/*Native.kt`), Kotlin
  wrappers, Room persistence, Keystore-backed `WalletStorage`.
- `:app` — KotlinExampleApp (`KotlinExampleApp/app`), the 1:1 port of
  SwiftExampleApp.

The native side is one cdylib, `libdash_sdk_jni.so`, built from
`packages/rs-unified-sdk-jni` and copied into
`sdk/src/main/jniLibs/<abi>/`.

## Prerequisites

- JDK 17, Android SDK platform 35, build-tools 35, NDK **r28+**
  (28.1.13356709 — 16 KB ELF page alignment is mandatory on API 35+).
- Rust stable with `aarch64-linux-android` / `x86_64-linux-android`
  targets, `cargo-ndk`, `protoc`.
- ABIs are 64-bit only (`arm64-v8a`, `x86_64`); halo2/Orchard does not
  build for 32-bit.

## The exFAT workaround (MANDATORY on this machine)

The repo lives on an exFAT volume (`/Volumes/Samsung_T5`). macOS
materializes xattrs there as `._*` AppleDouble files, which break:

- proto-glob build scripts (tenderdash-proto) during the cargo build,
- AAPT resource merging during the Gradle build,
- naive `find`/`glob` file enumeration in scripts (always exclude `._*`).

`build_android.sh` auto-creates an APFS sparse image
(`dash-build.sparseimage` mounted at `/Volumes/DashBuild`) and points
`CARGO_TARGET_DIR` there. Gradle needs the same treatment — **every**
Gradle invocation must relocate build dirs and the Android SDK:

```bash
cd packages/kotlin-sdk
export ANDROID_HOME=/Volumes/DashBuild/android-sdk
export DASH_GRADLE_BUILD_ROOT=/Volumes/DashBuild/gradle-build
./gradlew <tasks>
```

Forgetting `DASH_GRADLE_BUILD_ROOT` produces confusing AAPT
"duplicate resource" / merge errors from `._*` files, not a clear
message.

## Build Process

### 1. Build the native library

```bash
cd packages/kotlin-sdk
./build_android.sh --verify              # both ABIs, release, shielded ON
./build_android.sh --abi x86_64 --profile dev --verify   # fast emulator-only build
```

`--verify` checks the `.so` exports and 16 KB alignment. Output lands in
`sdk/src/main/jniLibs/<abi>/libdash_sdk_jni.so`.

### 2. Build SDK + app

```bash
export ANDROID_HOME=/Volumes/DashBuild/android-sdk
export DASH_GRADLE_BUILD_ROOT=/Volumes/DashBuild/gradle-build
./gradlew :sdk:assembleDebug :app:assembleDebug
```

## Testing

```bash
# JVM/Robolectric unit tests (no device needed):
./gradlew :sdk:testDebugUnitTest :app:testDebugUnitTest

# Compile the instrumented tests without running them:
./gradlew :sdk:compileDebugAndroidTestKotlin

# Instrumented tests (needs a running emulator/device):
./gradlew :sdk:connectedDebugAndroidTest

# Live-testnet integration tests are opt-in (skipped otherwise via the
# @TestnetTest guard in sdk/src/androidTest/.../testnet/):
./gradlew :sdk:connectedDebugAndroidTest -Ptestnet=true
```

## Emulator setup

The standing AVD is **dash-test** (API 35, x86_64):

```bash
export ANDROID_HOME=/Volumes/DashBuild/android-sdk
$ANDROID_HOME/emulator/emulator -avd dash-test -no-snapshot -no-boot-anim &
$ANDROID_HOME/platform-tools/adb wait-for-device

# Install + launch the example app
./gradlew :app:installDebug
adb shell am start -n org.dashfoundation.example/.MainActivity

# Watch native + SDK logs
adb logcat -s DashSDK dash_spv RustStdoutStderr
```

Emulator networking: the host machine is `10.0.2.2`, not `127.0.0.1`
(matters for local dashmate / custom SPV peers in Settings).

## Common Issues and Solutions

### Issue 1: Stale `.so` in the APK after a native rebuild

Gradle does not know the jniLibs changed (same path, new bytes) and may
package a cached merge. After every `build_android.sh` run:

```bash
./gradlew :sdk:assembleDebug :app:assembleDebug --rerun-tasks   # heavy hammer
# or the targeted version:
./gradlew :app:mergeDebugNativeLibs :app:assembleDebug
```

If the app still behaves stale on-device, uninstall first
(`adb uninstall org.dashfoundation.example`) — split APK caching can
keep the old library.

### Issue 2: AppleDouble (`._*`) noise

Symptoms: AAPT merge failures, cargo build-script glob panics, or tools
"finding" bogus files. Fixes: use the DashBuild relocations above; when
scripting, always filter (`grep -v '/\._'`, `find ... -name '._*'
-prune`). Never commit `._*` files (they show up in `git status` as
untracked noise — leave them).

### Issue 3: Kotlin nested comments

Kotlin block comments **nest** (unlike C/Java/Swift). Never write `/*`
inside a KDoc or block comment — e.g. a derivation path `m/9'/5'/...`
comment containing `/*` opens a nested comment and produces a baffling
"expecting */" error many lines later. Use `//` comments or reword.

### Issue 4: `UP-TO-DATE` after editing only jniLibs or BuildConfig inputs

`GIT_COMMIT` is stamped at configuration time in
`KotlinExampleApp/app/build.gradle.kts`; a new commit does not dirty the
task graph by itself. `--rerun-tasks` (or any real source change)
refreshes it.

### Issue 5: `connectedDebugAndroidTest` hangs with no device

The task waits forever if no emulator is booted. Check
`adb devices` first. In CI the emulator is provided by
`reactivecircus/android-emulator-runner` (see
`.github/workflows/kotlin-sdk-build.yml`).

### Issue 6: Room schema drift

Room schemas are exported to `sdk/schemas/` (also mounted as androidTest
assets). If you change an `@Entity`, bump the DB version or expect a
schema-hash crash at app start on devices with old data.

## Architecture Notes

- One JNI library: `rs-unified-sdk-jni` links `rs-sdk-ffi`,
  `platform-wallet-ffi`, and `key-wallet-ffi` as **rlibs** — FFI structs
  never cross JNI by value. Errors become
  `org.dashfoundation.dashsdk.ffi.DashSDKException(code, message)`;
  every export is panic-guarded.
- Handles cross as `jlong`; Kotlin owners are `AutoCloseable` with a
  `Cleaner` backstop (`Sdk`, `ManagedPlatformWallet`, `DataContractRef`).
- Rust→Kotlin callbacks (signer, mnemonic resolver, persistence, wallet
  events) can fire on Tokio worker threads.
- `PlatformWalletManager` is network-locked; network switch = destroy +
  recreate via `WalletManagerStore` (never reconfigure).
- The app (`KotlinExampleApp/app`) mirrors SwiftExampleApp: type-safe
  nav routes in `navigation/Routes.kt`, manual DI in `di/AppContainer.kt`,
  iOS accessibility identifiers reused as Compose `testTag`s.
- Parity status per Swift view: see `PARITY.md`.

## Verifying the Build

```bash
# 1. Native symbols present?
$ANDROID_HOME/ndk/28.1.13356709/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-nm \
  -D --defined-only sdk/src/main/jniLibs/arm64-v8a/libdash_sdk_jni.so | grep -c Java_
# 2. Full gate (what CI runs):
./gradlew :sdk:assembleDebug :sdk:testDebugUnitTest \
          :app:assembleDebug :app:testDebugUnitTest \
          :sdk:compileDebugAndroidTestKotlin
# 3. On-device smoke: FfiSmokeTest via connectedDebugAndroidTest.
```

## Important Files to Check

- `packages/kotlin-sdk/CLAUDE.md` — doctrine (what belongs in Rust).
- `packages/kotlin-sdk/build_android.sh` — native build + exFAT sparse image.
- `packages/kotlin-sdk/sdk/src/main/kotlin/org/dashfoundation/dashsdk/ffi/` — the JNI surface (what is actually bridged).
- `packages/kotlin-sdk/PARITY.md` — per-view parity vs SwiftExampleApp.
- `.github/workflows/kotlin-sdk-build.yml` — the CI pipeline.
