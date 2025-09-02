#!/usr/bin/env bash
#
# Bundle size analysis script for wasm-sdk
#
set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}WASM SDK Bundle Analysis${NC}"
echo "========================================"

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PKG_DIR="$SCRIPT_DIR/pkg"

if [ ! -d "$PKG_DIR" ]; then
    echo -e "${RED}Error: pkg directory not found. Run build script first.${NC}"
    exit 1
fi

cd "$PKG_DIR"

echo ""
echo -e "${YELLOW}📊 File Sizes:${NC}"
echo "----------------------------------------"
ls -lah *.wasm *.js *.ts 2>/dev/null || echo "Some files not found"

echo ""
echo -e "${YELLOW}🗜️  WASM File Analysis:${NC}"
echo "----------------------------------------"

for wasm_file in *.wasm; do
    if [ -f "$wasm_file" ]; then
        # Basic file size info
        size_bytes=$(wc -c < "$wasm_file")
        size_kb=$((size_bytes / 1024))
        size_mb=$((size_bytes / 1024 / 1024))
        
        echo "File: $wasm_file"
        echo "  Size: ${size_bytes} bytes (${size_kb}KB / ${size_mb}MB)"
        
        # Use wasm-dis to analyze sections if available
        if command -v wasm-dis &> /dev/null; then
            echo "  Sections:"
            wasm-dis "$wasm_file" | head -20 | grep -E "(section|import|export)" | sed 's/^/    /' || echo "    Analysis failed"
        fi
        
        # Compression analysis
        if command -v gzip &> /dev/null; then
            compressed_size=$(gzip -c "$wasm_file" | wc -c)
            compressed_kb=$((compressed_size / 1024))
            compression_ratio=$((100 - (compressed_size * 100 / size_bytes)))
            echo "  Gzip compressed: ${compressed_size} bytes (${compressed_kb}KB) - ${compression_ratio}% reduction"
        fi
        
        echo ""
    fi
done

echo -e "${YELLOW}📦 JavaScript Bundle Analysis:${NC}"
echo "----------------------------------------"

for js_file in *.js; do
    if [ -f "$js_file" ]; then
        js_size_bytes=$(wc -c < "$js_file")
        js_size_kb=$((js_size_bytes / 1024))
        echo "File: $js_file"
        echo "  Size: ${js_size_bytes} bytes (${js_size_kb}KB)"
        
        if command -v gzip &> /dev/null; then
            js_compressed_size=$(gzip -c "$js_file" | wc -c)
            js_compressed_kb=$((js_compressed_size / 1024))
            js_compression_ratio=$((100 - (js_compressed_size * 100 / js_size_bytes)))
            echo "  Gzip compressed: ${js_compressed_size} bytes (${js_compressed_kb}KB) - ${js_compression_ratio}% reduction"
        fi
        echo ""
    fi
done

echo -e "${YELLOW}📋 Package.json Analysis:${NC}"
echo "----------------------------------------"
if [ -f "package.json" ]; then
    echo "Package name: $(jq -r '.name // "not set"' package.json 2>/dev/null || echo "jq not available")"
    echo "Version: $(jq -r '.version // "not set"' package.json 2>/dev/null || echo "jq not available")"
    echo "Main entry: $(jq -r '.main // "not set"' package.json 2>/dev/null || echo "jq not available")"
    echo "Types entry: $(jq -r '.types // "not set"' package.json 2>/dev/null || echo "jq not available")"
else
    echo -e "${RED}package.json not found${NC}"
fi

echo ""
echo -e "${YELLOW}📊 Total Bundle Size Summary:${NC}"
echo "----------------------------------------"
total_size=0
for file in *.wasm *.js *.d.ts; do
    if [ -f "$file" ]; then
        file_size=$(wc -c < "$file" 2>/dev/null || echo 0)
        total_size=$((total_size + file_size))
    fi
done

total_kb=$((total_size / 1024))
total_mb=$((total_size / 1024 / 1024))

echo "Total uncompressed: ${total_size} bytes (${total_kb}KB / ${total_mb}MB)"

if command -v gzip &> /dev/null; then
    total_compressed=0
    for file in *.wasm *.js *.d.ts; do
        if [ -f "$file" ]; then
            file_compressed=$(gzip -c "$file" 2>/dev/null | wc -c || echo 0)
            total_compressed=$((total_compressed + file_compressed))
        fi
    done
    total_compressed_kb=$((total_compressed / 1024))
    total_compressed_mb=$((total_compressed / 1024 / 1024))
    total_compression_ratio=$((100 - (total_compressed * 100 / total_size)))
    
    echo "Total compressed: ${total_compressed} bytes (${total_compressed_kb}KB / ${total_compressed_mb}MB) - ${total_compression_ratio}% reduction"
fi

echo ""
echo -e "${GREEN}✅ Analysis complete!${NC}"
echo ""
echo -e "${BLUE}📝 Performance Targets Assessment:${NC}"
echo "----------------------------------------"
if [ $total_mb -le 2 ]; then
    echo -e "${GREEN}✅ Under 2MB target (optimistic)${NC}"
elif [ $total_mb -le 5 ]; then
    echo -e "${YELLOW}⚠️  Within 2-5MB range (realistic)${NC}"
elif [ $total_mb -le 15 ]; then
    echo -e "${YELLOW}⚠️  Within 5-15MB range (acceptable)${NC}"
else
    echo -e "${RED}❌ Over 15MB (needs optimization)${NC}"
fi

if command -v gzip &> /dev/null && [ $total_compressed_mb -le 5 ]; then
    echo -e "${GREEN}✅ Compressed size meets targets${NC}"
fi