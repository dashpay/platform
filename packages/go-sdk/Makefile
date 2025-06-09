.PHONY: all build test test-integration test-coverage clean install deps ffi

# Default target
all: deps ffi build test

# Install dependencies
deps:
	go mod download
	go mod tidy

# Build FFI library
ffi:
	@echo "Building rs-sdk-ffi..."
	cd ../rs-sdk-ffi && cargo build --release

# Build Go SDK
build: ffi
	go build -v ./...

# Run tests
test:
	go test -v -race -short ./...

# Run integration tests
test-integration:
	go test -v -race -tags=integration ./...

# Run tests with coverage
test-coverage:
	go test -v -race -coverprofile=coverage.out ./...
	go tool cover -html=coverage.out -o coverage.html
	@echo "Coverage report: coverage.html"

# Clean build artifacts
clean:
	go clean -cache
	rm -f coverage.out coverage.html
	cd ../rs-sdk-ffi && cargo clean

# Install the SDK
install: ffi
	go install ./...

# Format code
fmt:
	go fmt ./...

# Run linter
lint:
	golangci-lint run

# Generate documentation
doc:
	godoc -http=:6060

# Run example
example: build
	go run example/main.go