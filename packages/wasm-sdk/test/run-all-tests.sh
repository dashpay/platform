#!/bin/bash

# Comprehensive Test Runner for WASM SDK Samples and Examples
# Runs all test suites with proper reporting and error handling

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test configuration
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
LOG_DIR="test-results-${TIMESTAMP}"
PARALLEL_JOBS=4
TIMEOUT=300

echo -e "${BLUE}🧪 WASM SDK Comprehensive Test Suite${NC}"
echo -e "${BLUE}======================================${NC}"
echo ""

# Create results directory
mkdir -p "$LOG_DIR"
echo "Test results will be saved to: $LOG_DIR"

# Function to check prerequisites
check_prerequisites() {
    echo -e "${YELLOW}🔍 Checking prerequisites...${NC}"
    
    # Check Node.js version
    if ! command -v node &> /dev/null; then
        echo -e "${RED}❌ Node.js is required but not installed${NC}"
        exit 1
    fi
    
    local node_version=$(node --version | cut -d'v' -f2 | cut -d'.' -f1)
    if [ "$node_version" -lt 18 ]; then
        echo -e "${RED}❌ Node.js 18+ is required (found: $(node --version))${NC}"
        exit 1
    fi
    
    # Check if WASM SDK is built
    if [ ! -f "../pkg/wasm_sdk.js" ]; then
        echo -e "${RED}❌ WASM SDK not built. Please run ./build.sh first${NC}"
        exit 1
    fi
    
    # Check if web server dependencies are available
    if ! command -v python3 &> /dev/null; then
        echo -e "${YELLOW}⚠️  Python3 not found, web app tests may fail${NC}"
    fi
    
    echo -e "${GREEN}✅ Prerequisites check passed${NC}"
}

# Function to start web server
start_web_server() {
    echo -e "${YELLOW}🌐 Starting web server for browser tests...${NC}"
    
    cd ..
    python3 -m http.server 8888 > "$LOG_DIR/web-server.log" 2>&1 &
    WEB_SERVER_PID=$!
    cd test
    
    # Wait for server to start
    sleep 3
    
    # Test if server is running
    if ! curl -s http://localhost:8888 > /dev/null 2>&1; then
        echo -e "${RED}❌ Failed to start web server${NC}"
        kill $WEB_SERVER_PID 2>/dev/null || true
        exit 1
    fi
    
    echo -e "${GREEN}✅ Web server started (PID: $WEB_SERVER_PID)${NC}"
}

# Function to stop web server
stop_web_server() {
    if [ ! -z "$WEB_SERVER_PID" ]; then
        echo -e "${YELLOW}🛑 Stopping web server...${NC}"
        kill $WEB_SERVER_PID 2>/dev/null || true
        wait $WEB_SERVER_PID 2>/dev/null || true
    fi
}

# Function to install test dependencies
install_dependencies() {
    echo -e "${YELLOW}📦 Installing test dependencies...${NC}"
    
    if [ ! -f "package.json" ]; then
        echo -e "${RED}❌ package.json not found in test directory${NC}"
        exit 1
    fi
    
    npm install --no-fund --no-audit > "$LOG_DIR/npm-install.log" 2>&1
    
    if [ $? -ne 0 ]; then
        echo -e "${RED}❌ Failed to install dependencies${NC}"
        cat "$LOG_DIR/npm-install.log"
        exit 1
    fi
    
    # Install UI automation dependencies
    if [ -d "ui-automation" ]; then
        cd ui-automation
        npm install --no-fund --no-audit > "../$LOG_DIR/ui-npm-install.log" 2>&1
        npx playwright install > "../$LOG_DIR/playwright-install.log" 2>&1
        cd ..
    fi
    
    echo -e "${GREEN}✅ Dependencies installed${NC}"
}

# Function to run Node.js unit tests
run_node_tests() {
    echo -e "${YELLOW}🟢 Running Node.js unit tests...${NC}"
    
    local start_time=$(date +%s)
    
    # Run Jest tests
    timeout $TIMEOUT npm test -- --coverage --ci --reporters=default --reporters=jest-junit \
        > "$LOG_DIR/node-tests.log" 2>&1
    
    local exit_code=$?
    local end_time=$(date +%s)
    local duration=$((end_time - start_time))
    
    if [ $exit_code -eq 0 ]; then
        echo -e "${GREEN}✅ Node.js tests passed (${duration}s)${NC}"
        return 0
    else
        echo -e "${RED}❌ Node.js tests failed (${duration}s)${NC}"
        echo "Last 20 lines of output:"
        tail -n 20 "$LOG_DIR/node-tests.log"
        return 1
    fi
}

# Function to run Playwright UI tests
run_ui_tests() {
    echo -e "${YELLOW}🌐 Running UI automation tests...${NC}"
    
    if [ ! -d "ui-automation" ]; then
        echo -e "${YELLOW}⚠️  UI automation tests not found, skipping${NC}"
        return 0
    fi
    
    cd ui-automation
    
    local start_time=$(date +%s)
    
    # Run Playwright tests
    timeout $TIMEOUT npx playwright test --reporter=html,json,list \
        > "../$LOG_DIR/ui-tests.log" 2>&1
    
    local exit_code=$?
    local end_time=$(date +%s)
    local duration=$((end_time - start_time))
    
    # Copy Playwright report
    if [ -d "playwright-report" ]; then
        cp -r playwright-report "../$LOG_DIR/"
    fi
    
    if [ -f "test-results/results.json" ]; then
        cp test-results/results.json "../$LOG_DIR/ui-test-results.json"
    fi
    
    cd ..
    
    if [ $exit_code -eq 0 ]; then
        echo -e "${GREEN}✅ UI tests passed (${duration}s)${NC}"
        return 0
    else
        echo -e "${RED}❌ UI tests failed (${duration}s)${NC}"
        echo "Last 20 lines of output:"
        tail -n 20 "$LOG_DIR/ui-tests.log"
        return 1
    fi
}

# Function to run example validation tests
run_example_tests() {
    echo -e "${YELLOW}📝 Running example validation tests...${NC}"
    
    local start_time=$(date +%s)
    local failed_examples=()
    
    # Test each example script
    for example in ../examples/*.mjs; do
        if [ -f "$example" ]; then
            local example_name=$(basename "$example" .mjs)
            echo "  Testing $example_name..."
            
            timeout 30 node "$example" --network=testnet --quick-test \
                > "$LOG_DIR/example-${example_name}.log" 2>&1
            
            if [ $? -eq 0 ]; then
                echo -e "  ${GREEN}✅ $example_name${NC}"
            else
                echo -e "  ${RED}❌ $example_name${NC}"
                failed_examples+=("$example_name")
            fi
        fi
    done
    
    local end_time=$(date +%s)
    local duration=$((end_time - start_time))
    
    if [ ${#failed_examples[@]} -eq 0 ]; then
        echo -e "${GREEN}✅ All examples validated (${duration}s)${NC}"
        return 0
    else
        echo -e "${RED}❌ ${#failed_examples[@]} examples failed (${duration}s)${NC}"
        printf '  %s\n' "${failed_examples[@]}"
        return 1
    fi
}

# Function to run performance tests
run_performance_tests() {
    echo -e "${YELLOW}⚡ Running performance tests...${NC}"
    
    local start_time=$(date +%s)
    
    # Run performance-focused tests
    timeout $TIMEOUT npm test -- --testPathPattern=performance --verbose \
        > "$LOG_DIR/performance-tests.log" 2>&1
    
    local exit_code=$?
    local end_time=$(date +%s)
    local duration=$((end_time - start_time))
    
    if [ $exit_code -eq 0 ]; then
        echo -e "${GREEN}✅ Performance tests passed (${duration}s)${NC}"
        return 0
    else
        echo -e "${RED}❌ Performance tests failed (${duration}s)${NC}"
        return 1
    fi
}

# Function to generate comprehensive report
generate_report() {
    echo -e "${YELLOW}📊 Generating test report...${NC}"
    
    local report_file="$LOG_DIR/test-report.html"
    
    cat > "$report_file" << EOF
<!DOCTYPE html>
<html>
<head>
    <title>WASM SDK Test Report - $TIMESTAMP</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        .header { background: #f0f0f0; padding: 20px; border-radius: 5px; }
        .section { margin: 20px 0; }
        .pass { color: green; }
        .fail { color: red; }
        .warn { color: orange; }
        .code { background: #f5f5f5; padding: 10px; border-radius: 3px; }
        table { border-collapse: collapse; width: 100%; }
        th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }
        th { background-color: #f2f2f2; }
    </style>
</head>
<body>
    <div class="header">
        <h1>🧪 WASM SDK Test Report</h1>
        <p><strong>Generated:</strong> $(date)</p>
        <p><strong>Test Run ID:</strong> $TIMESTAMP</p>
    </div>

    <div class="section">
        <h2>📋 Test Summary</h2>
        <table>
            <tr><th>Test Suite</th><th>Status</th><th>Duration</th><th>Details</th></tr>
EOF

    # Add test results to report
    local overall_status="PASS"
    
    if [ -f "$LOG_DIR/node-tests.log" ]; then
        local node_status="PASS"
        if grep -q "FAIL" "$LOG_DIR/node-tests.log" 2>/dev/null; then
            node_status="FAIL"
            overall_status="FAIL"
        fi
        echo "            <tr><td>Node.js Unit Tests</td><td class=\"$(echo $node_status | tr '[:upper:]' '[:lower:])\">${node_status}</td><td>-</td><td><a href=\"node-tests.log\">Log</a></td></tr>" >> "$report_file"
    fi
    
    if [ -f "$LOG_DIR/ui-tests.log" ]; then
        local ui_status="PASS"
        if grep -q "failed" "$LOG_DIR/ui-tests.log" 2>/dev/null; then
            ui_status="FAIL"
            overall_status="FAIL"
        fi
        echo "            <tr><td>UI Automation Tests</td><td class=\"$(echo $ui_status | tr '[:upper:]' '[:lower:])\">${ui_status}</td><td>-</td><td><a href=\"ui-tests.log\">Log</a> | <a href=\"playwright-report/index.html\">Report</a></td></tr>" >> "$report_file"
    fi
    
    cat >> "$report_file" << EOF
        </table>
    </div>

    <div class="section">
        <h2>📁 Available Reports</h2>
        <ul>
            <li><a href="coverage/lcov-report/index.html">Code Coverage Report</a></li>
            <li><a href="playwright-report/index.html">Playwright Test Report</a></li>
            <li><a href="node-tests.log">Node.js Test Output</a></li>
            <li><a href="ui-tests.log">UI Test Output</a></li>
        </ul>
    </div>

    <div class="section">
        <h2>🔧 Environment Information</h2>
        <div class="code">
            <strong>Node.js:</strong> $(node --version)<br>
            <strong>Platform:</strong> $(uname -s)<br>
            <strong>Architecture:</strong> $(uname -m)<br>
            <strong>Test Directory:</strong> $(pwd)<br>
            <strong>WASM SDK:</strong> $(ls -la ../pkg/wasm_sdk.js 2>/dev/null | awk '{print $5" bytes, "$6" "$7" "$8}')
        </div>
    </div>

    <div class="section">
        <h2>🎯 Overall Status: <span class="$(echo $overall_status | tr '[:upper:]' '[:lower:]')">${overall_status}</span></h2>
    </div>

</body>
</html>
EOF

    echo -e "${GREEN}✅ Test report generated: $report_file${NC}"
}

# Function to cleanup
cleanup() {
    echo -e "${YELLOW}🧹 Cleaning up...${NC}"
    stop_web_server
    
    # Archive old test results
    if [ -d "test-results-archive" ]; then
        find test-results-archive -name "test-results-*" -mtime +7 -exec rm -rf {} \; 2>/dev/null || true
    fi
    
    mkdir -p test-results-archive
    if [ -d "$LOG_DIR" ]; then
        mv "$LOG_DIR" test-results-archive/
        echo -e "${GREEN}✅ Test results archived to test-results-archive/$LOG_DIR${NC}"
    fi
}

# Main execution
main() {
    local failed_suites=()
    local total_start_time=$(date +%s)
    
    # Setup trap for cleanup
    trap cleanup EXIT
    
    echo "Starting comprehensive test suite at $(date)"
    echo ""
    
    # Run test phases
    check_prerequisites
    install_dependencies
    start_web_server
    
    echo ""
    echo -e "${BLUE}🚀 Running Test Suites${NC}"
    echo -e "${BLUE}====================${NC}"
    
    # Phase 1: Node.js unit tests
    if ! run_node_tests; then
        failed_suites+=("Node.js Unit Tests")
    fi
    
    echo ""
    
    # Phase 2: UI automation tests
    if ! run_ui_tests; then
        failed_suites+=("UI Automation Tests")
    fi
    
    echo ""
    
    # Phase 3: Example validation
    if ! run_example_tests; then
        failed_suites+=("Example Validation")
    fi
    
    echo ""
    
    # Phase 4: Performance tests
    if ! run_performance_tests; then
        failed_suites+=("Performance Tests")
    fi
    
    echo ""
    
    # Generate report
    generate_report
    
    local total_end_time=$(date +%s)
    local total_duration=$((total_end_time - total_start_time))
    
    echo ""
    echo -e "${BLUE}📊 Test Results Summary${NC}"
    echo -e "${BLUE}======================${NC}"
    echo "Total Duration: ${total_duration}s"
    echo "Results Directory: test-results-archive/$LOG_DIR"
    
    if [ ${#failed_suites[@]} -eq 0 ]; then
        echo -e "${GREEN}🎉 All test suites passed!${NC}"
        exit 0
    else
        echo -e "${RED}❌ Failed test suites:${NC}"
        printf "${RED}  - %s${NC}\n" "${failed_suites[@]}"
        echo ""
        echo -e "${YELLOW}💡 Check individual logs in test-results-archive/$LOG_DIR/${NC}"
        exit 1
    fi
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --timeout)
            TIMEOUT="$2"
            shift 2
            ;;
        --parallel)
            PARALLEL_JOBS="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 [options]"
            echo ""
            echo "Options:"
            echo "  --timeout SECONDS    Set test timeout (default: 300)"
            echo "  --parallel JOBS      Set parallel job count (default: 4)"
            echo "  --help              Show this help message"
            echo ""
            echo "Example:"
            echo "  $0 --timeout 600 --parallel 8"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Run main function
main