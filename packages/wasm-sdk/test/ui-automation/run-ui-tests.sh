#!/bin/bash

# WASM SDK UI Automation Test Runner
# This script sets up and runs the UI automation tests

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)" 
WASM_SDK_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
UI_TEST_DIR="$SCRIPT_DIR"

# Debug mode flag
DEBUG=${DEBUG:-false}

# Function to print colored output
print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_debug() {
    if [ "$DEBUG" = "true" ]; then
        echo -e "${YELLOW}[DEBUG]${NC} $1"
    fi
}


# Function to validate paths
validate_paths() {
    print_debug "Validating directory paths..."
    print_debug "SCRIPT_DIR: $SCRIPT_DIR"
    print_debug "WASM_SDK_DIR: $WASM_SDK_DIR"
    print_debug "UI_TEST_DIR: $UI_TEST_DIR"
    
    # Check if UI test directory exists and contains expected files
    if [ ! -d "$UI_TEST_DIR" ]; then
        print_error "UI test directory not found: $UI_TEST_DIR"
        exit 1
    fi
    
    if [ ! -f "$UI_TEST_DIR/package.json" ]; then
        print_error "package.json not found in UI test directory: $UI_TEST_DIR"
        exit 1
    fi
    
    if [ ! -f "$UI_TEST_DIR/playwright.config.js" ]; then
        print_error "playwright.config.js not found in UI test directory: $UI_TEST_DIR"
        exit 1
    fi
    
    # Check if WASM SDK directory exists
    if [ ! -d "$WASM_SDK_DIR" ]; then
        print_error "WASM SDK directory not found: $WASM_SDK_DIR"
        exit 1
    fi
    
    if [ ! -f "$WASM_SDK_DIR/index.html" ]; then
        print_error "index.html not found in WASM SDK directory: $WASM_SDK_DIR"
        exit 1
    fi
    
    print_debug "Path validation passed ✓"
}

# Function to check prerequisites
check_prerequisites() {
    print_status "Checking prerequisites..."
    
    # Validate paths first
    validate_paths
    
    # Check Node.js
    if ! command -v node &> /dev/null; then
        print_error "Node.js is not installed. Please install Node.js 18+ and try again."
        exit 1
    fi
    
    NODE_VERSION=$(node --version | sed 's/v//')
    NODE_MAJOR=$(echo $NODE_VERSION | cut -d. -f1)
    if [ "$NODE_MAJOR" -lt 18 ]; then
        print_error "Node.js version $NODE_VERSION is too old. Please install Node.js 18+ and try again."
        exit 1
    fi
    print_debug "Node.js version: $NODE_VERSION ✓"
    
    # Check Python
    if ! command -v python3 &> /dev/null; then
        print_error "Python 3 is not installed. Please install Python 3 and try again."
        exit 1
    fi
    PYTHON_VERSION=$(python3 --version 2>&1 | cut -d' ' -f2)
    print_debug "Python version: $PYTHON_VERSION ✓"
    
    # Check if WASM SDK is built
    if [ ! -f "$WASM_SDK_DIR/pkg/wasm_sdk.js" ]; then
        print_warning "WASM SDK not found. Building..."
        cd "$WASM_SDK_DIR"
        if ! ./build.sh; then
            print_error "Failed to build WASM SDK"
            exit 1
        fi
    fi
    print_debug "WASM SDK found ✓"
    
    print_status "Prerequisites check passed ✓"
}

# Function to install dependencies
install_dependencies() {
    print_status "Installing test dependencies..."
    
    cd "$UI_TEST_DIR" || {
        print_error "Failed to change to UI test directory: $UI_TEST_DIR"
        exit 1
    }
    print_debug "Changed to directory: $(pwd)"
    
    if [ ! -d "node_modules" ]; then
        print_status "Installing npm dependencies..."
        if ! npm install; then
            print_error "Failed to install npm dependencies"
            exit 1
        fi
    fi
    
    # Check if browsers are installed
    if ! npx playwright --version &> /dev/null; then
        print_error "Playwright not found. Installing..."
        if ! npm install; then
            print_error "Failed to install Playwright"
            exit 1
        fi
    fi
    
    # Install browsers if needed
    if ! find "$HOME/.cache/ms-playwright" -maxdepth 1 -name "chromium-*" -type d -print -quit 2>/dev/null | grep -q .; then
        print_status "Installing Playwright browsers..."
        if ! npx playwright install chromium; then
            print_error "Failed to install Playwright browsers"
            exit 1
        fi
    fi
    
    print_status "Dependencies installed ✓"
}


# Function to run tests
run_tests() {
    print_status "Running UI automation tests..."
    
    cd "$UI_TEST_DIR" || {
        print_error "Failed to change to UI test directory: $UI_TEST_DIR"
        exit 1
    }
    print_debug "Running tests from directory: $(pwd)"
    
    # Verify npm scripts exist
    if [ ! -f "package.json" ]; then
        print_error "package.json not found in test directory"
        exit 1
    fi
    
    # Show available npm scripts for debugging
    print_debug "Available npm scripts:"
    if [ "$DEBUG" = "true" ]; then
        npm run 2>/dev/null | grep -E "^\s*(test:|build:|start)" || true
    fi
    
    # Determine test type from arguments
    case "${1:-all}" in
        "smoke")
            print_status "Running smoke tests..."
            npm run test:smoke
            ;;
        "queries")
            print_status "Running query execution tests..."
            npm run test:queries
            ;;
        "parameterized")
            print_status "Running parameterized tests..."
            npm run test:parameterized
            ;;
        "headed")
            print_status "Running tests in headed mode..."
            npm run test:headed
            ;;
        "debug")
            print_status "Running tests in debug mode..."
            npm run test:debug
            ;;
        "ui")
            print_status "Running tests in UI mode..."
            npm run test:ui
            ;;
        "all")
            print_status "Running all tests..."
            npm run test:all
            ;;
        *)
            # Pass through any other arguments to playwright
            print_status "Running custom playwright command: $*"
            npx playwright test "$@"
            ;;
    esac
}

# Function to show results
show_results() {
    print_status "Test execution completed!"
    
    if [ -d "$UI_TEST_DIR/playwright-report" ]; then
        print_status "HTML report available at: $UI_TEST_DIR/playwright-report/index.html"
        print_status "To view report: npm run test:report"
    fi
    
    if [ -f "$UI_TEST_DIR/test-results.json" ]; then
        print_status "JSON results available at: $UI_TEST_DIR/test-results.json"
    fi
}

# Function to print usage
print_usage() {
    echo "Usage: $0 [test_type]"
    echo ""
    echo "Test types:"
    echo "  smoke          - Run basic smoke tests"
    echo "  queries        - Run query execution tests"
    echo "  parameterized  - Run parameterized tests"
    echo "  headed         - Run tests in headed mode (visible browser)"
    echo "  debug          - Run tests in debug mode"
    echo "  ui             - Run tests in UI mode (interactive)"
    echo "  all            - Run all tests (default)"
    echo ""
    echo "Environment variables:"
    echo "  DEBUG=true     - Enable debug output"
    echo ""
    echo "Examples:"
    echo "  $0                    # Run all tests"
    echo "  $0 smoke              # Run smoke tests only"
    echo "  $0 headed             # Run tests with visible browser"
    echo "  DEBUG=true $0 smoke   # Run smoke tests with debug output"
    echo "  $0 --grep=\"Identity\"  # Run tests matching pattern"
}

# Main execution
main() {
    # Handle help flag
    if [[ "$1" == "-h" || "$1" == "--help" ]]; then
        print_usage
        exit 0
    fi
    
    print_status "Starting WASM SDK UI Automation Tests..."
    print_status "Working directory: $WASM_SDK_DIR"
    print_status "Test directory: $UI_TEST_DIR"
    
    check_prerequisites
    install_dependencies
    
    # Run tests and capture exit code
    if run_tests "$@"; then
        print_status "All tests completed successfully! ✅"
        show_results
        exit 0
    else
        print_error "Some tests failed! ❌"
        show_results
        exit 1
    fi
}

# Run main function with all arguments
main "$@"
