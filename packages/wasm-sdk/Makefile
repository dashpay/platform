# Makefile for WASM SDK development

.PHONY: help install build build-dev build-release test test-unit test-wasm lint fmt clean docs serve

# Default target
help:
	@echo "WASM SDK Development Commands:"
	@echo "  make install      - Install dependencies"
	@echo "  make build        - Build development version"
	@echo "  make build-release - Build optimized release version"
	@echo "  make test         - Run all tests"
	@echo "  make test-unit    - Run unit tests only"
	@echo "  make test-wasm    - Run WASM tests in browser"
	@echo "  make lint         - Run linting checks"
	@echo "  make fmt          - Format code"
	@echo "  make clean        - Clean build artifacts"
	@echo "  make docs         - Build documentation"
	@echo "  make serve        - Serve example app"

# Install dependencies
install:
	@echo "Installing dependencies..."
	@command -v rustup >/dev/null 2>&1 || curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
	@rustup target add wasm32-unknown-unknown
	@command -v wasm-pack >/dev/null 2>&1 || curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
	@rustup component add rustfmt clippy
	@npm install

# Build development version
build:
	@echo "Building development version..."
	@wasm-pack build --target web --out-dir pkg --dev

# Build release version
build-release:
	@echo "Building release version..."
	@wasm-pack build --target web --out-dir pkg --release
	@echo "Optimizing WASM..."
	@if command -v wasm-opt >/dev/null 2>&1; then \
		wasm-opt -Oz -o pkg/wasm_sdk_bg_optimized.wasm pkg/wasm_sdk_bg.wasm; \
		echo "Optimization complete. Size comparison:"; \
		ls -lh pkg/*.wasm; \
	else \
		echo "wasm-opt not found. Install binaryen for optimization."; \
	fi

# Run all tests
test: test-unit test-wasm

# Run unit tests
test-unit:
	@echo "Running unit tests..."
	@cargo test --workspace --lib
	@cargo test --workspace --doc

# Run WASM tests
test-wasm:
	@echo "Running WASM tests..."
	@wasm-pack test --chrome --headless

# Run specific test
test-specific:
	@echo "Running test: $(TEST)"
	@wasm-pack test --chrome --headless -- --test $(TEST)

# Lint code
lint:
	@echo "Running linters..."
	@cargo fmt --all -- --check
	@cargo clippy --workspace --all-features -- -D warnings

# Format code
fmt:
	@echo "Formatting code..."
	@cargo fmt --all

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	@cargo clean
	@rm -rf pkg/
	@rm -rf node_modules/
	@rm -f Cargo.lock

# Build documentation
docs:
	@echo "Building documentation..."
	@cargo doc --workspace --no-deps --all-features --open

# Serve example app
serve: build
	@echo "Starting development server..."
	@python3 -m http.server 8080 --directory .

# Check code coverage
coverage:
	@echo "Generating code coverage..."
	@cargo tarpaulin --workspace --out Html --all-features

# Security audit
audit:
	@echo "Running security audit..."
	@cargo audit

# Benchmark
bench:
	@echo "Running benchmarks..."
	@cargo bench --workspace

# Check bundle size
size: build-release
	@echo "Bundle size analysis:"
	@echo "===================="
	@ls -lh pkg/*.wasm
	@echo ""
	@echo "JavaScript size:"
	@ls -lh pkg/*.js
	@echo ""
	@echo "Total package size:"
	@du -sh pkg/

# Create release
release:
	@echo "Creating release..."
	@cargo release --workspace

# Quick development cycle
dev: fmt build test-unit
	@echo "Development build complete!"

# Pre-commit checks
pre-commit: fmt lint test-unit
	@echo "Pre-commit checks passed!"

# Install git hooks
install-hooks:
	@echo "Installing git hooks..."
	@echo "#!/bin/sh\nmake pre-commit" > .git/hooks/pre-commit
	@chmod +x .git/hooks/pre-commit
	@echo "Git hooks installed!"