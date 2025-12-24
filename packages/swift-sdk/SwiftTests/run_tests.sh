#!/bin/bash

# Swift SDK Test Runner Script
# This script runs the Swift SDK tests using Swift Package Manager

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

echo "🧪 Running Swift SDK Tests..."
echo "=========================="

# Change to the test directory
cd "$SCRIPT_DIR"

# Clean build artifacts
echo "🧹 Cleaning build artifacts..."
swift package clean

# Build the test package
echo "🔨 Building test package..."
swift build

# Run tests with verbose output
echo "🏃 Running tests..."
swift test --verbose

# Check test results
if [ $? -eq 0 ]; then
    echo ""
    echo "✅ All tests passed!"
    echo ""
    
    # Optionally run with coverage
    if [[ "$1" == "--coverage" ]]; then
        echo "📊 Generating code coverage..."
        swift test --enable-code-coverage
        
        # Find the coverage data
        COV_BUILD_DIR=$(swift build --show-bin-path)
        COV_DATA="${COV_BUILD_DIR}/codecov/default.profdata"
        
        if [ -f "$COV_DATA" ]; then
            echo "Coverage data generated at: $COV_DATA"
        fi
    fi
else
    echo ""
    echo "❌ Tests failed!"
    exit 1
fi

# Optional: Run specific test suites
if [[ "$1" == "--filter" && -n "$2" ]]; then
    echo ""
    echo "🔍 Running filtered tests: $2"
    swift test --filter "$2"
fi

# Show test summary
echo ""
echo "📋 Test Summary:"
echo "==============="
swift test list | grep -E "test[A-Z]" | wc -l | xargs echo "Total test methods:"

# Group by test class
echo ""
echo "Tests by class:"
swift test list | grep -E "^[A-Za-z]+Tests" | sort | uniq -c

echo ""
echo "🎉 Test run complete!"