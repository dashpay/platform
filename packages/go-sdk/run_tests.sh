#!/bin/bash

# Script to build dependencies and run Go SDK tests

set -e

echo "Setting up Go SDK tests..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check prerequisites
echo -e "${YELLOW}Checking prerequisites...${NC}"

# Check for Go
if ! command -v go &> /dev/null; then
    echo -e "${RED}Error: Go is not installed${NC}"
    echo "Please install Go 1.19 or higher from https://golang.org/dl/"
    exit 1
fi

GO_VERSION=$(go version | awk '{print $3}' | sed 's/go//')
echo -e "${GREEN}✓ Go ${GO_VERSION} found${NC}"

# Check for Rust
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: Rust/Cargo is not installed${NC}"
    echo "Please install Rust from https://rustup.rs/"
    exit 1
fi

RUST_VERSION=$(rustc --version | awk '{print $2}')
echo -e "${GREEN}✓ Rust ${RUST_VERSION} found${NC}"

# Build FFI library
echo -e "\n${YELLOW}Building rs-sdk-ffi library...${NC}"
cd ../rs-sdk-ffi
cargo build --release

# Check if build succeeded
if [ -f "../../target/release/libdash_sdk_ffi.so" ] || [ -f "../../target/release/libdash_sdk_ffi.dylib" ]; then
    echo -e "${GREEN}✓ FFI library built successfully${NC}"
else
    echo -e "${RED}Error: FFI library build failed${NC}"
    exit 1
fi

# Return to go-sdk directory
cd ../go-sdk

# Set library path
export CGO_LDFLAGS="-L../../target/release"
export LD_LIBRARY_PATH="../../target/release:$LD_LIBRARY_PATH"
export DYLD_LIBRARY_PATH="../../target/release:$DYLD_LIBRARY_PATH"

# Download dependencies
echo -e "\n${YELLOW}Downloading Go dependencies...${NC}"
go mod download

# Run tests
echo -e "\n${YELLOW}Running tests...${NC}"

# Unit tests
echo -e "\n${GREEN}Running unit tests...${NC}"
go test -v -count=1 ./...

# Run with race detector
echo -e "\n${GREEN}Running tests with race detector...${NC}"
go test -race -count=1 ./...

# Generate coverage report
echo -e "\n${GREEN}Generating coverage report...${NC}"
go test -coverprofile=coverage.out ./...
go tool cover -html=coverage.out -o coverage.html

echo -e "\n${GREEN}All tests completed!${NC}"
echo -e "Coverage report available at: coverage.html"

# Summary
echo -e "\n${YELLOW}Test Summary:${NC}"
go test -json ./... 2>/dev/null | jq -r 'select(.Action=="pass" or .Action=="fail") | "\(.Action): \(.Test)"' | sort | uniq -c