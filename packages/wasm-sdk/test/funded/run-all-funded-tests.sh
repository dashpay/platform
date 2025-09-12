#!/bin/bash

# Run All Real Funded Tests
# Executes all funded tests that consume actual testnet credits

set -e

echo "💰 Running All Real Funded Tests"
echo "================================"
echo "⚠️ WARNING: These tests will consume REAL testnet credits!"
echo ""

# Safety confirmation
if [ "$1" != "--confirm-fund-usage" ]; then
    echo "❌ Safety confirmation required"
    echo "Usage: $0 --confirm-fund-usage"
    echo ""
    echo "This will run tests that consume actual testnet credits:"
    echo "  • Document operations: ~2-5M credits per operation"
    echo "  • DPNS registration: ~5M credits per domain"  
    echo "  • Contract creation: ~25-30M credits per contract"
    echo ""
    echo "Add --confirm-fund-usage to proceed"
    exit 1
fi

# Load environment
if [ ! -f ".env" ]; then
    echo "❌ Environment file not found: .env"
    echo "Copy .env.example to .env and configure it"
    exit 1
fi

set -a
source .env
set +a

# Validate configuration
if [ "$ENABLE_FUNDED_TESTS" != "true" ]; then
    echo "❌ Funded tests not enabled"
    echo "Set ENABLE_FUNDED_TESTS=true in .env"
    exit 1
fi

if [ "$NETWORK" != "testnet" ]; then
    echo "❌ Network must be testnet for safety"
    echo "Current NETWORK: $NETWORK"
    exit 1
fi

if [ -z "$TEST_IDENTITY_1_ID" ] || [ -z "$TEST_IDENTITY_1_PRIVATE_KEY" ]; then
    echo "❌ Test identity not configured"
    echo "Configure TEST_IDENTITY_1_ID and TEST_IDENTITY_1_PRIVATE_KEY in .env"
    exit 1
fi

echo "✅ Configuration validated"
echo "🎯 Test identity: ${TEST_IDENTITY_1_ID:0:20}..."
echo "🌐 Network: $NETWORK"
echo ""

# Create results directory
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
RESULTS_DIR="test-results-${TIMESTAMP}"
mkdir -p "$RESULTS_DIR"

echo "📁 Results will be saved to: $RESULTS_DIR"
echo ""

# Run tests
echo "🚀 Executing funded tests..."
echo ""

failed_tests=()

# Test 1: Document operations
echo "📄 Running document operations test..."
if node real-document-operations.test.mjs > "$RESULTS_DIR/document-ops.log" 2>&1; then
    echo "✅ Document operations test passed"
else
    echo "❌ Document operations test failed"
    failed_tests+=("document-operations")
fi

# Test 2: DPNS operations  
echo "🌐 Running DPNS operations test..."
if node real-dpns-operations.test.mjs > "$RESULTS_DIR/dpns-ops.log" 2>&1; then
    echo "✅ DPNS operations test passed"
else
    echo "❌ DPNS operations test failed"
    failed_tests+=("dpns-operations")
fi

# Test 3: Contract operations
echo "📋 Running contract operations test..."
if node real-contract-operations.test.mjs > "$RESULTS_DIR/contract-ops.log" 2>&1; then
    echo "✅ Contract operations test passed"
else
    echo "❌ Contract operations test failed"  
    failed_tests+=("contract-operations")
fi

echo ""
echo "📊 Test Summary"
echo "==============="

if [ ${#failed_tests[@]} -eq 0 ]; then
    echo "🎉 All funded tests passed!"
    echo ""
    echo "📊 Check detailed results:"
    echo "  - $RESULTS_DIR/document-ops.log"
    echo "  - $RESULTS_DIR/dpns-ops.log"
    echo "  - $RESULTS_DIR/contract-ops.log"
    echo ""
    echo "💰 Credit usage details are in the individual logs"
    
    # Extract credit usage from logs
    echo "💳 Credit Usage Summary:"
    
    for log_file in "$RESULTS_DIR"/*.log; do
        if [ -f "$log_file" ]; then
            test_name=$(basename "$log_file" .log)
            credits_used=$(grep "Total Credits Used:" "$log_file" | tail -1 | grep -o '[0-9]\+' || echo "0")
            if [ "$credits_used" -gt 0 ]; then
                echo "  $test_name: $credits_used credits"
            else
                echo "  $test_name: No credits consumed (validation errors or free operations)"
            fi
        fi
    done
    
    exit 0
else
    echo "❌ Failed tests: ${failed_tests[*]}"
    echo ""
    echo "📋 Check logs for details:"
    for test in "${failed_tests[@]}"; do
        echo "  - $RESULTS_DIR/$test.log"
    done
    exit 1
fi