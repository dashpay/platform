# WASM SDK UI Automation Tests

Automated testing suite for the WASM SDK web interface (`index.html`) using Playwright.

## Features

- **Cross-browser testing** (currently configured for Chromium, easily extensible)
- **Automated parameter injection** from existing test data
- **Page Object Model** for maintainable test code
- **Network switching** (testnet/mainnet) testing
- **Error handling** and validation testing
- **Comprehensive reporting** with screenshots and videos on failure
- **GitHub Actions integration** for automated testing

## Quick Start

### Prerequisites

- Node.js 18+ installed
- Python 3 for serving the web interface
- Linux environment (Ubuntu/Debian recommended)

### Installation

```bash
cd /path/to/wasm-sdk/test/ui-automation
npm install
npx playwright install chromium
```

### Running Tests

The easiest way to run tests is using the provided shell script:

```bash
# From any directory, using the test runner script:
./run-ui-tests.sh smoke          # Basic functionality tests
./run-ui-tests.sh queries        # Query execution tests  
./run-ui-tests.sh parameterized  # Comprehensive parameter testing
./run-ui-tests.sh all           # Run all tests (default)

# Run in headed mode (see browser)
./run-ui-tests.sh headed

# Debug mode with detailed output
DEBUG=true ./run-ui-tests.sh smoke

# Pattern matching for specific tests
./run-ui-tests.sh --grep="should initialize SDK"
```

**Alternative: Direct npm commands** (from ui-automation directory):

```bash
npm test                    # Run all tests
npm run test:smoke          # Basic functionality tests
npm run test:queries        # Query execution tests
npm run test:parameterized  # Comprehensive parameter testing
npm run test:headed         # Run in headed mode
npm run test:debug          # Debug mode
npm run test:report         # View HTML report
```

## Test Structure

### Test Categories

1. **Basic Smoke Tests** (`basic-smoke.spec.js`)
   - SDK initialization
   - UI component visibility
   - Network switching
   - Basic interaction flows

2. **Query Execution Tests** (`query-execution.spec.js`)
   - Identity queries (getIdentity, getIdentityBalance, getIdentityKeys, etc.)
   - Data contract queries (getDataContract, getDataContracts, getDataContractHistory)
   - Document queries (getDocuments, getDocument)
   - System queries (getStatus, getCurrentEpoch, getTotalCreditsInPlatform)
   - Error handling scenarios
   - Proof support testing with automatic fallback

3. **Parameterized Tests** (`parameterized-queries.spec.js`)
   - Multiple parameter sets per query type
   - Cross-network testing scenarios
   - Parameter validation testing

### Architecture

```text
ui-automation/
├── tests/                  # Test specification files
│   ├── basic-smoke.spec.js        # Basic functionality tests
│   ├── query-execution.spec.js    # Comprehensive query testing
│   └── parameterized-queries.spec.js # Multi-parameter testing
├── utils/                  # Test utilities and page objects
│   ├── base-test.js       # Base test functionality
│   ├── wasm-sdk-page.js   # Page Object Model for index.html
│   └── parameter-injector.js # Parameter injection system
├── fixtures/              # Test data and fixtures
│   └── test-data.js       # Centralized test parameters
├── playwright.config.js   # Playwright configuration
├── run-ui-tests.sh        # Comprehensive test runner script
└── package.json           # npm scripts and dependencies
```

## Configuration

### Playwright Configuration

The `playwright.config.js` file is configured for:

- **Base URL**: `http://localhost:8888` (server managed via Playwright config's `webServer`)
- **Browsers**: Chromium (headless by default)
- **Timeouts**: 30s for actions, 120s for tests
- **Reporters**: HTML, JSON, and console output
- **Screenshots/Videos**: On failure only

### Test Data

Test parameters are centralized in `fixtures/test-data.js` and include:

- Known testnet identity IDs
- Data contract IDs (DPNS, DashPay, etc.)
- Document IDs and examples
- Token IDs for testing
- Parameter sets for each query type

## Usage Examples

### Running Specific Tests

```bash
# Run only identity query tests
npx playwright test --grep "Identity Queries"

# Run tests for a specific query
npx playwright test --grep "getIdentity"

# Run tests on headed browser for debugging
npx playwright test --headed --grep "smoke"
```

### Adding New Tests

1. **Add test data** to `fixtures/test-data.js`
2. **Create test file** in `tests/` directory
3. **Use page object** for UI interactions
4. **Use parameter injector** for form filling

Example:

```javascript
const { test, expect } = require('@playwright/test');
const { WasmSdkPage } = require('../utils/wasm-sdk-page');
const { ParameterInjector } = require('../utils/parameter-injector');

test('should execute my new query', async ({ page }) => {
  const wasmSdkPage = new WasmSdkPage(page);
  const parameterInjector = new ParameterInjector(wasmSdkPage);
  
  await wasmSdkPage.initialize('testnet');
  await wasmSdkPage.setupQuery('myCategory', 'myQueryType');
  
  const success = await parameterInjector.injectParameters('myCategory', 'myQueryType');
  expect(success).toBe(true);
  
  const result = await wasmSdkPage.executeQueryAndGetResult();
  expect(result.success || result.hasError).toBe(true);
});
```

## CI/CD Integration

### GitHub Actions Integration

Tests run automatically in CI or can be triggered manually with different configurations:

- Automatic execution after WASM SDK builds
- Manual execution with configurable test types and browsers
- Comprehensive reporting with HTML reports and test artifacts

### Local CI Testing

For continuous integration, use the test runner script which handles all setup automatically:

```bash
# In CI environment - the script handles all prerequisites
./run-ui-tests.sh smoke     # Quick smoke tests for PR validation
./run-ui-tests.sh all       # Full test suite for releases

# CI-friendly JSON output
DEBUG=false ./run-ui-tests.sh all

# Results available in:
# - playwright-report/ (HTML)
# - test-results.json (JSON)
# - test-results/ (screenshots, videos)
```

**Manual CI setup** (if not using the script):

```bash
# Install system dependencies
sudo npx playwright install-deps
npx playwright install chromium

# Run tests with CI-friendly output  
npm run test:ci

# Results will be in test-results/ directory
```

## Troubleshooting

### Common Issues

1. **Dependencies missing**: Run `sudo npx playwright install-deps`
2. **Port 8888 in use**: Playwright config's `webServer` starts the server automatically; ensure the port is free or update the config
3. **WASM build issues**: The script rebuilds WASM if needed
4. **Test timeouts**: Use `DEBUG=true ./run-ui-tests.sh` for details

### Debug Mode

```bash
# See detailed execution logs
DEBUG=true ./run-ui-tests.sh smoke

# Run with visible browser
./run-ui-tests.sh headed

# Interactive debugging
./run-ui-tests.sh debug
```

## Extending Tests

To add new tests:

1. Add test data to `fixtures/test-data.js`
2. Create test cases using the page object model
3. Use `parameter-injector.js` for form filling

## Known Issues

- Some queries don't yet support proof information in the WASM SDK
- Tests automatically skip proof testing for unsupported queries
- All core functionality works correctly

## Support

For issues or questions:

1. Use `DEBUG=true ./run-ui-tests.sh` to get detailed execution information
2. Check the HTML reports in `playwright-report/` for visual debugging
3. Review the implementation summary in `IMPLEMENTATION_SUMMARY.md`
4. Examine test screenshots and videos in `test-results/` for failed tests
5. Check GitHub Actions workflow runs for CI/CD issues
