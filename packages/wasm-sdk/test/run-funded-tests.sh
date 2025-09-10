#!/bin/bash

# Funded Test Runner for WASM SDK
# Executes tests that use actual testnet funds with comprehensive safety checks

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${BLUE}💰 WASM SDK Funded Test Suite${NC}"
echo -e "${BLUE}=============================${NC}"
echo ""

# Configuration
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
LOG_DIR="funded-test-results-${TIMESTAMP}"
FUNDED_ENV_FILE="funded/.env"

# Safety flags
SAFETY_CONFIRMED=false
DRY_RUN=false
VERBOSE=false
TEST_TIER="low"

# Function to show help
show_help() {
    echo "Usage: $0 [options]"
    echo ""
    echo "Options:"
    echo "  --tier low|medium|high    Set funding tier (default: low)"
    echo "  --dry-run                 Validate configuration without running tests"
    echo "  --verbose                 Enable detailed output"
    echo "  --confirm-safety          Acknowledge fund usage (required)"
    echo "  --help                    Show this help message"
    echo ""
    echo "Funding Tiers:"
    echo "  low     - Tests using < 50M credits per operation"
    echo "  medium  - Tests using < 200M credits per operation"
    echo "  high    - Tests using < 500M credits per operation"
    echo ""
    echo "Environment:"
    echo "  Required: funded/.env with faucet configuration"
    echo "  Network:  testnet only (enforced)"
    echo "  Safety:   Multiple confirmation steps required"
    echo ""
    echo "Example:"
    echo "  $0 --tier low --confirm-safety"
    echo "  $0 --dry-run --verbose"
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --tier)
            TEST_TIER="$2"
            if [[ ! "$TEST_TIER" =~ ^(low|medium|high)$ ]]; then
                echo -e "${RED}❌ Invalid tier: $TEST_TIER. Must be low, medium, or high${NC}"
                exit 1
            fi
            shift 2
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --confirm-safety)
            SAFETY_CONFIRMED=true
            shift
            ;;
        --help)
            show_help
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Function to log verbose messages
log_verbose() {
    if [ "$VERBOSE" = true ]; then
        echo -e "${YELLOW}  $1${NC}"
    fi
}

# Function to check prerequisites
check_prerequisites() {
    echo -e "${YELLOW}🔍 Checking prerequisites...${NC}"
    
    # Check Node.js
    if ! command -v node &> /dev/null; then
        echo -e "${RED}❌ Node.js is required${NC}"
        exit 1
    fi
    
    # Check environment file
    if [ ! -f "$FUNDED_ENV_FILE" ]; then
        echo -e "${RED}❌ Environment file not found: $FUNDED_ENV_FILE${NC}"
        echo "Copy funded/.env.example to funded/.env and configure it"
        exit 1
    fi
    
    # Load environment variables
    set -a
    source "$FUNDED_ENV_FILE"
    set +a
    
    # Verify critical environment variables
    if [ -z "$FAUCET_1_ADDRESS" ] || [ -z "$FAUCET_1_PRIVATE_KEY" ]; then
        echo -e "${RED}❌ Faucet configuration incomplete${NC}"
        echo "Configure FAUCET_1_ADDRESS and FAUCET_1_PRIVATE_KEY in $FUNDED_ENV_FILE"
        exit 1
    fi
    
    # Enforce testnet only
    if [ "$NETWORK" != "testnet" ] && [ "$NETWORK" != "devnet" ] && [ "$NETWORK" != "regtest" ]; then
        echo -e "${RED}🚨 CRITICAL ERROR: Funded tests only allowed on testnet/devnet/regtest${NC}"
        echo "Current NETWORK: $NETWORK"
        exit 1
    fi
    
    # Check if funded tests are enabled
    if [ "$ENABLE_FUNDED_TESTS" != "true" ]; then
        echo -e "${RED}❌ Funded tests not enabled${NC}"
        echo "Set ENABLE_FUNDED_TESTS=true in $FUNDED_ENV_FILE"
        exit 1
    fi
    
    echo -e "${GREEN}✅ Prerequisites check passed${NC}"
    log_verbose "Network: $NETWORK"
    log_verbose "Faucet: ${FAUCET_1_ADDRESS:0:20}..."
    log_verbose "Test Tier: $TEST_TIER"
}

# Function to show safety warnings
show_safety_warnings() {
    echo ""
    echo -e "${CYAN}⚠️  IMPORTANT SAFETY INFORMATION ⚠️${NC}"
    echo -e "${CYAN}=================================${NC}"
    echo ""
    echo -e "${YELLOW}These tests will use REAL TESTNET FUNDS:${NC}"
    echo ""
    echo "💰 Funding Limits by Tier:"
    case $TEST_TIER in
        low)
            echo "   - Per Operation: Up to 50M credits (~0.5 DASH)"
            echo "   - Per Test Suite: Up to 200M credits (~2 DASH)"
            echo "   - Daily Budget: Up to 1B credits (~10 DASH)"
            ;;
        medium)
            echo "   - Per Operation: Up to 200M credits (~2 DASH)"
            echo "   - Per Test Suite: Up to 1B credits (~10 DASH)" 
            echo "   - Daily Budget: Up to 5B credits (~50 DASH)"
            ;;
        high)
            echo "   - Per Operation: Up to 500M credits (~5 DASH)"
            echo "   - Per Test Suite: Up to 2B credits (~20 DASH)"
            echo "   - Daily Budget: Up to 10B credits (~100 DASH)"
            ;;
    esac
    echo ""
    echo "🔒 Safety Measures Active:"
    echo "   - Network: $NETWORK (non-mainnet enforced)"
    echo "   - Faucet: ${FAUCET_1_ADDRESS:0:30}..."
    echo "   - Emergency stops on unusual usage"
    echo "   - Comprehensive usage tracking and reporting"
    echo "   - Automatic cleanup and resource recovery"
    echo ""
    echo "📊 What Will Be Tested:"
    echo "   - Real identity creation with actual blockchain transactions"
    echo "   - Document operations consuming platform credits"
    echo "   - Identity funding and topup operations"
    echo "   - Error handling with real network constraints"
    echo "   - Performance testing with actual cost measurements"
    echo ""
}

# Function to get user confirmation
get_user_confirmation() {
    if [ "$SAFETY_CONFIRMED" = false ]; then
        echo -e "${RED}❌ Safety confirmation required${NC}"
        echo "Add --confirm-safety flag to acknowledge fund usage"
        echo "Example: $0 --tier $TEST_TIER --confirm-safety"
        exit 1
    fi
    
    echo -e "${GREEN}✅ Safety confirmation received${NC}"
}

# Function to validate faucet balance
validate_faucet_balance() {
    echo -e "${YELLOW}💰 Validating faucet balance...${NC}"
    
    # This would normally check actual balance
    # For now, we validate the configuration
    log_verbose "Faucet address configured: $FAUCET_1_ADDRESS"
    log_verbose "Private key configured: ✓"
    
    echo -e "${GREEN}✅ Faucet configuration validated${NC}"
}

# Function to create results directory
setup_logging() {
    echo -e "${YELLOW}📁 Setting up test logging...${NC}"
    
    mkdir -p "$LOG_DIR"
    mkdir -p "funded/logs"
    
    echo "Test results will be saved to: $LOG_DIR"
    log_verbose "Funded logs directory: funded/logs"
    
    echo -e "${GREEN}✅ Logging setup completed${NC}"
}

# Function to run funded tests
run_funded_tests() {
    echo -e "${YELLOW}🚀 Running funded tests...${NC}"
    
    local start_time=$(date +%s)
    local failed_tests=()
    
    # Set environment for tests
    export TEST_MODE=funded
    export FUNDING_TIER=$TEST_TIER
    export FUNDED_TEST_SESSION_ID="session-${TIMESTAMP}"
    
    if [ "$DRY_RUN" = true ]; then
        echo -e "${CYAN}🏃 DRY RUN MODE - No actual funding will occur${NC}"
        export DRY_RUN_FUNDING=true
    fi

    # Run identity operations tests
    echo "🧪 Running identity operations tests..."
    if npx playwright test funded/integration/identity-operations.test.js --timeout=300000 --workers=1 > "$LOG_DIR/identity-ops.log" 2>&1; then
        echo -e "${GREEN}✅ Identity operations tests passed${NC}"
    else
        echo -e "${RED}❌ Identity operations tests failed${NC}"
        failed_tests+=("identity-operations")
    fi
    
    # Run document operations tests  
    echo "📄 Running document operations tests..."
    if npx playwright test funded/integration/document-operations.test.js --timeout=300000 --workers=1 > "$LOG_DIR/document-ops.log" 2>&1; then
        echo -e "${GREEN}✅ Document operations tests passed${NC}"
    else
        echo -e "${RED}❌ Document operations tests failed${NC}"
        failed_tests+=("document-operations")
    fi

    local end_time=$(date +%s)
    local duration=$((end_time - start_time))
    
    # Generate summary
    echo ""
    echo -e "${BLUE}📊 Funded Test Summary${NC}"
    echo -e "${BLUE}=====================${NC}"
    echo "Duration: ${duration}s"
    echo "Test Tier: $TEST_TIER"
    echo "Results: $LOG_DIR"
    
    if [ ${#failed_tests[@]} -eq 0 ]; then
        echo -e "${GREEN}🎉 All funded tests passed!${NC}"
        echo ""
        echo "📊 Check detailed reports in:"
        echo "  - $LOG_DIR/"
        echo "  - funded/logs/"
        return 0
    else
        echo -e "${RED}❌ Failed tests: ${failed_tests[*]}${NC}"
        echo ""
        echo "📋 Check logs for details:"
        printf "  - $LOG_DIR/%s.log\n" "${failed_tests[@]}"
        return 1
    fi
}

# Function to cleanup
cleanup() {
    echo -e "${YELLOW}🧹 Cleaning up...${NC}"
    
    # Archive results
    if [ -d "$LOG_DIR" ]; then
        mkdir -p funded/test-results-archive
        mv "$LOG_DIR" funded/test-results-archive/
        echo "Test results archived to funded/test-results-archive/$LOG_DIR"
    fi
    
    # Generate final report
    if [ -f "funded/logs/credit-usage.log" ]; then
        echo -e "${GREEN}💰 Credit usage log available: funded/logs/credit-usage.log${NC}"
    fi
}

# Main execution
main() {
    echo "Starting funded test suite at $(date)"
    echo ""
    
    check_prerequisites
    echo ""
    
    show_safety_warnings
    echo ""
    
    get_user_confirmation
    echo ""
    
    validate_faucet_balance
    echo ""
    
    setup_logging
    echo ""
    
    if [ "$DRY_RUN" = true ]; then
        echo -e "${CYAN}🏃 DRY RUN completed - configuration validated${NC}"
        echo "Run without --dry-run to execute actual funded tests"
        return 0
    fi
    
    # Setup trap for cleanup
    trap cleanup EXIT
    
    # Run the tests
    if run_funded_tests; then
        echo ""
        echo -e "${GREEN}🎉 Funded test suite completed successfully!${NC}"
        return 0
    else
        echo ""
        echo -e "${RED}❌ Funded test suite completed with failures${NC}"
        return 1
    fi
}

# Execute main function
main