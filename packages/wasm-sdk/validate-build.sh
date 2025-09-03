#!/usr/bin/env bash
#
# Build validation script for wasm-sdk npm package
#
set -euo pipefail

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PKG_DIR="$SCRIPT_DIR/pkg"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}WASM SDK Build Validation${NC}"
echo "========================================"

# Check if pkg directory exists
if [ ! -d "$PKG_DIR" ]; then
    echo -e "${RED}❌ Error: pkg directory not found. Run build script first.${NC}"
    exit 1
fi

cd "$PKG_DIR"

validation_errors=0
validation_warnings=0

echo ""
echo -e "${YELLOW}🔍 File Structure Validation:${NC}"
echo "----------------------------------------"

# Validate required files exist
REQUIRED_FILES=("wasm_sdk.js" "wasm_sdk.d.ts" "wasm_sdk_bg.wasm" "package.json")
for file in "${REQUIRED_FILES[@]}"; do
    if [ -f "$file" ]; then
        echo -e "✅ $file: present"
    else
        echo -e "${RED}❌ $file: missing${NC}"
        ((validation_errors++))
    fi
done

# Check for optional but expected files
OPTIONAL_FILES=("README.md")
for file in "${OPTIONAL_FILES[@]}"; do
    if [ -f "$file" ]; then
        echo -e "✅ $file: present"
    else
        echo -e "${YELLOW}⚠️  $file: missing (optional)${NC}"
        ((validation_warnings++))
    fi
done

echo ""
echo -e "${YELLOW}📋 Package.json Validation:${NC}"
echo "----------------------------------------"

if [ -f "package.json" ]; then
    # Validate package.json structure
    if command -v jq &> /dev/null; then
        # Validate JSON syntax
        if jq empty package.json 2>/dev/null; then
            echo -e "✅ package.json: valid JSON"
        else
            echo -e "${RED}❌ package.json: invalid JSON syntax${NC}"
            ((validation_errors++))
        fi
        
        # Check required fields
        REQUIRED_FIELDS=("name" "version" "main" "types")
        for field in "${REQUIRED_FIELDS[@]}"; do
            value=$(jq -r ".$field // \"null\"" package.json 2>/dev/null)
            if [ "$value" != "null" ] && [ "$value" != "" ]; then
                echo -e "✅ $field: $value"
            else
                echo -e "${RED}❌ $field: missing or empty${NC}"
                ((validation_errors++))
            fi
        done
        
        # Check package name matches expected value from Stream A
        package_name=$(jq -r '.name // "null"' package.json 2>/dev/null)
        if [ "$package_name" = "dash" ]; then
            echo -e "✅ Package name matches Stream A configuration: $package_name"
        else
            echo -e "${RED}❌ Package name mismatch. Expected 'dash', got: $package_name${NC}"
            ((validation_errors++))
        fi
        
        # Validate entry points exist
        main_entry=$(jq -r '.main // "null"' package.json 2>/dev/null)
        if [ "$main_entry" != "null" ] && [ -f "$main_entry" ]; then
            echo -e "✅ Main entry point exists: $main_entry"
        elif [ "$main_entry" != "null" ]; then
            echo -e "${RED}❌ Main entry point missing: $main_entry${NC}"
            ((validation_errors++))
        fi
        
        types_entry=$(jq -r '.types // "null"' package.json 2>/dev/null)
        if [ "$types_entry" != "null" ] && [ -f "$types_entry" ]; then
            echo -e "✅ Types entry point exists: $types_entry"
        elif [ "$types_entry" != "null" ]; then
            echo -e "${RED}❌ Types entry point missing: $types_entry${NC}"
            ((validation_errors++))
        fi
        
        # Check optional but recommended fields
        RECOMMENDED_FIELDS=("description" "keywords" "author" "license" "repository")
        for field in "${RECOMMENDED_FIELDS[@]}"; do
            value=$(jq -r ".$field // \"null\"" package.json 2>/dev/null)
            if [ "$value" != "null" ] && [ "$value" != "" ]; then
                echo -e "✅ $field: present"
            else
                echo -e "${YELLOW}⚠️  $field: missing (recommended)${NC}"
                ((validation_warnings++))
            fi
        done
        
    else
        echo -e "${YELLOW}⚠️  jq not available, skipping detailed package.json validation${NC}"
        ((validation_warnings++))
    fi
else
    echo -e "${RED}❌ package.json: not found${NC}"
    ((validation_errors++))
fi

echo ""
echo -e "${YELLOW}🗂️  File Size Validation:${NC}"
echo "----------------------------------------"

# Check file sizes are reasonable
for file in *.wasm *.js *.d.ts; do
    if [ -f "$file" ]; then
        file_size=$(wc -c < "$file")
        file_size_kb=$((file_size / 1024))
        
        case "$file" in
            *.wasm)
                # WASM files should be substantial but not excessive
                if [ "$file_size" -lt 1024 ]; then
                    echo -e "${RED}❌ $file: suspiciously small (${file_size_kb}KB)${NC}"
                    ((validation_errors++))
                elif [ "$file_size" -gt $((20 * 1024 * 1024)) ]; then
                    echo -e "${RED}❌ $file: too large (${file_size_kb}KB > 20MB)${NC}"
                    ((validation_errors++))
                else
                    echo -e "✅ $file: reasonable size (${file_size_kb}KB)"
                fi
                ;;
            *.js)
                # JavaScript files should have reasonable size
                if [ "$file_size" -lt 100 ]; then
                    echo -e "${RED}❌ $file: suspiciously small (${file_size}B)${NC}"
                    ((validation_errors++))
                elif [ "$file_size" -gt $((1024 * 1024)) ]; then
                    echo -e "${YELLOW}⚠️  $file: large JS file (${file_size_kb}KB)${NC}"
                    ((validation_warnings++))
                else
                    echo -e "✅ $file: reasonable size (${file_size_kb}KB)"
                fi
                ;;
            *.d.ts)
                # TypeScript definition files
                if [ "$file_size" -lt 50 ]; then
                    echo -e "${RED}❌ $file: suspiciously small (${file_size}B)${NC}"
                    ((validation_errors++))
                else
                    echo -e "✅ $file: reasonable size (${file_size_kb}KB)"
                fi
                ;;
        esac
    fi
done

echo ""
echo -e "${YELLOW}🔗 JavaScript Module Validation:${NC}"
echo "----------------------------------------"

# Check JS file for basic module structure
if [ -f "wasm_sdk.js" ]; then
    if grep -q "export" wasm_sdk.js; then
        echo -e "✅ wasm_sdk.js: contains exports"
    else
        echo -e "${RED}❌ wasm_sdk.js: no exports found${NC}"
        ((validation_errors++))
    fi
    
    if grep -q "wasm" wasm_sdk.js; then
        echo -e "✅ wasm_sdk.js: references WASM"
    else
        echo -e "${YELLOW}⚠️  wasm_sdk.js: no WASM references found${NC}"
        ((validation_warnings++))
    fi
fi

# Check TypeScript definitions
if [ -f "wasm_sdk.d.ts" ]; then
    if grep -q "declare" wasm_sdk.d.ts || grep -q "export" wasm_sdk.d.ts; then
        echo -e "✅ wasm_sdk.d.ts: contains type declarations"
    else
        echo -e "${RED}❌ wasm_sdk.d.ts: no type declarations found${NC}"
        ((validation_errors++))
    fi
fi

echo ""
echo -e "${YELLOW}📊 Bundle Size Compliance:${NC}"
echo "----------------------------------------"

# Check against bundlesize.json if it exists
if [ -f "$SCRIPT_DIR/bundlesize.json" ] && command -v bundlesize &> /dev/null; then
    cd "$SCRIPT_DIR"
    if bundlesize 2>/dev/null; then
        echo -e "✅ Bundle sizes within defined limits"
    else
        echo -e "${YELLOW}⚠️  Bundle sizes exceed defined limits${NC}"
        ((validation_warnings++))
    fi
    cd "$PKG_DIR"
else
    # Manual size checks
    total_size=0
    for file in *.wasm *.js *.d.ts; do
        if [ -f "$file" ]; then
            file_size=$(wc -c < "$file")
            total_size=$((total_size + file_size))
        fi
    done
    
    total_mb=$((total_size / 1024 / 1024))
    if [ $total_mb -le 15 ]; then
        echo -e "✅ Total bundle size acceptable: ${total_mb}MB"
    else
        echo -e "${YELLOW}⚠️  Large bundle size: ${total_mb}MB${NC}"
        ((validation_warnings++))
    fi
fi

echo ""
echo -e "${BLUE}📝 Validation Summary:${NC}"
echo "========================================"

if [ "$validation_errors" -eq 0 ] && [ "$validation_warnings" -eq 0 ]; then
    echo -e "${GREEN}🎉 Perfect! No issues found.${NC}"
    echo -e "${GREEN}✅ Package is ready for npm publishing.${NC}"
    exit 0
elif [ "$validation_errors" -eq 0 ]; then
    echo -e "${YELLOW}✅ Validation passed with ${validation_warnings} warning(s).${NC}"
    echo -e "${YELLOW}Package is ready for npm publishing, but consider addressing warnings.${NC}"
    exit 0
else
    echo -e "${RED}❌ Validation failed with ${validation_errors} error(s) and ${validation_warnings} warning(s).${NC}"
    echo -e "${RED}Fix errors before attempting to publish.${NC}"
    exit 1
fi