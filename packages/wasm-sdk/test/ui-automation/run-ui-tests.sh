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
WASM_SDK_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
UI_TEST_DIR="$(dirname "${BASH_SOURCE[0]}")"
SERVER_PORT=8888
SERVER_PID=""

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

# Function to cleanup on exit
cleanup() {
    if [ ! -z "$SERVER_PID" ]; then
        print_status "Stopping web server (PID: $SERVER_PID)..."
        kill $SERVER_PID 2>/dev/null || true
        wait $SERVER_PID 2>/dev/null || true
    fi
}

# Set trap to cleanup on exit
trap cleanup EXIT

# Function to check prerequisites
check_prerequisites() {
    print_status "Checking prerequisites..."
    
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
    
    # Check Python
    if ! command -v python3 &> /dev/null; then
        print_error "Python 3 is not installed. Please install Python 3 and try again."
        exit 1
    fi
    
    # Check if WASM SDK is built
    if [ ! -f "$WASM_SDK_DIR/pkg/wasm_sdk.js" ]; then
        print_warning "WASM SDK not found. Building..."
        cd "$WASM_SDK_DIR"
        ./build.sh
    fi
    
    print_status "Prerequisites check passed ✓"
}

# Function to install dependencies
install_dependencies() {
    print_status "Installing test dependencies..."
    
    cd "$UI_TEST_DIR"
    
    if [ ! -d "node_modules" ]; then
        npm install
    fi
    
    # Check if browsers are installed
    if ! npx playwright --version &> /dev/null; then
        print_error "Playwright not found. Installing..."
        npm install
    fi
    
    # Install browsers if needed
    if [ ! -d "$HOME/.cache/ms-playwright/chromium-"* ]; then
        print_status "Installing Playwright browsers..."
        npx playwright install chromium
    fi
    
    print_status "Dependencies installed ✓"
}

# Function to start web server
start_web_server() {
    print_status "Starting web server on port $SERVER_PORT..."
    
    # Check if port is already in use
    if lsof -Pi :$SERVER_PORT -sTCP:LISTEN -t &> /dev/null; then
        print_warning "Port $SERVER_PORT is already in use. Assuming server is running."
        return 0
    fi
    
    cd "$WASM_SDK_DIR"
    python3 -m http.server $SERVER_PORT &> /dev/null &
    SERVER_PID=$!
    
    # Wait for server to start
    sleep 2
    
    # Verify server is running
    if ! curl -s "http://localhost:$SERVER_PORT" &> /dev/null; then
        print_error "Failed to start web server"
        exit 1
    fi
    
    print_status "Web server started (PID: $SERVER_PID) ✓"
}

# Function to run tests
run_tests() {
    print_status "Running UI automation tests..."
    
    cd "$UI_TEST_DIR"
    
    # Determine test type from arguments
    case "${1:-all}" in
        "smoke")
            npm run test:smoke
            ;;
        "queries")
            npm run test:queries
            ;;
        "parameterized")
            npm run test:parameterized
            ;;
        "headed")
            npm run test:headed
            ;;
        "debug")
            npm run test:debug
            ;;
        "ui")
            npm run test:ui
            ;;
        "all")
            npm run test:all
            ;;
        *)
            # Pass through any other arguments to playwright
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
    echo "Examples:"
    echo "  $0                    # Run all tests"
    echo "  $0 smoke              # Run smoke tests only"
    echo "  $0 headed             # Run tests with visible browser"
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
    start_web_server
    
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