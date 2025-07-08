#!/usr/bin/env bash
set -euo pipefail

# Quick size check for common module combinations
# This script builds a few key combinations and reports their sizes

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$SCRIPT_DIR"

echo "=== Quick Module Size Check ==="
echo

# Define key combinations to check
declare -A COMBOS=(
    ["base"]="console_error_panic_hook"
    ["identity"]="console_error_panic_hook,identity"
    ["document"]="console_error_panic_hook,document"
    ["defi"]="console_error_panic_hook,identity,tokens,contract"
    ["lite"]="console_error_panic_hook,identity,document"
    ["full"]="console_error_panic_hook,full"
)

# Temporary directory for builds
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# Function to format bytes
format_bytes() {
    local bytes=$1
    local kb=$((bytes / 1024))
    local mb=$((kb / 1024))
    
    if [ $mb -gt 0 ]; then
        echo "${mb}MB"
    else
        echo "${kb}KB"
    fi
}

# Build and measure each combination
echo "Building combinations..."
echo

RESULTS=()
FULL_SIZE=0

for name in "${!COMBOS[@]}"; do
    features="${COMBOS[$name]}"
    
    # Build
    if cargo build --target wasm32-unknown-unknown --release --no-default-features --features "$features" >/dev/null 2>&1; then
        # Run wasm-bindgen
        OUT_DIR="$TEMP_DIR/$name"
        mkdir -p "$OUT_DIR"
        
        if wasm-bindgen ../../target/wasm32-unknown-unknown/release/wasm_drive_verify.wasm \
            --out-dir "$OUT_DIR" \
            --target web \
            --out-name bundle >/dev/null 2>&1; then
            
            # Get size
            SIZE=$(stat -f%z "$OUT_DIR/bundle_bg.wasm" 2>/dev/null || stat -c%s "$OUT_DIR/bundle_bg.wasm" 2>/dev/null || echo "0")
            
            if [ "$name" = "full" ]; then
                FULL_SIZE=$SIZE
            fi
            
            RESULTS+=("$name|$SIZE")
            
            echo "✓ $name: $(format_bytes $SIZE)"
        else
            echo "✗ $name: wasm-bindgen failed"
        fi
    else
        echo "✗ $name: build failed"
    fi
done

# Show comparison table
echo
echo "=== Size Comparison ==="
echo
printf "%-15s %-10s %s\n" "Module" "Size" "Reduction"
printf "%-15s %-10s %s\n" "------" "----" "---------"

# Sort results by size
IFS=$'\n' SORTED=($(sort -t'|' -k2 -n <<<"${RESULTS[*]}"))

for result in "${SORTED[@]}"; do
    name=$(echo "$result" | cut -d'|' -f1)
    size=$(echo "$result" | cut -d'|' -f2)
    
    if [ "$FULL_SIZE" -gt 0 ] && [ "$name" != "full" ]; then
        reduction=$(awk "BEGIN {printf \"%.1f%%\", (1 - $size / $FULL_SIZE) * 100}")
    else
        reduction="baseline"
    fi
    
    printf "%-15s %-10s %s\n" "$name" "$(format_bytes $size)" "$reduction"
done

# Quick recommendations
echo
echo "=== Quick Recommendations ==="
echo
echo "• For identity-only apps: Use 'identity' module (~$(awk "BEGIN {printf \"%.0f%%\", (1 - $(echo "${COMBOS[identity]}" | wc -c) / $(echo "${COMBOS[full]}" | wc -c)) * 100}") smaller)"
echo "• For DeFi apps: Use 'identity,tokens,contract' combination"
echo "• For general use: 'identity,document' provides good balance"
echo "• Always use dynamic imports for rarely-used features"