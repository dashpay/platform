#!/bin/bash
# Test runner script for WASM SDK

set -e

echo "🧪 Running WASM SDK Tests"
echo "========================"

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "❌ wasm-pack is not installed. Please install it with:"
    echo "   cargo install wasm-pack"
    exit 1
fi

# Build the WASM package
echo "📦 Building WASM package..."
cargo build --target wasm32-unknown-unknown

# Run unit tests in Node.js environment
echo "🏃 Running unit tests in Node.js..."
wasm-pack test --node

# Run browser tests (headless Chrome)
echo "🌐 Running browser tests..."
wasm-pack test --headless --chrome

# Run browser tests with Firefox (optional)
if command -v firefox &> /dev/null; then
    echo "🦊 Running Firefox tests..."
    wasm-pack test --headless --firefox
fi

# Generate test coverage report (if grcov is installed)
if command -v grcov &> /dev/null; then
    echo "📊 Generating coverage report..."
    export CARGO_INCREMENTAL=0
    export RUSTFLAGS="-Cinstrument-coverage"
    export LLVM_PROFILE_FILE="wasm-sdk-%p-%m.profraw"
    
    cargo test --target wasm32-unknown-unknown
    
    grcov . --binary-path ./target/wasm32-unknown-unknown/debug/deps \
        -s . -t html --branch --ignore-not-existing --ignore '../*' \
        -o target/coverage/
    
    echo "📊 Coverage report generated at: target/coverage/index.html"
fi

echo "✅ All tests completed successfully!"