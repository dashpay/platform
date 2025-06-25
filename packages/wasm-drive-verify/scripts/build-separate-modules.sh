#!/usr/bin/env bash
set -euo pipefail

# Always run from the package root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$SCRIPT_DIR"

echo "Building separate WASM modules for wasm-drive-verify..."

# Clean previous builds
echo "Cleaning previous builds..."
rm -rf pkg-* dist

# Create directories
mkdir -p dist

# Function to build a specific module
build_module() {
    local module_name=$1
    local features=$2
    local out_dir="pkg-${module_name}"
    
    echo "Building ${module_name} module with features: ${features}..."
    
    # Build with specific features
    cargo build --target wasm32-unknown-unknown --release --no-default-features --features "${features}"
    
    # Create output directory
    mkdir -p "${out_dir}"
    
    # Run wasm-bindgen
    wasm-bindgen ../../target/wasm32-unknown-unknown/release/wasm_drive_verify.wasm \
        --out-dir "${out_dir}" \
        --target web \
        --out-name "wasm_drive_verify_${module_name}"
    
    # Optimize with wasm-opt if available
    if command -v wasm-opt &> /dev/null; then
        echo "Optimizing ${module_name} module with wasm-opt..."
        wasm-opt -Oz \
            "${out_dir}/wasm_drive_verify_${module_name}_bg.wasm" \
            -o "${out_dir}/wasm_drive_verify_${module_name}_bg.wasm"
    fi
    
    # Get module size
    local size=$(ls -lh "${out_dir}/wasm_drive_verify_${module_name}_bg.wasm" | awk '{print $5}')
    echo "Module ${module_name} size: ${size}"
}

# Build each module separately
build_module "core" "console_error_panic_hook"
build_module "identity" "console_error_panic_hook,identity"
build_module "document" "console_error_panic_hook,document"
build_module "contract" "console_error_panic_hook,contract"
build_module "tokens" "console_error_panic_hook,tokens"
build_module "governance" "console_error_panic_hook,governance"
build_module "transitions" "console_error_panic_hook,transitions"

# Build the full module for comparison
echo "Building full module for comparison..."
build_module "full" "console_error_panic_hook,full"

# Create a size comparison report
echo -e "\n=== Module Size Report ===" > dist/size-report.txt
for module in core identity document contract tokens governance transitions full; do
    if [ -f "pkg-${module}/wasm_drive_verify_${module}_bg.wasm" ]; then
        size=$(ls -lh "pkg-${module}/wasm_drive_verify_${module}_bg.wasm" | awk '{print $5}')
        echo "${module}: ${size}" >> dist/size-report.txt
    fi
done

echo -e "\nModule size report:"
cat dist/size-report.txt

echo -e "\nModular build complete!"
echo "Separate modules are available in pkg-* directories"