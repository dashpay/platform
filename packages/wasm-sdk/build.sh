#!/usr/bin/env bash
#
# Build WASM-SDK using unified build script with bundle size monitoring
#
# EXPERIMENTAL: This script is experimental and may be removed in the future.
#

set -euo pipefail

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PKG_DIR="$SCRIPT_DIR/pkg"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Building WASM SDK with bundle size monitoring...${NC}"

# Store pre-build state for comparison
PRE_BUILD_SIZES=()
if [ -d "$PKG_DIR" ]; then
    echo -e "${YELLOW}Capturing pre-build bundle sizes...${NC}"
    for file in "$PKG_DIR"/*.wasm "$PKG_DIR"/*.js; do
        [ ! -e "$file" ] && continue  # Skip if glob doesn't match
        if [ -f "$file" ]; then
            size=$(wc -c < "$file" 2>/dev/null || echo 0)
            PRE_BUILD_SIZES+=("$(basename "$file"):$size")
        fi
    done
fi

# Determine optimization level based on environment
OPT_LEVEL="full"
if [ "${CARGO_BUILD_PROFILE:-}" = "dev" ] || [ "${CI:-}" != "true" ]; then
    OPT_LEVEL="minimal"
fi

# Call unified build script with default features (no need to specify)
echo -e "${YELLOW}Running core build process...${NC}"
"$SCRIPT_DIR/../scripts/build-wasm.sh" --package wasm-sdk --opt-level "$OPT_LEVEL"

# Post-build size analysis and monitoring
echo -e "${YELLOW}Running post-build size analysis...${NC}"

cd "$PKG_DIR"

# Basic size validation
echo "Validating build output..."
REQUIRED_FILES=("wasm_sdk.js" "wasm_sdk.d.ts" "wasm_sdk_bg.wasm")
for file in "${REQUIRED_FILES[@]}"; do
    if [ ! -f "$file" ]; then
        echo "Error: Required file $file not found"
        exit 1
    fi
done

# Compare sizes if we had pre-build data
if [ ${#PRE_BUILD_SIZES[@]} -gt 0 ]; then
    echo -e "${YELLOW}Size comparison:${NC}"
    for pre_entry in "${PRE_BUILD_SIZES[@]}"; do
        filename="${pre_entry%:*}"
        pre_size="${pre_entry#*:}"
        if [ -f "$filename" ]; then
            new_size=$(wc -c < "$filename")
            if [ "$new_size" -gt "$pre_size" ]; then
                size_change=$((new_size - pre_size))
                echo "  $filename: +${size_change} bytes ($(echo "scale=1; $size_change*100/$pre_size" | bc -l 2>/dev/null || echo "N/A")%)"
            elif [ "$new_size" -lt "$pre_size" ]; then
                size_change=$((pre_size - new_size))
                echo "  $filename: -${size_change} bytes (-$(echo "scale=1; $size_change*100/$pre_size" | bc -l 2>/dev/null || echo "N/A")%)"
            else
                echo "  $filename: no change"
            fi
        fi
    done
fi

# Quick bundle size check using bundle-size tool if available
if command -v bundle-size &> /dev/null; then
    echo -e "${YELLOW}Running bundle-size analysis...${NC}"
    for file in *.wasm *.js; do
        if [ -f "$file" ]; then
            echo "  Analyzing $file:"
            bundle-size "$file" 2>/dev/null || echo "    Failed to analyze $file"
        fi
    done
fi

# Run bundlesize regression check if config exists
if [ -f "$SCRIPT_DIR/bundlesize.json" ] && command -v bundlesize &> /dev/null; then
    echo -e "${YELLOW}Running bundlesize regression check...${NC}"
    cd "$SCRIPT_DIR"
    bundlesize || {
        echo -e "${YELLOW}Warning: Bundle size regression detected. Review size increases.${NC}"
        # Don't fail the build for size warnings in development builds
        if [ "$OPT_LEVEL" = "full" ]; then
            echo "Consider running build-optimized.sh for production builds with strict size limits."
        fi
    }
    cd "$PKG_DIR"
fi

# Show final bundle info
# Copy JavaScript wrapper files
echo -e "${YELLOW}Integrating JavaScript wrapper files...${NC}"
if [ -d "../src-js" ]; then
    echo "Copying JavaScript wrapper to pkg directory..."
    
    # Copy wrapper files
    cp ../src-js/*.js . 2>/dev/null || echo "No .js wrapper files found"
    cp ../src-js/*.d.ts . 2>/dev/null || echo "No .d.ts wrapper files found"
    
    # Update package.json to use wrapper as entry point
    if [ -f "package.json" ] && command -v node >/dev/null 2>&1; then
        node -e "
        const fs = require('fs');
        const pkg = JSON.parse(fs.readFileSync('package.json', 'utf8'));
        if (fs.existsSync('index.js')) {
            pkg.main = 'index.js';
            pkg.module = 'index.js'; 
            if (fs.existsSync('types.d.ts')) {
                pkg.types = 'types.d.ts';
            }
            pkg.files = pkg.files || [];
            const wrapperFiles = ['index.js', 'config-manager.js', 'resource-manager.js', 'error-handler.js'];
            if (fs.existsSync('types.d.ts')) wrapperFiles.push('types.d.ts');
            wrapperFiles.forEach(file => {
                if (fs.existsSync(file) && !pkg.files.includes(file)) {
                    pkg.files.push(file);
                }
            });
            fs.writeFileSync('package.json', JSON.stringify(pkg, null, 2) + '\n');
            console.log('  ✅ Package configured to use JavaScript wrapper as entry point');
        } else {
            console.log('  ℹ️  JavaScript wrapper not found, using WASM bindings');
        }
        "
    fi
    
    echo -e "${GREEN}JavaScript wrapper integration complete!${NC}"
else
    echo -e "${YELLOW}No JavaScript wrapper found (src-js directory missing)${NC}"
fi

echo -e "${GREEN}Build complete! Final bundle info:${NC}"
ls -lah *.wasm *.js *.d.ts

# Calculate total bundle size
total_size=0
for file in *.wasm *.js *.d.ts; do
    if [ -f "$file" ]; then
        file_size=$(wc -c < "$file")
        total_size=$((total_size + file_size))
    fi
done

total_kb=$((total_size / 1024))
echo "Total bundle size: ${total_size} bytes (${total_kb}KB)"

echo -e "${GREEN}✅ WASM SDK build completed successfully!${NC}"
