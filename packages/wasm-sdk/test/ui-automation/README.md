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

```bash
# Run all tests
npm test

# Run specific test suites
npm run test:smoke          # Basic functionality tests
npm run test:queries        # Query execution tests
npm run test:parameterized  # Comprehensive parameter testing

# Run in headed mode (see browser)
npm run test:headed

# Debug mode
npm run test:debug

# Generate and view reports
npm run test:report
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

For continuous integration:

```bash
# Install browsers in CI
npx playwright install-deps
npx playwright install chromium

# Run tests with CI-friendly output
npm run test:ci

# Results will be in test-results/ directory
```

## Troubleshooting

### Common Issues

1. **"Host system is missing dependencies"**
   ```bash
   sudo npx playwright install-deps
   ```

2. **Connection refused on localhost:8888**
   - Ensure Python HTTP server is running: `python3 -m http.server 8888`
   - Check if port 8888 is available

3. **WASM module fails to load**
   - Ensure WASM SDK is built: `cd ../../ && ./build.sh`
   - Check browser console for errors

4. **Tests timeout**
   - Increase timeout in `playwright.config.js`
   - Check network connectivity to Dash Platform endpoints

### Debug Mode

```bash
# Run with debug console
npm run test:debug

# Run with UI mode for interactive debugging
npm run test:ui

# Run in headed mode to see browser
npm run test:headed
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

## Integration with Existing Test Infrastructure

The UI automation tests complement the existing Node.js-based tests:

- **Unit tests**: Test individual WASM functions
- **Integration tests**: Test SDK API calls directly  
- **UI tests**: Test complete user workflows through the web interface

All test types use the same parameter data from `test-data.js` for consistency.