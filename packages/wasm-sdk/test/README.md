# WASM SDK Test Suite

Comprehensive unit tests for the Dash Platform WASM SDK.

## Test Structure

```
test/
├── README.md                    # This file
├── test-plan.md                # Comprehensive test plan
├── test-summary.md             # Current test results summary
├── EXPECTED_FAILURES.md        # Documentation of expected failures
├── run-all-tests.mjs          # Main test runner
├── test-report.html           # Generated HTML report
│
├── sdk-init-simple.test.mjs   # SDK initialization tests
├── key-generation.test.mjs    # Key generation and derivation tests
├── dpns.test.mjs             # DPNS functionality tests
├── utilities-simple.test.mjs  # Utility functions tests
├── sample-query.test.mjs     # Sample query tests (network-dependent)
│
└── run-tests.mjs             # Key derivation test runner (legacy)
```

## Quick Start

### Run All Tests
```bash
node test/run-all-tests.mjs
```

### Run Individual Test Suites
```bash
# SDK initialization
node test/sdk-init-simple.test.mjs

# Key generation (BIP39/BIP32/BIP44)
node test/key-generation.test.mjs

# DPNS functions
node test/dpns.test.mjs

# Utility functions
node test/utilities-simple.test.mjs

# Sample network queries
node test/sample-query.test.mjs
```

## Test Categories

### ✅ Standalone Tests (No Network Required)
- **SDK Initialization**: Builder patterns, version checking
- **Key Generation**: Mnemonic generation, key derivation, address generation
- **DPNS Validation**: Username validation, homograph safety
- **Utility Functions**: Error handling, type validation

### 🌐 Network-Dependent Tests
- **Query Functions**: Identity, document, contract, token queries
- **State Transitions**: Token operations, document operations
- **System Queries**: Platform status, epoch information

## Current Status

| Test Suite | Total | Pass | Fail | Notes |
|------------|-------|------|------|-------|
| SDK Init | 10 | 9 | 1 | Address validation needs fix |
| Key Gen | 53 | 49 | 4 | Path helpers need fixes |
| DPNS | 34 | 31 | 3 | Homograph handling incomplete |
| Utilities | 14 | 13 | 1 | testSerialization bug |
| **Total** | **111** | **102** | **9** | **91.9% pass rate** |

## Test Reports

After running all tests, view the detailed report:
1. Open `test/test-report.html` in a browser
2. Click on test suites to expand details
3. Failed tests are highlighted in red

## Writing New Tests

### Test Template
```javascript
import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { webcrypto } from 'crypto';

// Set up WASM environment
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

if (!global.crypto) {
    Object.defineProperty(global, 'crypto', {
        value: webcrypto,
        configurable: true
    });
}

// Import and initialize WASM
import init, * as wasmSdk from '../pkg/wasm_sdk.js';

const wasmPath = join(__dirname, '../pkg/wasm_sdk_bg.wasm');
const wasmBuffer = readFileSync(wasmPath);
await init(wasmBuffer);

// Your tests here...
```

### Best Practices
1. Use descriptive test names
2. Handle expected errors gracefully
3. Document network dependencies
4. Clean up resources (call `.free()` on SDK instances)
5. Update `EXPECTED_FAILURES.md` for known issues

## Troubleshooting

### Common Issues

1. **"using deprecated parameters" warning**
   - This is a known issue and can be ignored
   - Use `2>&1 | grep -v "using deprecated parameters"` to filter

2. **Panics in test functions**
   - Some internal test functions cause panics
   - These are documented in `EXPECTED_FAILURES.md`

3. **Network timeouts**
   - Ensure internet connectivity
   - Check if Dash Platform testnet is operational
   - Some queries may timeout without valid data

4. **Module import errors**
   - Ensure Node.js v16+ with ES modules support
   - Run from the correct directory

## Contributing

When adding tests:
1. Follow the existing test structure
2. Add new test files to `run-all-tests.mjs`
3. Document expected failures
4. Update this README

## Future Work

- [ ] Complete query function tests
- [ ] Add state transition tests
- [ ] Implement proof verification tests
- [ ] Add performance benchmarks
- [ ] Create integration test suite

## Resources

- [WASM SDK Documentation](../AI_REFERENCE.md)
- [Dash Platform Documentation](https://docs.dash.org/projects/platform/)
- [Test Plan](test-plan.md)
- [Expected Failures](EXPECTED_FAILURES.md)