#!/usr/bin/env bash
# Unified WASM build script for Dash Platform WASM packages
set -euo pipefail

# Function to display usage
usage() {
    echo "Usage: $0 [OPTIONS]"
    echo "Options:"
    echo "  -p, --package NAME     Package name (required)"
    echo "  -t, --target TYPE      wasm-pack target type (default: web)"
    echo "  -o, --opt-level LEVEL  Optimization level: full, minimal, none (default: full)"
    echo "  -h, --help             Display this help message"
    echo ""
    echo "Example:"
    echo "  $0 --package wasm-sdk"
    echo "  $0 --package wasm-drive-verify --opt-level minimal"
}

# Default values
PACKAGE_NAME=""
TARGET_TYPE="web"
OPT_LEVEL="full"
USE_WASM_PACK=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -p|--package)
            if [[ $# -lt 2 ]]; then
                echo "Error: Missing argument for $1"
                usage
                exit 1
            fi
            PACKAGE_NAME="$2"
            shift 2
            ;;
        -t|--target)
            if [[ $# -lt 2 ]]; then
                echo "Error: Missing argument for $1"
                usage
                exit 1
            fi
            TARGET_TYPE="$2"
            shift 2
            ;;
        -o|--opt-level)
            if [[ $# -lt 2 ]]; then
                echo "Error: Missing argument for $1"
                usage
                exit 1
            fi
            OPT_LEVEL="$2"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Error: Unknown option $1"
            usage
            exit 1
            ;;
    esac
done

# Validate required arguments
if [ -z "$PACKAGE_NAME" ]; then
    echo "Error: Package name is required"
    usage
    exit 1
fi

# Determine build method based on package
case "$PACKAGE_NAME" in
    "wasm-sdk")
        USE_WASM_PACK=true
        WASM_FILE="wasm_sdk_bg.wasm"
        ;;
    "wasm-drive-verify")
        USE_WASM_PACK=false
        WASM_FILE="wasm_drive_verify_bg.wasm"
        ;;
    *)
        echo "Error: Unknown package '$PACKAGE_NAME'"
        exit 1
        ;;
esac

# Get script directory and package directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PACKAGE_DIR="$(dirname "$SCRIPT_DIR")/$PACKAGE_NAME"

# Change to package directory
cd "$PACKAGE_DIR"

echo "Building $PACKAGE_NAME..."

# Create pkg directory if it doesn't exist
mkdir -p pkg

if [ "$USE_WASM_PACK" = true ]; then
    # Build using wasm-pack
    echo "Building with wasm-pack..."
    
    # Disable LTO for wasm-pack builds to avoid conflicts
    export CARGO_PROFILE_RELEASE_LTO=false
    export RUSTFLAGS="-C lto=off"
    
    # Add features if specified
    FEATURES_ARG=""
    if [ -n "${CARGO_BUILD_FEATURES:-}" ]; then
        echo "CARGO_BUILD_FEATURES is set to: '$CARGO_BUILD_FEATURES'"
        FEATURES_ARG="--features $CARGO_BUILD_FEATURES"
    else
        echo "CARGO_BUILD_FEATURES is not set, using default features"
        # Explicitly pass default features to ensure they're used
        FEATURES_ARG="--features default"
    fi
    
    echo "Running: wasm-pack build --target $TARGET_TYPE --release --no-opt $FEATURES_ARG"
    wasm-pack build --target "$TARGET_TYPE" --release --no-opt $FEATURES_ARG
else
    # Build using cargo directly
    echo "Building with cargo..."
    
    # Add features if specified
    FEATURES_ARG=""
    if [ -n "${CARGO_BUILD_FEATURES:-}" ]; then
        FEATURES_ARG="--features $CARGO_BUILD_FEATURES"
    fi
    
    cargo build --target wasm32-unknown-unknown --release $FEATURES_ARG \
        --config 'profile.release.panic="abort"' \
        --config 'profile.release.strip=true' \
        --config 'profile.release.debug=false' \
        --config 'profile.release.incremental=false' \
        --config 'profile.release.lto=true' \
        --config 'profile.release.opt-level="z"' \
        --config 'profile.release.codegen-units=1'
    
    # Run wasm-snip if available
    if command -v wasm-snip &> /dev/null; then
        wasm-snip "../../target/wasm32-unknown-unknown/release/${PACKAGE_NAME//-/_}.wasm" \
            -o "../../target/wasm32-unknown-unknown/release/${PACKAGE_NAME//-/_}.wasm" \
            --snip-rust-fmt-code \
            --snip-rust-panicking-code
    fi
    
    # Run wasm-bindgen
    echo "Running wasm-bindgen..."
    if ! command -v wasm-bindgen &> /dev/null; then
        echo "Error: 'wasm-bindgen' not found. Install via 'cargo install wasm-bindgen-cli'." >&2
        exit 1
    fi
    
    wasm-bindgen \
        --typescript \
        --out-dir=pkg \
        --target="$TARGET_TYPE" \
        --omit-default-module-path \
        "../../target/wasm32-unknown-unknown/release/${PACKAGE_NAME//-/_}.wasm"
fi

# Optimize the WASM file
if [ "$OPT_LEVEL" != "none" ] && command -v wasm-opt &> /dev/null; then
    echo "Optimizing wasm using Binaryen (level: $OPT_LEVEL)..."
    
    WASM_PATH="pkg/$WASM_FILE"
    
    if [ "$OPT_LEVEL" = "full" ]; then
        # Check wasm-opt version to determine available options
        WASM_OPT_VERSION=$(wasm-opt --version 2>/dev/null || echo "")
        
        # Core optimization flags that should work with most versions
        CORE_FLAGS=(
            --strip-producers
            -Oz
            --enable-bulk-memory
            --enable-nontrapping-float-to-int
            --flatten
            --rereloop
            -Oz
            --converge
            --vacuum
            --merge-blocks
            --simplify-locals
            --remove-unused-brs
            --remove-unused-module-elements
            --remove-unused-names
            -Oz
            -Oz
        )
        
        # Additional flags to test for compatibility
        OPTIONAL_FLAGS=(
            "--code-folding"
            "--const-hoisting"
            "--dce"
            "-tnh"
            "--gsi"
            "--inlining-optimizing"
            "--optimize-added-constants"
            "--optimize-casts"
            "--optimize-instructions"
            "--optimize-stack-ir"
            "--remove-unused-types"
            "--post-emscripten"
            "--generate-global-effects"
            "--abstract-type-refining"
        )
        
        # Test which optional flags are supported
        SUPPORTED_FLAGS=()
        for flag in "${OPTIONAL_FLAGS[@]}"; do
            if wasm-opt "$flag" "$WASM_PATH" -o /dev/null 2>/dev/null; then
                SUPPORTED_FLAGS+=("$flag")
            else
                echo "Note: $flag not supported by this wasm-opt version, skipping..."
            fi
        done
        
        # Run optimization with core flags and any supported optional flags
        wasm-opt \
            "${CORE_FLAGS[@]}" \
            "${SUPPORTED_FLAGS[@]}" \
            "$WASM_PATH" \
            -o \
            "$WASM_PATH"
            
        # Create optimized version for wasm-sdk
        if [ "$PACKAGE_NAME" = "wasm-sdk" ]; then
            cp "$WASM_PATH" "pkg/optimized.wasm"
        fi
    else
        # Minimal optimization for development builds
        # Explicitly enable features used by newer toolchains:
        # - bulk memory (memory.copy)
        # - non-trapping float-to-int (i32/i64.trunc_sat_fXX_[su])
        wasm-opt \
            --strip-producers \
            -O2 \
            --enable-bulk-memory \
            --enable-nontrapping-float-to-int \
            "$WASM_PATH" \
            -o \
            "$WASM_PATH"
    fi
else
    if [ "$OPT_LEVEL" != "none" ]; then
        echo "wasm-opt command not found. Skipping wasm optimization."
    fi
fi

echo "Build complete!"
echo "Output files are in the pkg/ directory"
ls -lah pkg/
