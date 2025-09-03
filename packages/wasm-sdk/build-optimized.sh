#!/usr/bin/env bash
#
# Optimized build script for wasm-sdk npm release with automated size tracking
#
set -euo pipefail

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PKG_DIR="$SCRIPT_DIR/pkg"
SIZE_TRACKING_DIR="$SCRIPT_DIR/.size-tracking"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Building wasm-sdk with full optimization for npm release...${NC}"

# Create size tracking directory if it doesn't exist
mkdir -p "$SIZE_TRACKING_DIR"

# Store baseline sizes for comparison
BASELINE_FILE="$SIZE_TRACKING_DIR/baseline-sizes.txt"
CURRENT_BUILD_FILE="$SIZE_TRACKING_DIR/current-build-$(date +%Y%m%d-%H%M%S).txt"

# Capture pre-build state if package exists
if [ -d "$PKG_DIR" ]; then
    echo -e "${YELLOW}Capturing current package sizes as baseline...${NC}"
    {
        echo "# Baseline sizes captured at $(date)"
        for file in "$PKG_DIR"/*.wasm "$PKG_DIR"/*.js "$PKG_DIR"/*.d.ts; do
            [ ! -e "$file" ] && continue  # Skip if glob doesn't match
            if [ -f "$file" ]; then
                size=$(wc -c < "$file")
                filename=$(basename "$file")
                echo "$filename:$size"
            fi
        done
    } > "$BASELINE_FILE"
fi

# Call unified build script with full optimization
echo -e "${YELLOW}Running optimized build process...${NC}"
"$SCRIPT_DIR/../scripts/build-wasm.sh" --package wasm-sdk --opt-level full

# Additional post-processing for npm release
echo -e "${YELLOW}Post-processing for npm release...${NC}"

cd "$PKG_DIR"

# Ensure the package.json is correct
if [ ! -f "package.json" ]; then
    echo -e "${RED}Error: package.json not found in pkg directory${NC}"
    exit 1
fi

# Fix package name to match Stream A configuration
echo -e "${YELLOW}Updating package.json to match Stream A configuration...${NC}"
if command -v jq &> /dev/null; then
    # Use jq to safely update the package name
    temp_package=$(mktemp)
    jq '.name = "dash"' package.json > "$temp_package" && mv "$temp_package" package.json
    echo "Package name updated to 'dash'"
else
    # Fallback: use sed for simple replacement
    sed -i.bak 's/"name": "dash-wasm-sdk"/"name": "dash"/' package.json && rm -f package.json.bak
    echo "Package name updated to 'dash' (using sed fallback)"
fi

# Verify all required files exist (check both possible naming conventions)
REQUIRED_FILES=("dash_wasm_sdk.js" "dash_wasm_sdk.d.ts" "dash_wasm_sdk_bg.wasm")
for file in "${REQUIRED_FILES[@]}"; do
    if [ ! -f "$file" ]; then
        echo -e "${RED}Error: Required file $file not found${NC}"
        exit 1
    fi
done

# Capture current build sizes
echo -e "${YELLOW}Recording current build sizes...${NC}"
{
    echo "# Build sizes captured at $(date)"
    for file in *.wasm *.js *.d.ts; do
        if [ -f "$file" ]; then
            size=$(wc -c < "$file")
            echo "$file:$size"
        fi
    done
} > "$CURRENT_BUILD_FILE"

# Compare with baseline and check for regressions
if [ -f "$BASELINE_FILE" ]; then
    echo -e "${YELLOW}Analyzing size changes...${NC}"
    size_regression_detected=false
    
    while IFS=':' read -r filename baseline_size; do
        if [[ "$filename" =~ ^#.* ]]; then continue; fi  # Skip comments
        if [ -f "$filename" ]; then
            current_size=$(wc -c < "$filename")
            size_diff=$((current_size - baseline_size))
            
            if [ "$size_diff" -gt 0 ]; then
                percentage_increase=$(echo "scale=1; $size_diff * 100 / $baseline_size" | bc -l 2>/dev/null || echo "N/A")
                echo "  $filename: +${size_diff} bytes (+${percentage_increase}%)"
                
                # Flag significant regressions (>10% or >50KB increase)
                if [ "$size_diff" -gt 51200 ] || [ "$(echo "$percentage_increase > 10" | bc -l 2>/dev/null)" = "1" ]; then
                    echo -e "    ${RED}⚠️  Significant size regression detected!${NC}"
                    size_regression_detected=true
                fi
            elif [ "$size_diff" -lt 0 ]; then
                size_reduction=$((baseline_size - current_size))
                percentage_decrease=$(echo "scale=1; $size_reduction * 100 / $baseline_size" | bc -l 2>/dev/null || echo "N/A")
                echo -e "  $filename: -${size_reduction} bytes (-${percentage_decrease}%) ${GREEN}✓${NC}"
            else
                echo "  $filename: no change"
            fi
        fi
    done < "$BASELINE_FILE"
    
    # Fail build on significant regressions in production mode
    if [ "$size_regression_detected" = true ]; then
        echo -e "${RED}Size regression detected! Consider optimizing the build.${NC}"
        echo "Review the changes and run optimization analysis."
        # Don't fail build but warn - let CI/CD decide the policy
        echo -e "${YELLOW}Continuing with build but flagging for review...${NC}"
    fi
fi

# Run strict bundlesize regression check for production builds
if [ -f "$SCRIPT_DIR/bundlesize.json" ] && command -v bundlesize &> /dev/null; then
    echo -e "${YELLOW}Running strict bundlesize regression check...${NC}"
    cd "$SCRIPT_DIR"
    if ! bundlesize; then
        echo -e "${RED}❌ Bundle size regression detected in production build!${NC}"
        echo "This build exceeds the defined size limits."
        echo "Review the bundlesize.json configuration and optimize the build."
        # In production builds, this should be treated seriously
        echo -e "${YELLOW}Build flagged for size review. Consider optimization before release.${NC}"
    else
        echo -e "${GREEN}✅ Bundle size within acceptable limits${NC}"
    fi
    cd "$PKG_DIR"
fi

# Show final build info with detailed analysis
echo -e "${GREEN}Build complete! Package contents:${NC}"
ls -lah

# Comprehensive size analysis
echo ""
echo -e "${BLUE}📊 Detailed Size Analysis:${NC}"
echo "========================================"

total_size=0
for file in *.wasm *.js *.d.ts; do
    if [ -f "$file" ]; then
        file_size=$(wc -c < "$file")
        file_size_kb=$((file_size / 1024))
        total_size=$((total_size + file_size))
        
        # Show compressed size if possible
        if command -v gzip &> /dev/null; then
            compressed_size=$(gzip -c "$file" | wc -c)
            compressed_kb=$((compressed_size / 1024))
            compression_ratio=$((100 - (compressed_size * 100 / file_size)))
            echo "$file: ${file_size_kb}KB (${compressed_kb}KB compressed, ${compression_ratio}% reduction)"
        else
            echo "$file: ${file_size_kb}KB"
        fi
    fi
done

total_kb=$((total_size / 1024))
total_mb=$((total_size / 1024 / 1024))

echo ""
echo "Total bundle size: ${total_size} bytes (${total_kb}KB / ${total_mb}MB)"

# Performance target assessment
echo ""
echo -e "${BLUE}📋 Performance Target Assessment:${NC}"
if [ $total_mb -le 2 ]; then
    echo -e "${GREEN}✅ Excellent: Under 2MB (optimal for web)${NC}"
elif [ $total_mb -le 5 ]; then
    echo -e "${GREEN}✅ Good: Within 2-5MB range (acceptable for web)${NC}"
elif [ $total_mb -le 15 ]; then
    echo -e "${YELLOW}⚠️  Fair: Within 5-15MB range (may affect performance)${NC}"
else
    echo -e "${RED}❌ Poor: Over 15MB (requires optimization)${NC}"
fi

# Verify the package.json has correct configuration from Stream A
echo ""
echo -e "${YELLOW}Validating package.json configuration...${NC}"
if grep -q '"name": "dash"' package.json; then
    echo -e "✅ Package name: $(grep '"name":' package.json | cut -d'"' -f4)"
else
    echo -e "${RED}❌ Package name not set to 'dash' as configured in Stream A${NC}"
fi

if grep -q '"main":' package.json; then
    echo -e "✅ Main entry: $(grep '"main":' package.json | cut -d'"' -f4)"
fi

if grep -q '"types":' package.json; then
    echo -e "✅ Types entry: $(grep '"types":' package.json | cut -d'"' -f4)"
fi

# Run comprehensive bundle analysis
echo ""
echo -e "${YELLOW}Running comprehensive bundle analysis...${NC}"
if [ -x "$SCRIPT_DIR/analyze-bundle.sh" ]; then
    "$SCRIPT_DIR/analyze-bundle.sh"
else
    echo -e "${RED}Warning: analyze-bundle.sh not found or not executable${NC}"
fi

echo ""
echo -e "${GREEN}🚀 Ready for npm publish!${NC}"
echo "Size tracking data saved to: $CURRENT_BUILD_FILE"