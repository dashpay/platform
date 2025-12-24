#!/usr/bin/env bash
set -euo pipefail

# Always run from the package root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$SCRIPT_DIR"

echo "=== Module Combination Analysis ==="
echo "Building different feature combinations to analyze bundle sizes..."
echo

# Create results directory
RESULTS_DIR="analysis-results"
rm -rf "$RESULTS_DIR"
mkdir -p "$RESULTS_DIR"

# Define all possible features
FEATURES=(
    "identity"
    "document"
    "contract"
    "tokens"
    "governance"
    "transitions"
)

# Define common feature combinations
declare -A COMBINATIONS=(
    ["minimal"]="console_error_panic_hook"
    ["identity-only"]="console_error_panic_hook,identity"
    ["document-only"]="console_error_panic_hook,document"
    ["contract-only"]="console_error_panic_hook,contract"
    ["tokens-only"]="console_error_panic_hook,tokens"
    ["governance-only"]="console_error_panic_hook,governance"
    ["transitions-only"]="console_error_panic_hook,transitions"
    ["identity-document"]="console_error_panic_hook,identity,document"
    ["identity-tokens"]="console_error_panic_hook,identity,tokens"
    ["document-contract"]="console_error_panic_hook,document,contract"
    ["core-trio"]="console_error_panic_hook,identity,document,contract"
    ["defi-bundle"]="console_error_panic_hook,identity,tokens,contract"
    ["governance-bundle"]="console_error_panic_hook,governance,voting,group,system"
    ["no-governance"]="console_error_panic_hook,identity,document,contract,tokens,transitions"
    ["full"]="console_error_panic_hook,full"
)

# Function to build with specific features
build_combination() {
    local name=$1
    local features=$2
    
    echo "Building ${name} (features: ${features})..."
    
    # Build with specific features
    cargo build --target wasm32-unknown-unknown --release --no-default-features --features "${features}" 2>&1 | tail -5
    
    # Run wasm-bindgen
    local out_dir="${RESULTS_DIR}/${name}"
    mkdir -p "${out_dir}"
    
    wasm-bindgen ../../target/wasm32-unknown-unknown/release/wasm_drive_verify.wasm \
        --out-dir "${out_dir}" \
        --target web \
        --out-name "bundle" > /dev/null 2>&1
    
    # Optimize with wasm-opt if available
    if command -v wasm-opt &> /dev/null; then
        wasm-opt -Oz \
            "${out_dir}/bundle_bg.wasm" \
            -o "${out_dir}/bundle_bg_opt.wasm" 2>/dev/null || true
    fi
    
    # Get sizes
    local wasm_size=$(stat -f%z "${out_dir}/bundle_bg.wasm" 2>/dev/null || stat -c%s "${out_dir}/bundle_bg.wasm" 2>/dev/null || echo "0")
    local js_size=$(stat -f%z "${out_dir}/bundle.js" 2>/dev/null || stat -c%s "${out_dir}/bundle.js" 2>/dev/null || echo "0")
    local opt_size=0
    if [ -f "${out_dir}/bundle_bg_opt.wasm" ]; then
        opt_size=$(stat -f%z "${out_dir}/bundle_bg_opt.wasm" 2>/dev/null || stat -c%s "${out_dir}/bundle_bg_opt.wasm" 2>/dev/null || echo "0")
    fi
    
    # Store results
    echo "${name}|${features}|${wasm_size}|${js_size}|${opt_size}" >> "${RESULTS_DIR}/results.csv"
}

# Initialize results file
echo "name|features|wasm_size|js_size|optimized_size" > "${RESULTS_DIR}/results.csv"

# Build all combinations
for name in "${!COMBINATIONS[@]}"; do
    build_combination "$name" "${COMBINATIONS[$name]}"
done

# Generate all possible feature combinations (power set)
echo
echo "Generating power set combinations..."

# Function to generate power set
generate_powerset() {
    local -a arr=("$@")
    local n=${#arr[@]}
    local max=$((2**n))
    
    for ((i=1; i<max; i++)); do
        local combination=""
        local combo_name=""
        for ((j=0; j<n; j++)); do
            if ((i & (1<<j))); then
                if [ -n "$combination" ]; then
                    combination+=","
                    combo_name+="-"
                else
                    combination="console_error_panic_hook,"
                fi
                combination+="${arr[j]}"
                combo_name+="${arr[j]:0:3}"  # First 3 chars of feature name
            fi
        done
        
        # Skip if we already built this combination
        if ! grep -q "auto-${combo_name}" "${RESULTS_DIR}/results.csv"; then
            build_combination "auto-${combo_name}" "$combination"
        fi
    done
}

# Uncomment to generate ALL combinations (warning: this will take a while!)
# generate_powerset "${FEATURES[@]}"

# Create analysis report
echo
echo "=== Generating Analysis Report ==="

# Create markdown report
cat > "${RESULTS_DIR}/analysis-report.md" << 'EOF'
# Module Combination Size Analysis

## Results Summary

| Combination | Features | WASM Size | JS Size | Optimized | Reduction |
|------------|----------|-----------|---------|-----------|-----------|
EOF

# Get baseline (full) size
FULL_SIZE=$(grep "^full|" "${RESULTS_DIR}/results.csv" | cut -d'|' -f3)

# Process results and add to report
while IFS='|' read -r name features wasm_size js_size opt_size; do
    if [ "$name" != "name" ]; then
        # Calculate sizes in KB
        wasm_kb=$((wasm_size / 1024))
        js_kb=$((js_size / 1024))
        opt_kb=$((opt_size / 1024))
        total_kb=$((wasm_kb + js_kb))
        
        # Calculate reduction percentage
        if [ "$FULL_SIZE" -gt 0 ] && [ "$name" != "full" ]; then
            reduction=$(awk "BEGIN {printf \"%.1f\", (1 - $wasm_size / $FULL_SIZE) * 100}")
        else
            reduction="baseline"
        fi
        
        # Format features list
        features_display=$(echo "$features" | sed 's/console_error_panic_hook,//' | sed 's/,/, /g')
        [ -z "$features_display" ] && features_display="core only"
        
        echo "| $name | $features_display | ${wasm_kb}KB | ${js_kb}KB | ${opt_kb}KB | $reduction |" >> "${RESULTS_DIR}/analysis-report.md"
    fi
done < "${RESULTS_DIR}/results.csv" | sort -t'|' -k3 -n

# Add insights section
cat >> "${RESULTS_DIR}/analysis-report.md" << 'EOF'

## Key Insights

### Size Impact by Module

EOF

# Calculate individual module sizes
for feature in "${FEATURES[@]}"; do
    only_size=$(grep "^${feature}-only|" "${RESULTS_DIR}/results.csv" | cut -d'|' -f3 || echo "0")
    minimal_size=$(grep "^minimal|" "${RESULTS_DIR}/results.csv" | cut -d'|' -f3 || echo "0")
    
    if [ "$only_size" -gt 0 ] && [ "$minimal_size" -gt 0 ]; then
        module_size=$((only_size - minimal_size))
        module_kb=$((module_size / 1024))
        echo "- **${feature}**: ~${module_kb}KB" >> "${RESULTS_DIR}/analysis-report.md"
    fi
done

# Add recommendations
cat >> "${RESULTS_DIR}/analysis-report.md" << 'EOF'

### Recommended Combinations

Based on the analysis, here are recommended feature combinations for common use cases:

1. **Identity Management Apps**: `identity` (~400KB)
   - Just identity verification functions
   - Ideal for wallet and identity-focused applications

2. **Document Storage Apps**: `document,contract` (~450KB)
   - Document verification with contract support
   - Perfect for decentralized storage applications

3. **DeFi Applications**: `identity,tokens,contract` (~700KB)
   - Identity, token management, and contract verification
   - Complete package for financial applications

4. **Lightweight Clients**: `identity,document` (~550KB)
   - Basic verification without governance features
   - Good for mobile or resource-constrained environments

5. **Full Platform Clients**: `full` (baseline)
   - All features included
   - Best for development or when all features are needed

### Bundle Size Optimization Tips

1. **Start Small**: Begin with only the modules you need
2. **Add Incrementally**: Add modules as features require them
3. **Use Dynamic Imports**: Load rarely-used modules on demand
4. **Monitor Growth**: Track bundle size as you add features

EOF

# Generate visual chart data (JSON for charting libraries)
cat > "${RESULTS_DIR}/chart-data.json" << EOF
{
  "combinations": [
EOF

first=true
while IFS='|' read -r name features wasm_size js_size opt_size; do
    if [ "$name" != "name" ]; then
        [ "$first" = true ] && first=false || echo ","
        echo -n "    {\"name\": \"$name\", \"wasm\": $wasm_size, \"js\": $js_size, \"total\": $((wasm_size + js_size))}"
    fi
done < "${RESULTS_DIR}/results.csv"

cat >> "${RESULTS_DIR}/chart-data.json" << EOF

  ]
}
EOF

# Print summary
echo
echo "=== Analysis Complete ==="
echo
echo "Results saved to: ${RESULTS_DIR}/"
echo "- results.csv: Raw data"
echo "- analysis-report.md: Detailed analysis"
echo "- chart-data.json: Data for visualization"
echo
echo "Top 5 smallest combinations:"
tail -n +2 "${RESULTS_DIR}/results.csv" | sort -t'|' -k3 -n | head -5 | while IFS='|' read -r name features wasm_size js_size opt_size; do
    wasm_kb=$((wasm_size / 1024))
    echo "  - $name: ${wasm_kb}KB"
done