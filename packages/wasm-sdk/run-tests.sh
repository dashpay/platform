#!/bin/bash

# Run WASM SDK tests
echo "Running WASM SDK tests..."

# Build the project first
echo "Building WASM SDK..."
wasm-pack build --target web --out-dir pkg

# Run unit tests in Chrome headless
echo "Running unit tests..."
wasm-pack test --chrome --headless

# Run tests with coverage if available
# wasm-pack test --chrome --headless --coverage

# Run specific test suites if needed
# echo "Running BIP39 tests..."
# wasm-pack test --chrome --headless -- --test bip39_tests

# echo "Running monitoring tests..."
# wasm-pack test --chrome --headless -- --test monitoring_tests

# echo "Running DAPI client tests..."
# wasm-pack test --chrome --headless -- --test dapi_client_tests

# echo "Running prefunded balance tests..."
# wasm-pack test --chrome --headless -- --test prefunded_balance_tests

# echo "Running identity info tests..."
# wasm-pack test --chrome --headless -- --test identity_info_tests

# echo "Running contract history tests..."
# wasm-pack test --chrome --headless -- --test contract_history_tests

echo "Tests completed!"