# WASM SDK UI Automation Tests

Automated testing suite for the WASM SDK web interface (`index.html`) using Playwright.

## Features

- **Cross-browser testing** (currently configured for Chromium, easily extensible)
- **Automated parameter injection** from existing test data
- **Page Object Model** for maintainable test code
- **Parameterized testing** with multiple data sets
- **Network switching** (testnet/mainnet) testing
- **Error handling** and validation testing
- **Comprehensive reporting** with screenshots and videos on failure

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

# From WASM SDK root directory:
yarn test:ui                # Run all UI tests
yarn test:ui:smoke          # Run smoke tests only
yarn test:ui:headed         # Run with visible browser
yarn test:ui:debug          # Run in debug mode
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
   - Identity queries (getIdentity, getIdentityBalance, getIdentityKeys)
   - Data contract queries (getDataContract, getDataContracts)
   - Document queries (getDocuments, getDocument)
   - System queries (getStatus, getCurrentEpoch, getTotalCreditsInPlatform)
   - Error handling scenarios

3. **Parameterized Tests** (`parameterized-queries.spec.js`)
   - Multiple parameter sets per query
   - Cross-network testing
   - Parameter validation
   - Random parameter stress testing

### Architecture

```
ui-automation/
├── tests/                  # Test specification files
├── utils/                  # Test utilities and page objects
│   ├── base-test.js       # Base test functionality
│   ├── wasm-sdk-page.js   # Page Object Model for index.html
│   └── parameter-injector.js # Parameter injection system
├── fixtures/              # Test data and fixtures
│   └── test-data.js       # Centralized test parameters
└── playwright.config.js   # Playwright configuration
```

## Configuration

### Playwright Configuration

The `playwright.config.js` file is configured for:
- **Base URL**: `http://localhost:8888` (auto-started Python server)
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

**Integration with existing yarn workspace:**
```bash
# From platform root directory:
yarn workspace @dashevo/wasm-sdk test:ui:smoke
# Note: Currently WASM SDK is not configured as a yarn workspace
# Use direct script execution instead
```

## Troubleshooting

### Common Issues

**The test runner script (`run-ui-tests.sh`) automatically handles most setup issues, but here are manual solutions if needed:**

1. **"Host system is missing dependencies"**
   ```bash
   sudo npx playwright install-deps
   # or specifically:
   sudo apt-get install libavif13
   ```

2. **Connection refused on localhost:8888**
   - The script automatically starts the Python server
   - If manual setup needed: `python3 -m http.server 8888` from WASM SDK directory
   - Check if port 8888 is available: `lsof -i :8888`

3. **WASM module fails to load**
   - The script automatically builds WASM SDK if missing
   - Manual build: `cd ../../ && ./build.sh`
   - Check browser console for errors in debug mode

4. **Tests timeout or initialization issues**
   - SDK initialization is now more robust with retry logic
   - Use `DEBUG=true ./run-ui-tests.sh` to see detailed timing information
   - Check network connectivity to Dash Platform endpoints

5. **SDK initialization stuck in loading state**
   - Fixed in current version with enhanced waiting logic
   - The test now waits properly for SDK to reach success state
   - Network switching properly waits for re-initialization

### Debug Mode

The test runner script provides comprehensive debugging options:

```bash
# Enable debug output to see detailed execution information
DEBUG=true ./run-ui-tests.sh smoke

# Run with visible browser to see what's happening
./run-ui-tests.sh headed

# Interactive debugging mode
./run-ui-tests.sh debug

# UI mode for step-by-step debugging
./run-ui-tests.sh ui

# Check script paths and configuration
DEBUG=true ./run-ui-tests.sh --help
```

**Alternative npm commands:**
```bash
npm run test:debug    # Debug mode
npm run test:ui       # UI mode for interactive debugging  
npm run test:headed   # Run in headed mode to see browser
```

### Common Issues and Solutions

1. **Script not found or permission errors**
   ```bash
   # Make sure script is executable
   chmod +x run-ui-tests.sh
   
   # Run from correct directory
   cd /path/to/wasm-sdk/test/ui-automation
   ./run-ui-tests.sh smoke
   ```

2. **Path resolution issues**
   ```bash
   # Use DEBUG mode to see resolved paths
   DEBUG=true ./run-ui-tests.sh smoke
   
   # The script should work from any directory
   /full/path/to/run-ui-tests.sh smoke
   ```

## Extending Tests

### Adding New Query Categories

1. Update `test-data.js` with new query parameters
2. Add parameter mapping to `parameter-injector.js`
3. Create test cases in appropriate spec files
4. Update documentation

### Cross-Browser Testing

To enable Firefox and WebKit testing, uncomment the browser configurations in `playwright.config.js`:

```javascript
projects: [
  { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
  { name: 'firefox', use: { ...devices['Desktop Firefox'] } },
  { name: 'webkit', use: { ...devices['Desktop Safari'] } },
]
```

### Performance Testing

Add performance assertions:
```javascript
test('should execute query within time limit', async ({ page }) => {
  const start = Date.now();
  // ... execute query
  const duration = Date.now() - start;
  expect(duration).toBeLessThan(10000); // 10 second limit
});
```

## Recent Improvements

### v1.2 - Test Validation and Accuracy Improvements

- **✅ Fixed False Positive Test Results**: Tests now properly distinguish between query execution success and actual data retrieval success
- **✅ Enhanced Result Validation**: Tests verify that queries return valid data, not error messages like "Identity not found"
- **✅ Proper Error Detection**: Tests correctly fail when queries return error messages instead of expected data
- **✅ Improved Assertions**: Changed from `expect(result.success || result.hasError).toBe(true)` to proper success validation

### v1.1 - Stability and Robustness Improvements

- **✅ Fixed SDK Initialization Issues**: Enhanced timing logic with retry mechanism for reliable SDK initialization
- **✅ Improved Network Switching**: Proper waiting for re-initialization after network changes
- **✅ Enhanced Error Handling**: Graceful handling of unavailable UI elements (e.g., proof toggles)
- **✅ Better Path Resolution**: Test runner script works from any directory with robust path validation
- **✅ Comprehensive Debug Mode**: `DEBUG=true` provides detailed execution information for troubleshooting
- **✅ Automated Setup**: Script automatically handles Python server, WASM building, and dependency installation
- **✅ Improved Test Stability**: Eliminated race conditions and timing issues in test execution

### Integration with Existing Test Infrastructure

The UI automation tests complement the existing Node.js-based tests:

- **Unit tests**: Test individual WASM functions
- **Integration tests**: Test SDK API calls directly  
- **UI tests**: Test complete user workflows through the web interface

All test types use the same parameter data from `test-data.js` for consistency.

## Support

For issues or questions:

1. Use `DEBUG=true ./run-ui-tests.sh` to get detailed execution information
2. Check the HTML reports in `playwright-report/` for visual debugging
3. Review the implementation summary in `IMPLEMENTATION_SUMMARY.md`
4. Examine test screenshots and videos in `test-results/` for failed tests