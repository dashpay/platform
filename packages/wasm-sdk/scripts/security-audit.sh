#!/bin/bash

# Security audit script for WASM SDK

set -e

echo "🔒 Running Security Audit for WASM SDK"
echo "====================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Counters
WARNINGS=0
ERRORS=0

# Function to check command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to print result
print_result() {
    if [ $1 -eq 0 ]; then
        echo -e "${GREEN}✓${NC} $2"
    else
        echo -e "${RED}✗${NC} $2"
        ERRORS=$((ERRORS + 1))
    fi
}

# Function to print warning
print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
    WARNINGS=$((WARNINGS + 1))
}

echo -e "\n📋 Checking dependencies..."

# Check for required tools
if command_exists cargo-audit; then
    echo -e "${GREEN}✓${NC} cargo-audit installed"
else
    echo -e "${RED}✗${NC} cargo-audit not installed. Installing..."
    cargo install cargo-audit
fi

if command_exists cargo-deny; then
    echo -e "${GREEN}✓${NC} cargo-deny installed"
else
    print_warning "cargo-deny not installed. Install with: cargo install cargo-deny"
fi

echo -e "\n🔍 Running security checks..."

# 1. Cargo audit
echo -e "\n1. Checking for known vulnerabilities..."
if cargo audit; then
    print_result 0 "No known vulnerabilities found"
else
    print_result 1 "Vulnerabilities found! Run 'cargo audit' for details"
fi

# 2. Check for unsafe code
echo -e "\n2. Checking for unsafe code blocks..."
UNSAFE_COUNT=$(grep -r "unsafe" src/ --include="*.rs" | wc -l)
if [ $UNSAFE_COUNT -eq 0 ]; then
    print_result 0 "No unsafe code blocks found"
else
    print_warning "Found $UNSAFE_COUNT unsafe code blocks"
    echo "   Review each unsafe block:"
    grep -r "unsafe" src/ --include="*.rs" | head -5
fi

# 3. Check for hardcoded secrets
echo -e "\n3. Checking for hardcoded secrets..."
# Exclude common false positives like data tokens, cache tokens, etc.
SECRETS=$(grep -r -E "(api_key|apikey|password|secret|private_key|privatekey|auth_token)" src/ --include="*.rs" | grep -v -E "(test|example|mock|cache|Cache)" | grep -E "=\s*[\"\']" | wc -l)
if [ $SECRETS -eq 0 ]; then
    print_result 0 "No hardcoded secrets found"
else
    print_result 1 "Potential secrets found! Review these lines:"
    grep -r -E "(api_key|apikey|password|secret|private_key|privatekey|auth_token)" src/ --include="*.rs" | grep -v -E "(test|example|mock|cache|Cache)" | grep -E "=\s*[\"\']" | head -5
fi

# 4. Check dependencies
echo -e "\n4. Checking dependency licenses..."
if [ -f "Cargo.deny.toml" ]; then
    if command_exists cargo-deny; then
        cargo deny check licenses || print_warning "License check failed"
    fi
else
    print_warning "No Cargo.deny.toml found for license checking"
fi

# 5. Check for outdated dependencies
echo -e "\n5. Checking for outdated dependencies..."
OUTDATED=$(cargo outdated --exit-code 1 2>/dev/null | wc -l)
if [ $OUTDATED -eq 0 ]; then
    print_result 0 "All dependencies up to date"
else
    print_warning "$OUTDATED dependencies are outdated. Run 'cargo outdated' for details"
fi

# 6. Check WASM optimization
echo -e "\n6. Checking WASM build configuration..."
if grep -q 'lto = "fat"' Cargo.toml && grep -q 'opt-level = "z"' Cargo.toml; then
    print_result 0 "WASM optimization settings correct"
else
    print_warning "WASM optimization not fully configured in Cargo.toml"
fi

# 7. Check for debug information
echo -e "\n7. Checking for debug information in release..."
if grep -q 'debug = false' Cargo.toml && grep -q 'strip = "symbols"' Cargo.toml; then
    print_result 0 "Debug information properly stripped in release"
else
    print_warning "Debug information may be included in release builds"
fi

# 8. Check error handling
echo -e "\n8. Checking error handling..."
UNWRAPS=$(grep -r "unwrap()" src/ --include="*.rs" | grep -v -E "(test|#\[cfg\(test\)\])" | wc -l)
EXPECTS=$(grep -r "expect(" src/ --include="*.rs" | grep -v -E "(test|#\[cfg\(test\)\])" | wc -l)
if [ $((UNWRAPS + EXPECTS)) -eq 0 ]; then
    print_result 0 "No unwrap() or expect() in production code"
else
    print_warning "Found $UNWRAPS unwrap() and $EXPECTS expect() calls in production code"
    echo "   These could cause panics. Consider using proper error handling."
fi

# 9. Check for TODO/FIXME comments
echo -e "\n9. Checking for TODO/FIXME comments..."
TODOS=$(grep -r -E "(TODO|FIXME|XXX|HACK)" src/ --include="*.rs" | wc -l)
if [ $TODOS -eq 0 ]; then
    print_result 0 "No TODO/FIXME comments found"
else
    print_warning "Found $TODOS TODO/FIXME comments that may indicate security issues"
fi

# 10. Check cryptographic implementations
echo -e "\n10. Checking cryptographic implementations..."
CUSTOM_CRYPTO=$(grep -r -E "(impl.*Hash|impl.*Cipher|impl.*Encrypt|impl.*Decrypt)" src/ --include="*.rs" | wc -l)
if [ $CUSTOM_CRYPTO -eq 0 ]; then
    print_result 0 "No custom cryptographic implementations found"
else
    print_warning "Found potential custom crypto implementations. Ensure using audited libraries"
fi

# Generate security report
echo -e "\n📊 Security Audit Summary"
echo "========================"
echo -e "Errors:   ${RED}$ERRORS${NC}"
echo -e "Warnings: ${YELLOW}$WARNINGS${NC}"

if [ $ERRORS -eq 0 ] && [ $WARNINGS -eq 0 ]; then
    echo -e "\n${GREEN}✅ Security audit passed with no issues!${NC}"
    exit 0
elif [ $ERRORS -eq 0 ]; then
    echo -e "\n${YELLOW}⚠️  Security audit passed with warnings${NC}"
    exit 0
else
    echo -e "\n${RED}❌ Security audit failed!${NC}"
    echo "Please fix the errors before proceeding."
    exit 1
fi