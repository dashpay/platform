#!/usr/bin/env bash
# Build libdash_sdk_jni.so for Android — the analog of ../swift-sdk/build_ios.sh.
#
# Produces one cdylib per ABI containing the JNI exports of rs-unified-sdk-jni plus
# the C ABI of rs-sdk-ffi / platform-wallet-ffi / key-wallet-ffi (linked in
# as rlibs), and drops them into sdk/src/main/jniLibs/<abi>/ where AGP
# packages them.
#
# ABI policy: arm64-v8a (devices) + x86_64 (emulator) only. armeabi-v7a is
# intentionally unsupported: minSdk 29 devices are effectively all 64-bit
# and the shielded (halo2/Orchard) crypto is not viable on 32-bit.
#
# Requirements:
#   - Android NDK r28+ (16 KB page alignment by default). Set ANDROID_NDK_HOME
#     or install via sdkmanager "ndk;28.1.13356709".
#   - cargo-ndk (cargo install cargo-ndk)
#   - rustup targets: aarch64-linux-android, x86_64-linux-android
#
# Usage:
#   ./build_android.sh [--abi arm64|x86_64|all] [--profile dev|release]
#                      [--no-shielded] [--clean] [--verify]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
JNILIBS_DIR="$SCRIPT_DIR/sdk/src/main/jniLibs"
PACKAGE="rs-unified-sdk-jni"
LIB_NAME="libdash_sdk_jni.so"
MIN_SDK=29

ABI="all"
PROFILE="release"
SHIELDED=1
CLEAN=0
VERIFY=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --abi)         ABI="$2"; shift 2 ;;
        --profile)     PROFILE="$2"; shift 2 ;;
        --no-shielded) SHIELDED=0; shift ;;
        --clean)       CLEAN=1; shift ;;
        --verify)      VERIFY=1; shift ;;
        -h|--help)     grep '^#' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "Unknown option: $1" >&2; exit 1 ;;
    esac
done

case "$PROFILE" in
    dev)     CARGO_PROFILE="dev-android" ;;
    release) CARGO_PROFILE="release-android" ;;
    *) echo "Invalid --profile '$PROFILE' (dev|release)" >&2; exit 1 ;;
esac

TARGETS=()
case "$ABI" in
    arm64)  TARGETS=(arm64-v8a) ;;
    x86_64) TARGETS=(x86_64) ;;
    all)    TARGETS=(arm64-v8a x86_64) ;;
    *) echo "Invalid --abi '$ABI' (arm64|x86_64|all)" >&2; exit 1 ;;
esac

FEATURES=""
if [[ $SHIELDED -eq 1 ]]; then
    FEATURES="shielded"
fi
FEATURE_ARGS=()
if [[ -n "$FEATURES" ]]; then
    FEATURE_ARGS+=(--features "$FEATURES")
fi

# --- Toolchain checks -------------------------------------------------------

if ! command -v cargo-ndk >/dev/null 2>&1; then
    echo "cargo-ndk not found — install with: cargo install cargo-ndk" >&2
    exit 1
fi

if [[ -z "${ANDROID_NDK_HOME:-}" ]]; then
    # Try the default sdkmanager install location, newest first.
    NDK_ROOT="${ANDROID_HOME:-$HOME/Library/Android/sdk}/ndk"
    if [[ -d "$NDK_ROOT" ]]; then
        # BSD sort has no -V; numeric-sort the dotted version components.
        ANDROID_NDK_HOME="$NDK_ROOT/$(find "$NDK_ROOT" -mindepth 1 -maxdepth 1 -type d -exec basename {} \; | sort -t. -k1,1n -k2,2n -k3,3n | tail -1)"
        export ANDROID_NDK_HOME
    fi
fi
if [[ -z "${ANDROID_NDK_HOME:-}" || ! -d "$ANDROID_NDK_HOME" ]]; then
    echo "ANDROID_NDK_HOME is not set and no NDK found — install NDK r28+" >&2
    exit 1
fi
NDK_MAJOR="$(sed -n 's/^Pkg.Revision = \([0-9]*\).*/\1/p' "$ANDROID_NDK_HOME/source.properties" 2>/dev/null || echo 0)"
if [[ "$NDK_MAJOR" -lt 28 ]]; then
    echo "WARNING: NDK r$NDK_MAJOR < r28 — 16 KB page alignment is not the default." >&2
    echo "         RUSTFLAGS below force it, but prefer NDK r28+." >&2
fi

for t in aarch64-linux-android x86_64-linux-android; do
    if ! rustup target list --installed | grep -q "^$t$"; then
        echo "Adding Rust target $t"
        rustup target add "$t"
    fi
done

# macOS materializes xattrs (com.apple.provenance) as AppleDouble ._* files
# on non-APFS volumes, which breaks proto-glob build scripts (tenderdash).
# When the repo lives on such a volume, host the cargo target dir on an APFS
# sparse image instead (created on first use, mounted on demand).
# macOS-only: diskutil/hdiutil don't exist on Linux CI runners.
if [[ "$(uname)" == "Darwin" && -z "${CARGO_TARGET_DIR:-}" ]]; then
    REPO_FS="$(diskutil info "$(df "$REPO_ROOT" | tail -1 | awk '{print $NF}')" 2>/dev/null \
        | sed -n 's/.*File System Personality: *//p')"
    if [[ -n "$REPO_FS" && "$REPO_FS" != *APFS* ]]; then
        IMAGE="$(dirname "$REPO_ROOT")/../dash-build.sparseimage"
        MOUNTPOINT="/Volumes/DashBuild"
        if [[ ! -d "$MOUNTPOINT" ]]; then
            [[ -f "$IMAGE" ]] || hdiutil create -size 200g -type SPARSE -fs APFS \
                -volname DashBuild "$IMAGE" >/dev/null
            hdiutil attach "$IMAGE" -mountpoint "$MOUNTPOINT" >/dev/null
        fi
        export CARGO_TARGET_DIR="$MOUNTPOINT/cargo-target"
        echo "Repo volume is $REPO_FS — using CARGO_TARGET_DIR=$CARGO_TARGET_DIR"
    fi
fi
export COPYFILE_DISABLE=1

if [[ $CLEAN -eq 1 ]]; then
    echo "Cleaning $PACKAGE"
    cargo clean -p "$PACKAGE" --profile "$CARGO_PROFILE" 2>/dev/null || cargo clean -p "$PACKAGE"
fi

# --- Build ------------------------------------------------------------------

# Safety net: force 16 KB ELF page alignment even on NDK < r28.
export RUSTFLAGS="${RUSTFLAGS:-} -C link-arg=-Wl,-z,max-page-size=16384"

# Some C-vendoring crates (rs-x11-hash) hardcode `cc.compiler("clang")` in
# their build.rs, bypassing the CC_<target> env cargo-ndk sets. Prepend the
# NDK LLVM bin dir so a bare `clang` resolves to the NDK driver, which
# locates the Android sysroot relative to its own path.
NDK_LLVM_BIN="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt"
NDK_LLVM_BIN="$NDK_LLVM_BIN/$(ls "$NDK_LLVM_BIN" | head -1)/bin"
export PATH="$NDK_LLVM_BIN:$PATH"

echo "Building $PACKAGE (${TARGETS[*]}) profile=$CARGO_PROFILE features='${FEATURES}'"
cd "$REPO_ROOT"

CARGO_NDK_ARGS=()
for t in "${TARGETS[@]}"; do
    CARGO_NDK_ARGS+=(-t "$t")
done

cargo ndk "${CARGO_NDK_ARGS[@]}" -o "$JNILIBS_DIR" -P "$MIN_SDK" \
    build -p "$PACKAGE" --profile "$CARGO_PROFILE" \
    "${FEATURE_ARGS[@]}" --no-default-features

# cargo-ndk copies every built .so, including the standalone cdylibs the FFI
# dependency crates also produce (librs_sdk_ffi.so, libplatform_wallet_ffi.so,
# …). Those are already linked INTO $LIB_NAME via their rlibs — packaging
# them would triple the APK for nothing. Keep only the shim library.
for t in "${TARGETS[@]}"; do
    find "$JNILIBS_DIR/$t" -name "*.so" ! -name "$LIB_NAME" -delete 2>/dev/null || true
done

# --- Verify -----------------------------------------------------------------

LLVM_BIN="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt"
LLVM_BIN="$LLVM_BIN/$(ls "$LLVM_BIN" | head -1)/bin"

for t in "${TARGETS[@]}"; do
    SO="$JNILIBS_DIR/$t/$LIB_NAME"
    if [[ ! -f "$SO" ]]; then
        echo "ERROR: expected output missing: $SO" >&2
        exit 1
    fi
    SIZE=$(du -h "$SO" | cut -f1)
    echo "Built $SO ($SIZE)"

    if [[ $VERIFY -eq 1 ]]; then
        JNI_COUNT=$("$LLVM_BIN/llvm-nm" -D "$SO" | grep -c " T Java_" || true)
        C_ABI_OK=$("$LLVM_BIN/llvm-nm" -D "$SO" | grep -c " T dash_sdk_init" || true)
        # BSD awk (macOS) has no strtonum; hex-compare in shell instead.
        ALIGN_BAD=0
        while read -r align; do
            [[ $((align)) -lt 16384 ]] && ALIGN_BAD=$((ALIGN_BAD + 1))
        done < <("$LLVM_BIN/llvm-readelf" -l "$SO" | awk '/LOAD/ { print $NF }')
        echo "  JNI exports: $JNI_COUNT, C ABI anchored: $C_ABI_OK, misaligned LOAD segments: $ALIGN_BAD"
        if [[ "$JNI_COUNT" -eq 0 ]]; then
            echo "ERROR: no Java_* symbols exported" >&2; exit 1
        fi
        if [[ "$ALIGN_BAD" -ne 0 ]]; then
            echo "ERROR: LOAD segments not 16 KB aligned (Android 15+ requirement)" >&2; exit 1
        fi
    fi
done

echo "Done. Libraries are in $JNILIBS_DIR/<abi>/$LIB_NAME"
